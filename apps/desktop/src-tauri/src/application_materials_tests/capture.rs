use super::*;

fn browser_frame(target: &str, text: &str, now_ms: i64) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "protocolVersion": ort_ipc::PROTOCOL_VERSION,
        "requestId": uuid::Uuid::now_v7(),
        "sentAt": jiff::Timestamp::from_millisecond(now_ms).unwrap().to_string(),
        "kind": "capture.selection",
        "payload": {
            "text": text,
            "url": "https://example.test/job",
            "title": "Synthetic job",
            "browser": "chrome",
            "target": target
        }
    }))
    .unwrap()
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "end-to-end capture review regression"
)]
fn authenticated_capture_waits_for_review_and_preserves_existing_work() {
    let temp = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store =
        ort_storage::EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
    let now = 1_800_000_000_000;
    let job = browser_frame("job", "Synthetic selected job", now);
    let job_id = accept_authenticated_capture(&store, &job, now).unwrap();
    assert!(load_stage_one(&store).unwrap().is_none());
    assert!(load(&store).unwrap().is_none());
    assert_eq!(
        accept_authenticated_capture(&store, &job, now).unwrap(),
        job_id
    );
    assert!(matches!(
        accept_authenticated_capture(&store, &browser_frame("question", "Why?", now), now),
        Err(StorageError::RevisionConflict)
    ));
    resolve_pending_capture(
        &store,
        job_id,
        true,
        None,
        Some("Synthetic selected job"),
        Some("https://example.test/job"),
    )
    .unwrap();
    assert_eq!(
        load_stage_one(&store)
            .unwrap()
            .unwrap()
            .draft
            .job_description,
        "Synthetic selected job"
    );
    assert!(load_pending_capture(&store).unwrap().is_none());
    let next_job = browser_frame("job", "Replacement job", now);
    let next_id = accept_authenticated_capture(&store, &next_job, now).unwrap();
    assert!(matches!(
        resolve_pending_capture(&store, next_id, true, Some(1), Some(""), Some("")),
        Err(StorageError::InvalidData)
    ));
    assert!(load_pending_capture(&store).unwrap().is_some());
    assert!(matches!(
        resolve_pending_capture(
            &store,
            next_id,
            true,
            None,
            Some("Replacement job"),
            Some("https://example.test/job")
        ),
        Err(StorageError::RevisionConflict)
    ));
    assert_eq!(
        load_stage_one(&store)
            .unwrap()
            .unwrap()
            .draft
            .job_description,
        "Synthetic selected job"
    );
    let current_revision = load_stage_one(&store).unwrap().unwrap().revision;
    resolve_pending_capture(
        &store,
        next_id,
        true,
        Some(current_revision),
        Some("Edited replacement job"),
        Some(""),
    )
    .unwrap();
    assert_eq!(
        load_stage_one(&store)
            .unwrap()
            .unwrap()
            .draft
            .job_description,
        "Edited replacement job"
    );

    save(&store, None, &workspace()).unwrap();
    let question = browser_frame("question", "Why this role?", now);
    let question_id = accept_authenticated_capture(&store, &question, now).unwrap();
    assert!(load(&store).unwrap().unwrap().workspace.question.is_empty());
    assert!(matches!(
        resolve_pending_capture(
            &store,
            question_id,
            true,
            None,
            Some("Why this role?"),
            Some("https://example.test/job")
        ),
        Err(StorageError::RevisionConflict)
    ));
    let current_revision = load(&store).unwrap().unwrap().revision;
    resolve_pending_capture(
        &store,
        question_id,
        true,
        Some(current_revision),
        Some("Why this team?"),
        Some("https://example.test/job"),
    )
    .unwrap();
    assert_eq!(
        load(&store).unwrap().unwrap().workspace.question,
        "Why this team?"
    );
    assert!(load_pending_capture(&store).unwrap().is_none());
}

#[test]
fn automatic_job_capture_replaces_fields_and_preserves_conflicting_data() {
    let temp = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store =
        ort_storage::EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
    let old = save_stage_one(
        &store,
        None,
        &StageOneDraft {
            job_description: "Old job".into(),
            job_url: "https://example.test/old".into(),
            style: DocumentStyle::Technical,
        },
    )
    .unwrap();
    let now = 1_800_000_000_000;
    let id =
        accept_authenticated_capture(&store, &browser_frame("job", "New captured job", now), now)
            .unwrap();
    assert!(matches!(
        apply_pending_job_capture(&store, id, None),
        Err(StorageError::RevisionConflict)
    ));
    assert_eq!(
        load_stage_one(&store)
            .unwrap()
            .unwrap()
            .draft
            .job_description,
        "Old job"
    );
    assert!(load_pending_capture(&store).unwrap().is_some());
    assert!(matches!(
        apply_pending_job_capture(&store, uuid::Uuid::now_v7(), Some(old.revision)),
        Err(StorageError::RevisionConflict)
    ));
    let updated = apply_pending_job_capture(&store, id, Some(old.revision)).unwrap();
    assert_eq!(updated.draft.job_description, "New captured job");
    assert_eq!(updated.draft.job_url, "https://example.test/job");
    assert!(updated.revision > old.revision);
    assert!(load_pending_capture(&store).unwrap().is_none());
    assert!(load(&store).unwrap().is_none());

    let question =
        accept_authenticated_capture(&store, &browser_frame("question", "Why?", now), now).unwrap();
    assert!(matches!(
        apply_pending_job_capture(&store, question, Some(updated.revision)),
        Err(StorageError::InvalidData)
    ));
    assert!(load_pending_capture(&store).unwrap().is_some());
    resolve_pending_capture(&store, question, false, None, None, None).unwrap();
    save(&store, None, &workspace()).unwrap();
    let job =
        accept_authenticated_capture(&store, &browser_frame("job", "Later job", now), now).unwrap();
    assert!(matches!(
        apply_pending_job_capture(&store, job, Some(updated.revision)),
        Err(StorageError::RevisionConflict)
    ));
    assert_eq!(
        load_stage_one(&store)
            .unwrap()
            .unwrap()
            .draft
            .job_description,
        "New captured job"
    );
    assert!(load_pending_capture(&store).unwrap().is_some());
    assert!(load(&store).unwrap().is_some());
}

/// Run with the real Chrome binary and an exact-ID development native host.
/// All captures, database keys and browser state belong to a temporary QA profile.
#[test]
#[ignore = "launches real Chrome in a disposable profile; run explicitly"]
#[cfg(all(feature = "dev-browser-bridge", target_os = "macos"))]
#[expect(
    clippy::too_many_lines,
    reason = "real Chrome to encrypted review integration"
)]
fn development_chrome_native_roundtrip() {
    use ort_ipc::capture_session::CaptureSession;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};
    struct ChildGuard(std::process::Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let temp = tempfile::Builder::new()
        .prefix("ort-chrome-intake-")
        .tempdir_in("/private/tmp")
        .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap();
    let output = std::process::Command::new("node")
        .arg(root.join("tools/dev-browser-bridge.mjs"))
        .arg("id")
        .output()
        .unwrap();
    assert!(output.status.success());
    let id = String::from_utf8(output.stdout).unwrap();
    let store = Arc::new(
        ort_storage::EncryptedStore::open_or_initialize(
            &temp.path().join("data"),
            "chrome-qa",
            &MemoryDatabaseKeyVault::new(),
        )
        .unwrap(),
    );
    let intake = store.clone();
    let sessions = Arc::new(Mutex::new(CaptureSession::default()));
    let authority = sessions.clone();
    let socket = temp.path().join("socket");
    let server = ort_ipc::development::Server::start(&socket, id.trim(), move |request, bytes| {
        let now = jiff::Timestamp::now().as_millisecond();
        crate::browser_bridge::receive_development_request(
            &authority,
            request,
            bytes,
            now,
            true,
            |bytes| accept_authenticated_capture(&intake, bytes, now),
        )
    })
    .unwrap();
    let child = std::process::Command::new("node")
        .arg(root.join("apps/extension/scripts/smoke-chrome.mjs"))
        .arg("dev-bridge")
        .env("ORT_DEV_QA_ROOT", temp.path())
        .env("ORT_DEV_BRIDGE_DIRECTORY", &socket)
        .env("ORT_DEV_QA_HOST", root.join("target/debug/ort-native-host"))
        .spawn()
        .unwrap();
    let mut child = ChildGuard(child);
    let now = || jiff::Timestamp::now().as_millisecond();
    let wait = |name: &str| {
        let start = Instant::now();
        while !temp.path().join(name).exists() {
            assert!(
                start.elapsed() < Duration::from_secs(30),
                "Chrome did not reach {name}"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    };
    let signal = |name: &str| std::fs::write(temp.path().join(name), b"").unwrap();
    let arm = |name: &str, target: &str| {
        let start = Instant::now();
        while !sessions.lock().unwrap().connected(now()) {
            assert!(start.elapsed() < Duration::from_secs(10));
            std::thread::sleep(Duration::from_millis(25));
        }
        let id = sessions
            .lock()
            .unwrap()
            .start(target, now())
            .unwrap()
            .session_id
            .unwrap();
        signal(&format!("{name}-armed"));
        id
    };
    let pending = || {
        let start = Instant::now();
        loop {
            if let Some(pending) = load_pending_capture(&store).unwrap() {
                break pending.capture;
            }
            assert!(
                start.elapsed() < Duration::from_secs(5),
                "capture was not delivered"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
    };
    let phase = |value: &str| {
        let start = Instant::now();
        while sessions.lock().unwrap().status(now()).phase != value {
            assert!(start.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(25));
        }
    };
    wait("browser-ready");
    let cancelled = arm("cancel", "job");
    wait("cancel-selecting");
    phase("selecting");
    sessions.lock().unwrap().cancel(cancelled, now());
    wait("cancelled");
    assert!(
        sessions
            .lock()
            .unwrap()
            .authorize(cancelled, "job", now())
            .is_err()
    );
    assert!(load_pending_capture(&store).unwrap().is_none());
    let job_id = arm("job", "job");
    wait("job-ready");
    let job = pending();
    assert_eq!(job.request_id, job_id);
    assert_eq!(
        job.payload.text,
        "Synthetic job description\nRésumé experience required."
    );
    let origin = std::fs::read_to_string(temp.path().join("browser-ready")).unwrap();
    assert_eq!(job.payload.url, format!("{origin}/job?jobId=42"));
    assert_eq!(job.payload.title, "Synthetic job");
    assert!(load_stage_one(&store).unwrap().is_none());
    let applied = apply_pending_job_capture(&store, job.request_id, None).unwrap();
    assert_eq!(applied.draft.job_url, job.payload.url);
    assert!(load_pending_capture(&store).unwrap().is_none());
    assert_eq!(
        load_stage_one(&store)
            .unwrap()
            .unwrap()
            .draft
            .job_description,
        job.payload.text
    );
    save(&store, None, &workspace()).unwrap();
    signal("job-reviewed");
    arm("question", "question");
    wait("question-ready");
    let question = pending();
    assert_eq!(question.payload.target, "question");
    assert_eq!(question.payload.text, "Why this team?");
    assert!(load(&store).unwrap().unwrap().workspace.question.is_empty());
    let revision = load(&store).unwrap().unwrap().revision;
    resolve_pending_capture(
        &store,
        question.request_id,
        true,
        Some(revision),
        Some(&question.payload.text),
        Some(&question.payload.url),
    )
    .unwrap();
    assert_eq!(
        load(&store).unwrap().unwrap().workspace.question,
        question.payload.text
    );
    signal("question-reviewed");
    arm("partial", "question");
    wait("partial-ready");
    let partial = pending();
    assert_eq!(partial.payload.text, "two");
    resolve_pending_capture(&store, partial.request_id, false, None, None, None).unwrap();
    signal("partial-reviewed");
    for (name, target, expected) in [
        (
            "scrolled",
            "job",
            "Beginning of long job\nMiddle of long job\nEnd of long job",
        ),
        (
            "pane",
            "question",
            "Beginning of panel job\nMiddle of panel job\nEnd of panel job",
        ),
    ] {
        let request_id = arm(name, target);
        wait(&format!("{name}-ready"));
        let capture = pending();
        assert_eq!(capture.request_id, request_id);
        assert_eq!(capture.payload.target, target);
        assert_eq!(capture.payload.text, expected);
        assert_eq!(capture.payload.url, format!("{origin}/job?jobId=42"));
        resolve_pending_capture(&store, request_id, false, None, None, None).unwrap();
        signal(&format!("{name}-reviewed"));
    }
    arm("empty", "question");
    wait("empty-picked");
    phase("idle");
    assert_eq!(
        sessions.lock().unwrap().status(now()).error,
        Some("EMPTY_SELECTION")
    );
    assert!(load_pending_capture(&store).unwrap().is_none());
    arm("reload", "question");
    wait("page-reloaded");
    phase("idle");
    assert_eq!(
        sessions.lock().unwrap().status(now()).error,
        Some("PAGE_CHANGED")
    );
    assert!(load_pending_capture(&store).unwrap().is_none());
    sessions.lock().unwrap().disconnect(now());
    drop(server);
    assert!(!socket.join("session.json").exists());
    assert!(!socket.join("bridge.sock").exists());
    signal("disconnected");
    let start = Instant::now();
    loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(20));
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn chrome_package_capture_reaches_desktop_review_contract() {
    let temp = TempDir::new().unwrap();
    let vault = MemoryDatabaseKeyVault::new();
    let store =
        ort_storage::EncryptedStore::open_or_initialize(temp.path(), "test", &vault).unwrap();
    let frame = include_bytes!("../../../../../fixtures/ipc/chrome-capture.v1.json");
    let capture: ort_ipc::CaptureEnvelope = serde_json::from_slice(frame).unwrap();
    let now = capture
        .sent_at
        .parse::<jiff::Timestamp>()
        .unwrap()
        .as_millisecond();
    let request_id = accept_authenticated_capture(&store, frame, now).unwrap();
    assert_eq!(request_id, capture.request_id);
    assert!(load_stage_one(&store).unwrap().is_none());
    let pending = load_pending_capture(&store).unwrap().unwrap();
    assert_eq!(pending.capture.payload.text, capture.payload.text);
    assert_eq!(pending.capture.payload.url, capture.payload.url);
    resolve_pending_capture(
        &store,
        request_id,
        true,
        None,
        Some(&capture.payload.text),
        Some(&capture.payload.url),
    )
    .unwrap();
    let draft = load_stage_one(&store).unwrap().unwrap().draft;
    assert_eq!(draft.job_description, capture.payload.text);
    assert_eq!(draft.job_url, capture.payload.url);
    assert!(load_pending_capture(&store).unwrap().is_none());
}
