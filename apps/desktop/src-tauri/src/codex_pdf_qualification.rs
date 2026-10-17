//! Explicit offline check of the pinned executable, with a loopback-only fake
//! Responses provider. No credentials, real account, or external inference.
use super::*;
use base64::{Engine, engine::general_purpose::STANDARD};
use std::{io::Write, net::TcpListener};

fn fixture_response() -> String {
    json!({"contact":{"fullName":"Synthetic Example","email":"","phone":"","location":"","links":[{"label":"LinkedIn","url":"linkedin.com/in/synthetic-example"}]},
        "sections":[{"heading":"Projects","entries":[{"leftHeading":"Synthetic Project","leftSubheading":"[GitHub](https://github.com/synthetic-example/project?tab=readme#details)","headingSuffix":"","rightTop":"2025","rightMiddle":"","rightBottom":"","mainInfo":{"format":"bullets","items":["Synthetic fixture output."]}}]}]}).to_string()
}

fn fixture_images(prepared: &crate::codex_import::PreparedInput) -> Vec<String> {
    let directory = std::env::var_os("ORT_CODEX_QUALIFY_PAGE_DIR").map(PathBuf::from);
    prepared
        .items
        .iter()
        .filter_map(|item| item["path"].as_str())
        .enumerate()
        .map(|(index, path)| {
            // Optional browser-rendered synthetic pages exercise full-size input.
            if let Some(directory) = &directory {
                let bytes =
                    std::fs::read(directory.join(format!("page-{}.png", index + 1))).unwrap();
                assert!(bytes.len() <= 3 * 1024 * 1024 && bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
                std::fs::write(path, bytes).unwrap();
            }
            format!(
                "data:image/png;base64,{}",
                STANDARD.encode(std::fs::read(path).unwrap())
            )
        })
        .collect()
}

fn provider(listener: &TcpListener) -> Value {
    listener.set_nonblocking(true).unwrap();
    let end = Instant::now() + Duration::from_secs(30);
    let (mut stream, _) = loop {
        if let Ok(connection) = listener.accept() {
            break connection;
        }
        assert!(
            Instant::now() < end,
            "runtime did not contact local fixture"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.starts_with("POST /v1/responses "));
    let mut length = None;
    loop {
        line.clear();
        reader.read_line(&mut line).unwrap();
        if line == "\r\n" {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = Some(value.trim().parse::<usize>().unwrap());
        }
        assert!(!line.to_ascii_lowercase().starts_with("authorization:"));
        assert!(!line.to_ascii_lowercase().starts_with("content-encoding:"));
    }
    let length = length.unwrap();
    assert!(length < 1_000_000);
    let mut body = vec![0; length];
    reader.read_exact(&mut body).unwrap();
    let request = serde_json::from_slice(&body).unwrap();
    let item = json!({"type":"message","id":"msg_fixture","role":"assistant","status":"completed","phase":"final_answer","content":[{"type":"output_text","text":fixture_response(),"annotations":[]}]});
    let events = [
        json!({"type":"response.created","response":{"id":"resp_fixture","status":"in_progress","output":[]}}),
        json!({"type":"response.output_item.added","output_index":0,"item":item}),
        json!({"type":"response.output_item.done","output_index":0,"item":item}),
        json!({"type":"response.completed","response":{"id":"resp_fixture","status":"completed","output":[item],"usage":{"input_tokens":20,"output_tokens":30,"total_tokens":50,"input_tokens_details":{"cached_tokens":0},"output_tokens_details":{"reasoning_tokens":0}}}}),
    ];
    let mut body = String::new();
    for event in &events {
        use std::fmt::Write;
        write!(
            body,
            "event: {}\ndata: {event}\n\n",
            event["type"].as_str().unwrap()
        )
        .unwrap();
    }
    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
    request
}

fn qualification_links() -> Vec<ort_ai::resume_import::PdfHyperlink> {
    match std::env::var_os("ORT_CODEX_QUALIFY_LINKS_PATH") {
        Some(path) => serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap(),
        None => vec![ort_ai::resume_import::PdfHyperlink {
            page: 1,
            rect: [80, 210, 160, 230],
            url: "https://github.com/synthetic-example/project?tab=readme#details".into(),
        }],
    }
}

fn request_links(request: &Value) -> Value {
    let input = request["input"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["content"].as_array())
        .flatten()
        .filter_map(|item| item["text"].as_str())
        .filter_map(|text| serde_json::from_str::<Value>(text).ok())
        .find(|value| value.get("pdfHyperlinks").is_some())
        .unwrap();
    input["pdfHyperlinks"]["links"].clone()
}

#[test]
#[ignore = "explicit offline official-runtime image qualification; loopback mock provider only, no account or external inference"]
fn pdf_image_runtime_qualification() {
    let path = PathBuf::from(std::env::var_os("ORT_CODEX_QUALIFY_PATH").unwrap());
    ort_codex_install::verify_payload(&path).unwrap();
    let auth = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let config = format!(
        "model_provider = \"pdf_fixture\"\n{}\n[model_providers.pdf_fixture]\nname = \"Offline PDF fixture\"\nbase_url = \"http://127.0.0.1:{port}/v1\"\nwire_api = \"responses\"\nrequires_openai_auth = false\nsupports_websockets = false\nrequest_max_retries = 0\nstream_max_retries = 0\n",
        include_str!("codex-runtime.toml")
    );
    std::fs::write(auth.path().join("config.toml"), config).unwrap();
    // Same OS sandbox, except the one allowed loopback port is the fixture.
    // The provider override is confined to this disposable test configuration.
    let policy = sandbox_policy(&path, auth.path(), scratch.path(), port).unwrap();
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
    let end = Instant::now() + Duration::from_secs(30);
    let mut session = Session::from_child(child, scratch, None, end, &|| false).unwrap();
    assert!(session.image_models.contains("gpt-6.1-sol"));
    let account = session
        .rpc(
            "account/read",
            json!({"refreshToken":false}),
            Duration::from_secs(3),
            &|| false,
        )
        .unwrap();
    assert!(account["account"].is_null());
    let links = qualification_links();
    let expected_links = serde_json::to_value(&links).unwrap();
    let pages = crate::import_review::ai_pdf::PdfPages::fixture_with_links(2, links);
    let prepared = crate::codex_import::prepare(
        session.scratch_path(),
        &ort_ai::resume_import::input_with_pdf_hyperlinks(pages.links()).to_string(),
        Some(&pages),
        &|| false,
    )
    .unwrap();
    let image_path = PathBuf::from(prepared.items[2]["path"].as_str().unwrap());
    let expected_images = fixture_images(&prepared);
    let server = std::thread::spawn(move || provider(&listener));
    let output = session.with_ephemeral_thread(json!({"model":"gpt-6.1-sol","modelProvider":"pdf_fixture","ephemeral":true,"approvalPolicy":"never","sandbox":"read-only","cwd":session.scratch_path(),"baseInstructions":ort_ai::resume_import::SYSTEM,"developerInstructions":"Return only JSON. Never use tools."}), end, &|| false,
        |session, _, thread| {
            let turn = session.rpc("turn/start", json!({"threadId":thread,"model":"gpt-6.1-sol","effort":"medium","approvalPolicy":"never","input":prepared.items,"outputSchema":ort_ai::resume_import::output_schema()}), Duration::from_secs(10), &|| false)?;
            session.run_turn(thread, turn["turn"]["id"].as_str().unwrap(), end, &|| false)
        }).unwrap();
    let document = ort_ai::resume_import::decode(&output).unwrap();
    document
        .validate(ort_domain::DocumentLimits::default())
        .unwrap();
    assert_eq!(document.contact.full_name, "Synthetic Example");
    assert_eq!(
        document.contact.links[0].url,
        "https://linkedin.com/in/synthetic-example"
    );
    assert_eq!(
        document.sections[0].entries[0].subheading,
        "[GitHub](https://github.com/synthetic-example/project?tab=readme#details)"
    );
    let request = server.join().unwrap();
    assert_eq!(request["model"], "gpt-6.1-sol");
    assert_eq!(request["reasoning"]["effort"], "medium");
    assert_eq!(request_links(&request), expected_links);
    assert_eq!(
        request["text"]["format"]["schema"],
        ort_ai::resume_import::output_schema()
    );
    let images: Vec<_> = request["input"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["content"].as_array())
        .flatten()
        .filter(|item| item["type"] == "input_image")
        .collect();
    assert_eq!(images.len(), 2, "both page images must reach the provider");
    for (image, expected) in images.iter().zip(expected_images) {
        // This runtime preserves the original bytes, but omits a separate
        // detail field on the Responses request. Verify the actual payload.
        assert_eq!(image["image_url"], expected);
    }
    drop(prepared);
    assert!(!image_path.exists());
    assert!(!auth.path().join("auth.json").exists());
}
