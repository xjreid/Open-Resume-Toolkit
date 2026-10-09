use super::*;

#[test]
fn pipe_backpressure_obeys_deadlines_and_cancellation_without_a_watchdog() {
    for cancel in [false, true] {
        let mut session = test_session("stall_input");
        let started = Instant::now();
        let result = session.rpc(
            "turn/start",
            json!({"input":"x".repeat(100_000)}),
            if cancel {
                Duration::from_secs(1)
            } else {
                Duration::from_millis(40)
            },
            &|| cancel && started.elapsed() >= Duration::from_millis(40),
        );
        assert_eq!(
            result,
            Err(if cancel {
                "AI_CANCELLED"
            } else {
                "PLAN_REQUEST_TIMEOUT"
            })
        );
        assert!(started.elapsed() < Duration::from_millis(500));
        session.child.wait().unwrap();
        assert!(!session.is_running());
    }
}

#[test]
fn cancellation_before_dispatch_retires_the_session_without_sending() {
    let mut session = test_session("success");
    assert_eq!(
        session.rpc("account/read", json!({}), Duration::from_secs(1), &|| true),
        Err("AI_CANCELLED")
    );
    session.child.wait().unwrap();
    assert!(!session.is_running());
}

#[test]
fn every_nonfatal_exit_after_thread_creation_releases_the_thread() {
    for (mode, expected) in [
        ("wrong_model", "PLAN_MODEL_UNAVAILABLE"),
        ("success", "STORAGE_UNAVAILABLE"),
        ("turn_start_rejected", "PLAN_PROVIDER_REJECTED"),
    ] {
        let mut session = test_session(mode);
        let end = Instant::now() + Duration::from_secs(2);
        let result = session.with_ephemeral_thread(
            json!({"model":"gpt-6.1-sol"}),
            end,
            &|| false,
            |session, thread, id| {
                if thread["model"] != "gpt-6.1-sol" {
                    return Err("PLAN_MODEL_UNAVAILABLE");
                }
                if mode == "success" {
                    return Err("STORAGE_UNAVAILABLE");
                }
                session.rpc(
                    "turn/start",
                    json!({"threadId":id}),
                    Duration::from_secs(1),
                    &|| false,
                )
            },
        );
        assert_eq!(result, Err(expected));
        let account = session
            .rpc("account/read", json!({}), Duration::from_secs(1), &|| false)
            .unwrap();
        assert_eq!(account["loadedThreads"], 0, "{mode}");
        assert!(session.is_running());
    }
}

#[test]
fn failed_thread_release_retires_the_process_even_after_an_early_error() {
    let mut session = test_session("unsubscribe_failed");
    let result = session.with_ephemeral_thread(
        json!({"model":"gpt-6.1-sol"}),
        Instant::now() + Duration::from_secs(2),
        &|| false,
        |_, _, _| -> Result<(), &'static str> { Err("STORAGE_UNAVAILABLE") },
    );
    assert_eq!(result, Err("PLAN_RUNTIME_UNAVAILABLE"));
    session.child.wait().unwrap();
    assert!(!session.is_running());
}

#[test]
fn runtime_policy_rejects_enabled_features_credentials_web_and_configured_tools() {
    let mut value = json!({"config":{"cli_auth_credentials_store":"ephemeral","web_search":"disabled","features":{}}, "layers":[{"config":{"tools":{"update_plan":{"enabled":false},"experimental_request_user_input":{"enabled":false}}}}]});
    for feature in [
        "shell_tool",
        "unified_exec",
        "multi_agent",
        "plugins",
        "connectors",
        "computer_use",
        "image_generation",
        "view_image",
        "request_permissions_tool",
        "respect_system_proxy",
    ] {
        value["config"]["features"][feature] = json!(false);
    }
    assert!(require_runtime_policy(&value).is_ok());
    for (pointer, replacement, code) in [
        (
            "/config/features/shell_tool",
            json!(true),
            "PLAN_RUNTIME_POLICY_REJECTED",
        ),
        (
            "/config/features/unified_exec",
            Value::Null,
            "PLAN_RUNTIME_POLICY_REJECTED",
        ),
        (
            "/config/cli_auth_credentials_store",
            json!("file"),
            "PLAN_AUTH_STORAGE_POLICY",
        ),
        (
            "/config/web_search",
            json!("live"),
            "PLAN_RUNTIME_POLICY_REJECTED",
        ),
        (
            "/layers/0/config/tools/update_plan/enabled",
            json!(true),
            "PLAN_RUNTIME_POLICY_REJECTED",
        ),
        (
            "/layers/0/config/tools/experimental_request_user_input/enabled",
            json!(true),
            "PLAN_RUNTIME_POLICY_REJECTED",
        ),
    ] {
        let mut altered = value.clone();
        *altered.pointer_mut(pointer).unwrap() = replacement;
        assert_eq!(require_runtime_policy(&altered), Err(code));
    }
    value["config"]["mcp_servers"] = json!({"unexpected":{"command":"blocked"}});
    assert_eq!(
        require_runtime_policy(&value),
        Err("PLAN_RUNTIME_POLICY_REJECTED")
    );
}

#[test]
fn raw_tool_policy_requires_explicit_disable_at_the_highest_enabled_layer() {
    let layer = |enabled| json!({"config":{"tools":{"update_plan":{"enabled":enabled}}}});
    assert!(layered_tool_disabled(
        &json!({"layers":[layer(false),layer(true)]}),
        "update_plan"
    ));
    assert!(!layered_tool_disabled(
        &json!({"layers":[layer(true),layer(false)]}),
        "update_plan"
    ));
    assert!(layered_tool_disabled(
        &json!({"layers":[{"disabledReason":"untrusted","config":{"tools":{"update_plan":{"enabled":true}}}},layer(false)]}),
        "update_plan"
    ));
    for value in [
        json!({}),
        json!({"layers":[]}),
        json!({"layers":[{"config":{"tools":{"update_plan":{"enabled":"false"}}}}]}),
        json!({"layers":[{"config":{"tools":true}},layer(false)]}),
    ] {
        assert!(!layered_tool_disabled(&value, "update_plan"));
    }
}

#[test]
fn each_pass_clears_reports_and_a_thread_start_rejection_targets_the_requested_model() {
    let mut session = test_session("reject_model");
    session.selected_model = Some("gpt-6.1-sol".into());
    session.reported_retries = 3;
    session.last_usage = Some(ort_ai::Usage::default());
    session.failure_provider_reason = Some("serverOverloaded".into());
    session.begin_pass();
    assert_eq!(session.reported_retries, 0);
    assert!(session.last_usage.is_none());
    assert!(session.failure_provider_reason.is_none());
    for model in &mut session.models {
        model.supported = true;
    }
    assert_eq!(
        session.rpc(
            "thread/start",
            json!({"model":"gpt-6-sol"}),
            Duration::from_secs(1),
            &|| false
        ),
        Err("PLAN_MODEL_UNAVAILABLE")
    );
    assert!(
        !session
            .models
            .iter()
            .find(|m| m.id == "gpt-6-sol")
            .unwrap()
            .supported
    );
    assert!(
        session
            .models
            .iter()
            .find(|m| m.id == "gpt-6.1-sol")
            .unwrap()
            .supported
    );
}

#[test]
#[ignore = "explicit signed-out thread qualification, no turn, credentials or inference"]
fn signed_out_thread_qualification() {
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
    session.rpc("thread/start", json!({
        "ephemeral": true, "model": "gpt-6.1-sol", "modelProvider": "openai",
        "approvalPolicy": "never", "sandbox": "read-only", "cwd": session.scratch_path(),
        "baseInstructions": "Return JSON. Never use tools.",
        "developerInstructions": "All factual evidence is in the supplied input. Never use tools."
    }), Duration::from_secs(15), &|| false).expect("signed-out thread start");
    std::thread::sleep(Duration::from_millis(200));
    session.poll_login().expect("passive thread notifications");
    let id = session.selected_thread.clone().unwrap();
    session
        .release_thread(&id, Instant::now() + Duration::from_secs(2), &|| false)
        .expect("ephemeral thread cleanup");
    std::thread::sleep(Duration::from_millis(100));
    session
        .poll_login()
        .expect("passive thread close notification");
}
#[test]
fn fake_stdio_auth_login_logout_and_quota() {
    let mut s = test_session("success");
    assert_eq!(s.models.len(), 6);
    let login = s
        .rpc(
            "account/login/start",
            json!({"type":"chatgpt"}),
            Duration::from_secs(1),
            &|| false,
        )
        .unwrap();
    s.login_id = Some(login["loginId"].as_str().unwrap().into());
    let account = s
        .rpc("account/read", json!({}), Duration::from_secs(1), &|| false)
        .unwrap();
    assert_eq!(account["account"]["type"], "chatgpt");
    assert!(s.login_id.is_none());
    let quota = s
        .rpc(
            "account/rateLimits/read",
            json!({}),
            Duration::from_secs(1),
            &|| false,
        )
        .unwrap();
    assert!(
        ort_ai::plan::quota_from_value(&quota, 1)
            .unwrap()
            .blocking_window(20)
            .is_none()
    );
    s.rpc("account/logout", json!({}), Duration::from_secs(1), &|| {
        false
    })
    .unwrap();
    assert!(
        s.rpc("account/read", json!({}), Duration::from_secs(1), &|| false)
            .unwrap()["account"]
            .is_null()
    );
}
#[test]
fn fake_stdio_passes_normalize_usage_and_retries_without_double_counting() {
    for mode in [
        "success",
        "passive_status",
        "unknown_notification",
        "retry",
        "missing_usage",
        "failed",
        "tool",
        "permission",
    ] {
        let mut s = test_session(mode);
        s.rpc(
            "turn/start",
            json!({"threadId":"thread"}),
            Duration::from_secs(1),
            &|| false,
        )
        .unwrap();
        let output = s.run_turn(
            "thread",
            "turn",
            Instant::now() + Duration::from_secs(1),
            &|| false,
        );
        if matches!(mode, "tool" | "permission") {
            assert_eq!(output, Err("PLAN_CONTAINMENT_VIOLATION"));
        } else if mode == "unknown_notification" {
            assert_eq!(output, Err("PLAN_PROTOCOL_INVALID"));
            assert_eq!(
                s.failure_provider_reason.as_deref(),
                Some("unsupportedProtocolNotification")
            );
        } else if mode == "failed" {
            assert_eq!(output, Err("PLAN_PROVIDER_REJECTED"));
        } else {
            assert!(output.is_ok());
        }
        if mode == "missing_usage" {
            assert!(s.last_usage.is_none());
        }
        if mode == "retry" {
            assert_eq!(s.reported_retries, 1);
        }
        if let Some(usage) = s.last_usage {
            assert_eq!(
                usage.input_tokens + usage.cached_input_tokens + usage.output_tokens,
                130
            );
        }
    }
}
#[test]
fn fake_stdio_declined_malformed_stale_and_deadline_fail_closed() {
    let mut declined = test_session("declined");
    declined
        .rpc(
            "account/login/start",
            json!({}),
            Duration::from_secs(1),
            &|| false,
        )
        .unwrap();
    declined.login_id = Some("login".into());
    declined
        .rpc("account/read", json!({}), Duration::from_secs(1), &|| false)
        .unwrap();
    assert_eq!(declined.login_error, Some("PLAN_LOGIN_DECLINED"));
    for (mode, expected) in [
        ("wrong_id", "PLAN_PROTOCOL_INVALID"),
        ("malformed", "PLAN_PROTOCOL_INVALID"),
        ("timeout", "PLAN_REQUEST_TIMEOUT"),
    ] {
        assert_eq!(
            test_session(mode).rpc(
                "account/read",
                json!({}),
                Duration::from_millis(100),
                &|| false
            ),
            Err(expected)
        );
    }
    assert_eq!(
        test_session("success").rpc("account/read", json!({}), Duration::from_secs(1), &|| true),
        Err("AI_CANCELLED")
    );
    assert_eq!(
        test_session("success").rpc("command/exec", json!({}), Duration::from_secs(1), &|| false),
        Err("PLAN_CONTAINMENT_VIOLATION")
    );
}
#[cfg(target_os = "macos")]
#[test]
#[ignore = "explicit macOS OS containment probe; no network provider request"]
fn os_containment_qualification() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let source = root.join("probe.c");
    let binary = root.join("probe");
    let auth = root.join("auth");
    let scratch = root.join("scratch");
    std::fs::create_dir(&auth).unwrap();
    std::fs::create_dir(&scratch).unwrap();
    let secret = root.join("outside-secret");
    std::fs::write(&secret, "private").unwrap();
    std::fs::write(
        &source,
        include_str!("../tests/fixtures/codex-containment-probe.c"),
    )
    .unwrap();
    assert!(
        Command::new("/usr/bin/clang")
            .arg(&source)
            .arg("-o")
            .arg(&binary)
            .status()
            .unwrap()
            .success()
    );
    let gateway = crate::codex_egress::Gateway::start().unwrap();
    let other = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let policy = sandbox_policy(&binary, &auth, &scratch, gateway.port).unwrap();
    let output = Command::new("/usr/bin/sandbox-exec")
        .args(["-p", &policy])
        .arg(&binary)
        .arg(&secret)
        .arg(scratch.join("allowed"))
        .arg(root.join("outside-write"))
        .arg(gateway.port.to_string())
        .arg(other.local_addr().unwrap().port().to_string())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "OS denied-boundary probe failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(!root.join("outside-write").exists());
    assert!(scratch.join("allowed").exists());
}

#[test]
#[ignore = "explicit offline macOS runtime qualification; no login or inference"]
fn runtime_qualification() {
    let path = PathBuf::from(
        std::env::var("ORT_CODEX_QUALIFY_PATH").expect("temporary official executable"),
    );
    assert_eq!(
        hex::encode(Sha256::digest(std::fs::read(&path).unwrap())),
        compatibility().executable_sha256
    );
    let auth = tempfile::tempdir().unwrap();
    let mut session = Session::start_runtime(
        &Runtime { path },
        auth.path(),
        Instant::now() + Duration::from_secs(30),
        &|| false,
    )
    .expect("contained official app-server");
    let account = session
        .rpc(
            "account/read",
            json!({"refreshToken":false}),
            Duration::from_secs(10),
            &|| false,
        )
        .expect("account protocol");
    assert!(
        account["account"].is_null(),
        "qualification never uses user credentials"
    );
    assert_eq!(session.models.len(), 6);
}
#[test]
fn provider_diagnostics_are_safe_and_model_rejection_disables_only_that_model() {
    let mut s = test_session("success");
    assert_eq!(s.provider_failure(&json!({"codexErrorInfo":{"httpConnectionFailed":{"httpStatusCode":503}},"message":"private resume contents"})), "AI_PROVIDER_TEMPORARY");
    assert_eq!(s.failure_http_status, Some(503));
    assert_eq!(
        s.failure_provider_reason.as_deref(),
        Some("httpConnectionFailed")
    );
    s.selected_model = Some("gpt-6.1-sol".into());
    assert_eq!(
        s.provider_failure(&json!({"message":"This model is not available for your account"})),
        "PLAN_MODEL_UNAVAILABLE"
    );
    assert!(
        !s.models
            .iter()
            .find(|m| m.id == "gpt-6.1-sol")
            .unwrap()
            .supported
    );
    assert!(s.models.iter().filter(|m|m.id!="gpt-6.1-sol").all(|m|m.explanation.as_deref() != Some("Codex rejected access to this model for the connected account. Reconnect after account access changes.")));
}
#[test]
fn startup_can_be_cancelled_before_protocol_initialization() {
    let child = Command::new("/usr/bin/python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codex-app-server.py"))
        .arg("success")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    assert!(matches!(
        Session::from_child(
            child,
            tempfile::tempdir().unwrap(),
            None,
            Instant::now() + Duration::from_secs(1),
            &|| true
        ),
        Err("AI_CANCELLED")
    ));
}
#[test]
fn tools_and_permissions_are_never_allowed() {
    for method in [
        "item/commandExecution/outputDelta",
        "item/commandExecution/requestApproval",
        "item/fileChange/requestApproval",
        "item/tool/call",
        "mcpServer/elicitation/request",
        "item/permissions/requestApproval",
        "unknown",
    ] {
        assert!(!allowed_notification(&json!({"method":method})));
    }
    assert!(!allowed_notification(
        &json!({"method":"item/started","params":{"item":{"type":"commandExecution"}}})
    ));
    assert!(allowed_notification(
        &json!({"method":"item/completed","params":{"item":{"type":"agentMessage"}}})
    ));
}
#[test]
fn sandbox_does_not_allow_tools_or_user_roots() {
    let p = sandbox_policy(
        Path::new("/Applications/CodexCLI.app/Contents/MacOS/codex"),
        Path::new("/private/tmp/ort-auth"),
        Path::new("/private/tmp/ort-work"),
        12345,
    )
    .unwrap();
    assert!(p.contains("(deny default)"));
    assert!(!p.contains("process-fork"));
    assert!(!p.contains("/Users"));
    assert!(!p.contains("/bin/sh"));
    // SecurityServer is also required by native TLS certificate validation.
    // Credential-file access and authorization rights remain denied.
    assert!(!p.contains("/Library/Keychains"));
    assert!(!p.contains("authorization-right-obtain"));
    assert!(!p.contains("com.apple.securityd"));
}
