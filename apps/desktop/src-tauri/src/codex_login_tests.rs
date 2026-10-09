use super::*;
use std::io::Write;

#[test]
fn managed_login_is_memory_only_without_a_persistent_fallback() {
    let config = include_str!("codex-runtime.toml");
    assert!(config.contains("cli_auth_credentials_store = \"ephemeral\""));
    assert!(!config.contains("cli_auth_credentials_store = \"keyring\""));
    assert!(!config.contains("cli_auth_credentials_store = \"file\""));
    assert!(!config.contains("cli_auth_credentials_store = \"auto\""));
    assert!(config.contains("forced_login_method = \"chatgpt\""));
    assert!(
        require_memory_auth(&json!({"config":{"cli_auth_credentials_store":"ephemeral"}})).is_ok()
    );
    for store in ["keyring", "file", "auto", "unknown"] {
        assert_eq!(
            require_memory_auth(&json!({"config":{"cli_auth_credentials_store":store}})),
            Err("PLAN_AUTH_STORAGE_POLICY")
        );
    }
    assert_eq!(
        require_memory_auth(&json!({"config":{}})),
        Err("PLAN_PROTOCOL_INVALID")
    );
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "explicit offline official-runtime memory-auth qualification; synthetic credential only, no browser, Keychain or provider"]
fn memory_auth_runtime_qualification() {
    let path = PathBuf::from(std::env::var_os("ORT_CODEX_QUALIFY_PATH").unwrap());
    ort_codex_install::verify_payload(&path).unwrap();
    let auth = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let secret = "sk-ort-synthetic-not-a-real-key";
    // Only this disposable test permits API-key login, which stores locally
    // without contacting a provider. It exercises the same auth storage used
    // by managed OAuth after its token exchange. All other production config
    // and OS containment rules apply; no reachable gateway is provided.
    let config = include_str!("codex-runtime.toml").replace(
        "forced_login_method = \"chatgpt\"",
        "forced_login_method = \"api\"",
    );
    std::fs::write(auth.path().join("config.toml"), config).unwrap();
    let start = || {
        let policy = sandbox_policy(&path, auth.path(), scratch.path(), 1).unwrap();
        let child = Command::new("/usr/bin/sandbox-exec")
            .args(["-p", &policy])
            .arg(&path)
            .args(["app-server", "--listen", "stdio://"])
            .env_clear()
            .env("CODEX_HOME", auth.path())
            .env("PATH", "/usr/bin:/bin")
            .env("RUST_LOG", "off")
            .env("TMPDIR", scratch.path())
            .current_dir(scratch.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        Session::from_child(
            child,
            tempfile::tempdir().unwrap(),
            None,
            Instant::now() + Duration::from_secs(30),
            &|| false,
        )
        .unwrap()
    };
    let read = |session: &mut Session| {
        session
            .rpc(
                "account/read",
                json!({"refreshToken":false}),
                Duration::from_secs(10),
                &|| false,
            )
            .unwrap()
    };
    let login = |session: &mut Session| {
        session
            .rpc(
                "account/login/start",
                json!({"type":"apiKey", "apiKey":secret}),
                Duration::from_secs(10),
                &|| false,
            )
            .unwrap()
    };
    let mut first = start();
    assert!(read(&mut first)["account"].is_null());
    login(&mut first);
    assert_eq!(
        read(&mut first)
            .pointer("/account/type")
            .and_then(Value::as_str),
        Some("apiKey")
    );
    assert_eq!(
        read(&mut first)
            .pointer("/account/type")
            .and_then(Value::as_str),
        Some("apiKey")
    );
    assert!(!auth.path().join("auth.json").exists());
    first
        .rpc(
            "account/logout",
            json!({}),
            Duration::from_secs(10),
            &|| false,
        )
        .unwrap();
    assert!(read(&mut first)["account"].is_null());
    login(&mut first);
    assert!(!read(&mut first)["account"].is_null());
    drop(first);
    let mut restarted = start();
    assert!(read(&mut restarted)["account"].is_null());
    drop(restarted);
    assert!(!auth.path().join("auth.json").exists());

    // Old files must never be adopted by memory-only mode, even in the same
    // isolated namespace. This is our own synthetic fixture, never user auth.
    std::fs::write(
        auth.path().join("auth.json"),
        serde_json::to_vec(&json!({"OPENAI_API_KEY":secret})).unwrap(),
    )
    .unwrap();
    let mut with_old_file = start();
    assert!(read(&mut with_old_file)["account"].is_null());
    drop(with_old_file);
    std::fs::remove_file(auth.path().join("auth.json")).unwrap();
}

#[test]
fn login_failures_are_classified_without_returning_provider_text() {
    for (error, expected) in [
        (
            "Token exchange failed: error sending request for url (https://auth.openai.com/oauth/token)",
            "PLAN_LOGIN_TRANSPORT_FAILED",
        ),
        (
            "DNS error with private details",
            "PLAN_LOGIN_TRANSPORT_FAILED",
        ),
        (
            "certificate verification failed",
            "PLAN_LOGIN_TRANSPORT_FAILED",
        ),
        ("request timed out", "PLAN_LOGIN_TRANSPORT_FAILED"),
        (
            "Token exchange failed: 400 invalid_grant",
            "PLAN_LOGIN_TOKEN_EXCHANGE_FAILED",
        ),
        ("token_exchange_failed", "PLAN_LOGIN_TOKEN_EXCHANGE_FAILED"),
        (
            "Sign-in completed but credentials could not be saved locally.",
            "PLAN_LOGIN_STORAGE_FAILED",
        ),
        (
            "Codex is not enabled for your workspace",
            "PLAN_LOGIN_RESTRICTED",
        ),
        ("Sign-in was declined", "PLAN_LOGIN_DECLINED"),
        ("Login cancelled", "PLAN_LOGIN_DECLINED"),
        ("access_denied", "PLAN_LOGIN_DECLINED"),
        (
            "Unknown provider failure containing secret=never-retain",
            "PLAN_LOGIN_FAILED",
        ),
    ] {
        assert_eq!(
            login_failure_code(&json!({"params":{"error":error}})),
            expected
        );
    }
    for value in [
        json!({}),
        json!({"params":{"error":null}}),
        json!({"params":{"error":{"secret":"never-retain"}}}),
    ] {
        assert_eq!(login_failure_code(&value), "PLAN_LOGIN_FAILED");
    }
}

#[test]
#[ignore = "explicit OAuth transport qualification with a fake code; no browser, account or inference"]
fn oauth_token_transport_qualification() {
    // Never disturb another application's pending OAuth callback. The released
    // runtime may cancel an existing listener when it tries to reuse this port.
    let guards: Vec<_> = [1455, 1457]
        .into_iter()
        .map(|port| {
            std::net::TcpListener::bind(("127.0.0.1", port))
                .expect("OAuth callback port is busy; close pending sign-in before qualification")
        })
        .collect();
    let path = PathBuf::from(std::env::var_os("ORT_CODEX_QUALIFY_PATH").unwrap());
    ort_codex_install::verify_payload(&path).unwrap();
    let auth = tempfile::tempdir().unwrap();
    let mut session = Session::start_runtime(
        &Runtime { path },
        auth.path(),
        Instant::now() + Duration::from_secs(30),
        &|| false,
    )
    .unwrap();
    drop(guards);
    let login = session
        .rpc(
            "account/login/start",
            json!({"type":"chatgpt"}),
            Duration::from_secs(10),
            &|| false,
        )
        .unwrap();
    session.login_id = Some(login["loginId"].as_str().unwrap().to_owned());
    let authorize = url::Url::parse(login["authUrl"].as_str().unwrap()).unwrap();
    let parameters: std::collections::HashMap<_, _> = authorize.query_pairs().collect();
    let redirect = url::Url::parse(parameters["redirect_uri"].as_ref()).unwrap();
    assert_eq!(redirect.scheme(), "http");
    assert_eq!(redirect.host_str(), Some("127.0.0.1"));
    assert_eq!(redirect.port(), Some(1455));
    let mut callback = redirect;
    callback
        .query_pairs_mut()
        .append_pair("code", "ort-invalid-code-transport-probe")
        .append_pair("state", parameters["state"].as_ref());
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", 1455)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(45)))
        .unwrap();
    write!(
        stream,
        "GET {}?{} HTTP/1.1\r\nHost: localhost:1455\r\nConnection: close\r\n\r\n",
        callback.path(),
        callback.query().unwrap()
    )
    .unwrap();
    let mut response = String::new();
    stream
        .take(64 * 1024)
        .read_to_string(&mut response)
        .unwrap();
    assert!(response.contains("token_exchange_failed"));
    assert!(
        !response.contains("error sending request")
            && !response.contains("error decoding response"),
        "OAuth request did not receive an HTTP response through the contained runtime"
    );
    assert!(
        response.contains("400") || response.contains("401") || response.contains("403"),
        "Expected an HTTP rejection for the deliberately invalid authorization code"
    );
    let account = session
        .rpc(
            "account/read",
            json!({"refreshToken":false}),
            Duration::from_secs(10),
            &|| false,
        )
        .unwrap();
    assert!(account["account"].is_null());
    assert!(!auth.path().join("auth.json").exists());
}
