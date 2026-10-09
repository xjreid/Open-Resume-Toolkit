use super::*;
#[test]
fn download_destinations_are_pinned_and_redirects_cannot_escape_https() {
    assert!(allowed_download_url(
        &reqwest::Url::parse(&manifest().download_url).unwrap()
    ));
    assert!(allowed_download_url(
        &reqwest::Url::parse(
            "https://release-assets.githubusercontent.com/official?token=redirect"
        )
        .unwrap()
    ));
    for url in [
        "http://github.com/openai/codex/releases/download/test",
        "https://github.com/attacker/codex/releases/download/test",
        "https://github.com/openai/codex/releases/latest",
        "https://release-assets.githubusercontent.com.evil.test/x",
        "https://user:password@release-assets.githubusercontent.com/x",
        "https://release-assets.githubusercontent.com:444/x",
        "file:///tmp/codex",
        "https://127.0.0.1/runtime",
        "https://release-assets.githubusercontent.com/x#fragment",
    ] {
        assert!(
            !allowed_download_url(&reqwest::Url::parse(url).unwrap()),
            "{url}"
        );
    }
}
#[test]
fn cancellation_prevents_approval_and_new_attempts_can_reset() {
    let installer = RuntimeInstaller::default();
    installer.update(InstallPhase::Downloading, 10).unwrap();
    installer.cancelled.store(true, Ordering::Release);
    assert_eq!(installer.check_cancel(), Err("PLAN_INSTALL_CANCELLED"));
    assert_eq!(
        installer.update(InstallPhase::AwaitingApproval, 20),
        Err("PLAN_INSTALL_CANCELLED")
    );
    assert_eq!(
        installer.snapshot().unwrap().phase,
        InstallPhase::Downloading
    );
    installer.cancelled.store(false, Ordering::Release);
    installer.update(InstallPhase::Verifying, 20).unwrap();
    assert!(installer.snapshot().unwrap().active());
    installer.update(InstallPhase::Complete, 20).unwrap();
    assert!(!installer.snapshot().unwrap().active());
}
#[test]
fn approval_script_uses_literal_arguments_and_no_download_or_runtime_execution() {
    assert!(APPROVAL_SCRIPT.contains("quoted form of item 1 of argv"));
    assert!(APPROVAL_SCRIPT.contains("quoted form of item 2 of argv"));
    assert!(APPROVAL_SCRIPT.contains("quoted form of item 3 of argv"));
    assert!(APPROVAL_SCRIPT.contains("quoted form of item 4 of argv"));
    assert!(APPROVAL_SCRIPT.contains("/usr/bin/env -i"));
    assert!(APPROVAL_SCRIPT.contains("/usr/bin/perl -T"));
    assert!(!APPROVAL_SCRIPT.contains("curl"));
    assert!(!APPROVAL_SCRIPT.contains("sudo"));
    assert!(!APPROVAL_SCRIPT.contains("app-server"));
}

#[test]
fn unexpected_http_status_and_missing_or_wrong_length_fail_closed() {
    assert!(
        validate_download_response(reqwest::StatusCode::OK, Some(manifest().archive_bytes)).is_ok()
    );
    for status in [200, 201, 204, 206, 301, 302, 400, 401, 404, 429, 500, 503] {
        for length in [
            None,
            Some(0),
            Some(manifest().archive_bytes - 1),
            Some(manifest().archive_bytes + 1),
        ] {
            assert!(
                validate_download_response(reqwest::StatusCode::from_u16(status).unwrap(), length)
                    .is_err()
            );
        }
    }
}

#[tokio::test]
async fn cancellation_wakes_network_waits_and_ignores_stale_notifications() {
    let installer = RuntimeInstaller::default();
    installer.cancellation.notify_one();
    assert!(
        tokio::time::timeout(Duration::from_millis(10), installer.wait_cancelled())
            .await
            .is_err()
    );
    installer.cancelled.store(true, Ordering::Release);
    installer.cancellation.notify_one();
    assert!(
        tokio::time::timeout(Duration::from_millis(10), installer.wait_cancelled())
            .await
            .is_ok()
    );
}

#[test]
#[cfg(target_os = "macos")]
fn macos_approval_arguments_remain_literal_without_invoking_administrator_access() {
    // Exercise the real AppleScript argument bridge with the privilege clause
    // removed. The text must be echoed literally, never evaluated as a command.
    let script = APPROVAL_SCRIPT.replace(" with administrator privileges", "");
    let work = tempfile::tempdir().unwrap();
    let helper = work.path().join("helper");
    let bytes = b"#!/bin/sh\nprintf '%s\\n' \"$1\"\n";
    std::fs::write(&helper, bytes).unwrap();
    let digest = hex::encode(Sha256::digest(bytes));
    let argument = "/private/tmp/a' \" $(false); `false` & spaces";
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", &script, "--", &unprivileged_bootstrap()])
        .arg(&helper)
        .arg(argument)
        .arg(&digest)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "AppleScript argument bridge failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim_end(),
        argument
    );
}

#[cfg(target_os = "macos")]
fn unprivileged_bootstrap() -> String {
    // Exercise the production copy/verify/exec pipeline as our test user. The
    // shipped bootstrap always requires root; no test performs elevation.
    PRIVILEGED_BOOTSTRAP.replace("die \"administrator required\\n\" unless $> == 0;", "")
}

#[test]
#[cfg(target_os = "macos")]
fn elevated_boundary_rejects_replaced_symlinked_and_nonregular_helpers() {
    use std::os::unix::fs::symlink;
    let work = tempfile::tempdir().unwrap();
    let helper = work.path().join("helper");
    let marker = work.path().join("executed");
    let original = b"#!/bin/sh\nprintf '%s\\n' \"$0\"\n";
    let digest = hex::encode(Sha256::digest(original));
    let run = || {
        Command::new("/usr/bin/perl")
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .args(["-T", "-e", &unprivileged_bootstrap(), "--"])
            .arg(&helper)
            .arg(&marker)
            .arg(&digest)
            .output()
            .unwrap()
    };
    std::fs::write(&helper, original).unwrap();
    let valid = run();
    assert!(
        valid.status.success(),
        "{}",
        String::from_utf8_lossy(&valid.stderr)
    );
    let executed = String::from_utf8(valid.stdout).unwrap();
    assert!(executed.starts_with("/private/tmp/ort-codex-root-"));
    assert!(!executed.contains(helper.to_str().unwrap()));

    // Simulate replacement while the user is approving installation. The
    // native pre-elevation digest cannot authorize these different bytes.
    std::fs::write(&helper, b"#!/bin/sh\ntouch \"$1\"\n").unwrap();
    assert!(!run().status.success());
    assert!(!marker.exists());
    std::fs::remove_file(&helper).unwrap();
    let original_path = work.path().join("original");
    std::fs::write(&original_path, original).unwrap();
    symlink(&original_path, &helper).unwrap();
    assert!(!run().status.success());
    std::fs::remove_file(&helper).unwrap();
    std::fs::hard_link(&original_path, &helper).unwrap();
    assert!(!run().status.success());
    std::fs::remove_file(&helper).unwrap();
    assert!(
        Command::new("/usr/bin/mkfifo")
            .arg(&helper)
            .status()
            .unwrap()
            .success()
    );
    let start = Instant::now();
    assert!(!run().status.success());
    assert!(start.elapsed() < Duration::from_secs(1));
    std::fs::remove_file(&helper).unwrap();
    let file = std::fs::File::create(&helper).unwrap();
    file.set_len(16 * 1024 * 1024 + 1).unwrap();
    assert!(!run().status.success());
    assert!(!marker.exists());
}

#[tokio::test]
#[ignore = "explicit pinned official download; never executes a runtime, installs or logs in"]
async fn official_download_verifies_without_administrator_or_account_access() {
    let installer = RuntimeInstaller::default();
    let archive = tempfile::NamedTempFile::new().unwrap();
    download(
        &mut archive.reopen().unwrap(),
        &installer,
        &CancelSignal::default(),
    )
    .await
    .unwrap();
    let mut payload = tempfile::NamedTempFile::new().unwrap();
    crate::codex_archive::extract(archive.path(), payload.as_file_mut()).unwrap();
    ort_codex_install::verify_payload(payload.path()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        payload
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o500))
            .unwrap();
    }
}
