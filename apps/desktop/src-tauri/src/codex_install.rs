//! Opt-in installer. No renderer-controlled URLs, paths, commands or versions.
use crate::{
    ai_request::{AiRequestGate, CancelSignal},
    chatgpt_plan::PlanRuntime,
    codex_runtime,
};
use ort_codex_install::{Result, manifest};
use ort_domain::CommandResponse;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{Manager, WebviewWindow};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InstallPhase {
    Idle,
    Downloading,
    Verifying,
    AwaitingApproval,
    Checking,
    Complete,
    Cancelled,
    Failed,
}
#[derive(Clone, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInstallStatus {
    pub phase: InstallPhase,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub error_code: Option<String>,
}
impl Default for RuntimeInstallStatus {
    fn default() -> Self {
        Self {
            phase: InstallPhase::Idle,
            downloaded_bytes: 0,
            total_bytes: manifest().archive_bytes,
            error_code: None,
        }
    }
}
impl RuntimeInstallStatus {
    fn active(&self) -> bool {
        matches!(
            self.phase,
            InstallPhase::Downloading
                | InstallPhase::Verifying
                | InstallPhase::AwaitingApproval
                | InstallPhase::Checking
        )
    }
}
#[derive(Default)]
pub(crate) struct RuntimeInstaller {
    status: Mutex<RuntimeInstallStatus>,
    cancelled: AtomicBool,
    cancellation: tokio::sync::Notify,
}
impl RuntimeInstaller {
    fn snapshot(&self) -> Result<RuntimeInstallStatus> {
        self.status
            .lock()
            .map(|s| s.clone())
            .map_err(|_| "PLAN_INSTALL_IO_FAILED")
    }
    fn update(&self, phase: InstallPhase, bytes: u64) -> Result<()> {
        let mut state = self.status.lock().map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
        if matches!(
            phase,
            InstallPhase::Verifying | InstallPhase::AwaitingApproval
        ) && self.cancelled.load(Ordering::Acquire)
        {
            return Err("PLAN_INSTALL_CANCELLED");
        }
        state.phase = phase;
        state.downloaded_bytes = bytes;
        Ok(())
    }
    fn check_cancel(&self) -> Result<()> {
        if self.cancelled.load(Ordering::Acquire) {
            Err("PLAN_INSTALL_CANCELLED")
        } else {
            Ok(())
        }
    }
    async fn wait_cancelled(&self) {
        loop {
            let notified = self.cancellation.notified();
            if self.cancelled.load(Ordering::Acquire) {
                return;
            }
            notified.await;
        }
    }
}
fn response<T: Serialize>(value: Result<T>) -> CommandResponse<T> {
    match value {
        Ok(value) => CommandResponse::success(value),
        Err(code) => CommandResponse::failure(code, "errors.codexInstall", true),
    }
}
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn load_codex_runtime_install(window: WebviewWindow) -> CommandResponse<RuntimeInstallStatus> {
    if window.label() != "main" {
        return crate::window_not_authorized();
    }
    response(window.state::<RuntimeInstaller>().snapshot())
}
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn cancel_codex_runtime_install(
    window: WebviewWindow,
) -> CommandResponse<RuntimeInstallStatus> {
    if window.label() != "main" {
        return crate::window_not_authorized();
    }
    let installer = window.state::<RuntimeInstaller>();
    let state = installer
        .status
        .lock()
        .map_err(|_| "PLAN_INSTALL_IO_FAILED");
    response(state.and_then(|s| {
        if !matches!(s.phase, InstallPhase::Downloading | InstallPhase::Verifying) {
            return Err("PLAN_INSTALL_APPROVAL_PENDING");
        }
        installer.cancelled.store(true, Ordering::Release);
        installer.cancellation.notify_one();
        Ok(s.clone())
    }))
}
#[tauri::command]
pub async fn install_codex_runtime(window: WebviewWindow) -> CommandResponse<RuntimeInstallStatus> {
    if window.label() != "main" {
        return crate::window_not_authorized();
    }
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return response(Err("PLAN_PLATFORM_UNSUPPORTED"));
    }
    let gate = window.state::<AiRequestGate>();
    let Some(lease) = gate.begin(Uuid::now_v7()) else {
        return response(Err("AI_BUSY"));
    };
    let installer = window.state::<RuntimeInstaller>();
    {
        let Ok(mut state) = installer.status.lock() else {
            return response(Err("PLAN_INSTALL_IO_FAILED"));
        };
        if state.active() {
            return response(Err("AI_BUSY"));
        }
        *state = RuntimeInstallStatus {
            phase: InstallPhase::Downloading,
            ..RuntimeInstallStatus::default()
        };
        installer.cancelled.store(false, Ordering::Release);
    }
    let result = run(&window, &installer, lease.cancellation_signal())
        .await
        .map_err(|code| {
            if code == "AI_CANCELLED" {
                "PLAN_INSTALL_CANCELLED"
            } else {
                code
            }
        });
    let Ok(mut state) = installer.status.lock() else {
        return response(Err("PLAN_INSTALL_IO_FAILED"));
    };
    match result {
        Ok(()) => {
            state.phase = InstallPhase::Complete;
            state.error_code = None;
        }
        Err(code) => {
            state.phase = if code == "PLAN_INSTALL_CANCELLED" {
                InstallPhase::Cancelled
            } else {
                InstallPhase::Failed
            };
            state.error_code = Some(code.into());
        }
    }
    CommandResponse::success(state.clone())
}

fn allowed_download_url(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && url.fragment().is_none()
        && (url.as_str() == manifest().download_url
            || url.host_str() == Some("release-assets.githubusercontent.com"))
}
async fn download(
    file: &mut std::fs::File,
    installer: &RuntimeInstaller,
    signal: &CancelSignal,
) -> Result<()> {
    let client = reqwest::Client::builder()
        .https_only(true)
        .no_proxy()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(180))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() <= 3 && allowed_download_url(attempt.url()) {
                attempt.follow()
            } else {
                attempt.error("unapproved runtime download redirect")
            }
        }))
        .build()
        .map_err(|_| "PLAN_INSTALL_DOWNLOAD_FAILED")?;
    let request = client
        .get(&manifest().download_url)
        .header("Accept-Encoding", "identity")
        .header("User-Agent", "Open-Resume-Toolkit-runtime-installer")
        .send();
    let mut result = tokio::select! {
        result = request => result.map_err(|_| "PLAN_INSTALL_DOWNLOAD_FAILED")?,
        () = installer.wait_cancelled() => return Err("PLAN_INSTALL_CANCELLED"),
        () = signal.wait() => return Err("PLAN_INSTALL_CANCELLED"),
    };
    validate_download_response(result.status(), result.content_length())?;
    let mut total = 0_u64;
    let mut digest = Sha256::new();
    loop {
        installer.check_cancel()?;
        let chunk = tokio::select! {
            result = tokio::time::timeout(Duration::from_secs(10), result.chunk()) => result.map_err(|_| "PLAN_INSTALL_DOWNLOAD_FAILED")?.map_err(|_| "PLAN_INSTALL_DOWNLOAD_FAILED")?,
            () = installer.wait_cancelled() => return Err("PLAN_INSTALL_CANCELLED"),
            () = signal.wait() => return Err("PLAN_INSTALL_CANCELLED"),
        };
        let Some(chunk) = chunk else {
            break;
        };
        total = total
            .checked_add(chunk.len() as u64)
            .ok_or("PLAN_INSTALL_VERIFY_FAILED")?;
        if total > manifest().archive_bytes {
            return Err("PLAN_INSTALL_VERIFY_FAILED");
        }
        digest.update(&chunk);
        file.write_all(&chunk)
            .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
        installer.update(InstallPhase::Downloading, total)?;
    }
    if total != manifest().archive_bytes
        || hex::encode(digest.finalize()) != manifest().archive_sha256
    {
        return Err("PLAN_INSTALL_VERIFY_FAILED");
    }
    file.sync_all().map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    Ok(())
}
fn validate_download_response(status: reqwest::StatusCode, length: Option<u64>) -> Result<()> {
    if status != reqwest::StatusCode::OK || length != Some(manifest().archive_bytes) {
        Err("PLAN_INSTALL_DOWNLOAD_FAILED")
    } else {
        Ok(())
    }
}
async fn run(
    window: &WebviewWindow,
    installer: &RuntimeInstaller,
    signal: Arc<CancelSignal>,
) -> Result<()> {
    // Stop the supervised session before replacing its executable. Credentials
    // and connection policy are not changed by installation.
    let runtime = window.state::<PlanRuntime>();
    if runtime.login_pending() {
        return Err("PLAN_LOGIN_PENDING");
    }
    runtime.stop();
    let work = tempfile::Builder::new()
        .prefix("ort-codex-install-")
        .tempdir()
        .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    let work_path = work
        .path()
        .canonicalize()
        .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    let helper = prepare_helper(&work_path)?;
    let archive = work_path.join("release.tar.gz");
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .read(true)
        .open(&archive)
        .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    download(&mut file, installer, &signal).await?;
    drop(file);
    installer.update(InstallPhase::Verifying, manifest().archive_bytes)?;
    let payload = work_path.join("codex");
    let archive_path = archive.clone();
    let payload_path = payload.clone();
    let native = window.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut output = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&payload_path)
            .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
        crate::codex_archive::extract(&archive_path, &mut output)?;
        drop(output);
        ort_codex_install::verify_payload(&payload_path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&payload_path, std::fs::Permissions::from_mode(0o500))
                .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
        }
        native.state::<RuntimeInstaller>().check_cancel()
    })
    .await
    .map_err(|_| "PLAN_INSTALL_VERIFY_FAILED")??;
    if signal.is_cancelled() {
        return Err("PLAN_INSTALL_CANCELLED");
    }
    installer.update(InstallPhase::AwaitingApproval, manifest().archive_bytes)?;
    tauri::async_runtime::spawn_blocking(move || authorize(&helper, &payload))
        .await
        .map_err(|_| "PLAN_INSTALL_PERMISSION_DENIED")??;
    installer.update(InstallPhase::Checking, manifest().archive_bytes)?;
    tauri::async_runtime::spawn_blocking(|| {
        let path = Path::new(ort_codex_install::DESTINATION);
        // Installation verifies bytes and protected ownership only. Runtime
        // startup belongs exclusively to enabled PlanRuntime sessions.
        codex_runtime::verify_runtime(path)
    })
    .await
    .map_err(|_| "PLAN_INSTALL_VERIFY_FAILED")??;
    Ok(())
}
fn prepare_helper(work: &Path) -> Result<PathBuf> {
    let digest = option_env!("ORT_CODEX_INSTALLER_SHA256").ok_or("PLAN_INSTALL_HELPER_MISSING")?;
    let binary = std::env::current_exe().map_err(|_| "PLAN_INSTALL_HELPER_MISSING")?;
    let original = binary
        .parent()
        .and_then(Path::parent)
        .ok_or("PLAN_INSTALL_HELPER_MISSING")?
        .join("Helpers/ORT Codex Installer.app/Contents/MacOS/ort-codex-install");
    if original.canonicalize().ok().as_deref() != Some(original.as_path()) {
        return Err("PLAN_INSTALL_HELPER_MISSING");
    }
    let meta = original
        .symlink_metadata()
        .map_err(|_| "PLAN_INSTALL_HELPER_MISSING")?;
    if !meta.is_file() || meta.len() > 16 * 1024 * 1024 {
        return Err("PLAN_INSTALL_HELPER_MISSING");
    }
    let helper = work.join("ort-codex-install");
    let mut input = ort_codex_install::open_payload(&original, meta.len())?;
    let mut output = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&helper)
        .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    ort_codex_install::check_bytes(&mut input, &mut output, meta.len(), digest, false)?;
    output.sync_all().map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        output
            .set_permissions(std::fs::Permissions::from_mode(0o500))
            .map_err(|_| "PLAN_INSTALL_IO_FAILED")?;
    }
    Ok(helper)
}
const PRIVILEGED_BOOTSTRAP: &str = include_str!("codex-install-bootstrap.pl");
const APPROVAL_SCRIPT: &str = r#"on run argv
return do shell script ("/usr/bin/env -i PATH=/usr/bin:/bin /usr/bin/perl -T -e " & (quoted form of item 1 of argv) & " -- " & (quoted form of item 2 of argv) & " " & (quoted form of item 3 of argv) & " " & (quoted form of item 4 of argv)) with administrator privileges
end run"#;
fn authorize(helper: &Path, payload: &Path) -> Result<()> {
    let digest = option_env!("ORT_CODEX_INSTALLER_SHA256").ok_or("PLAN_INSTALL_HELPER_MISSING")?;
    let mut child = Command::new("/usr/bin/osascript")
        .args(["-e", APPROVAL_SCRIPT, "--"])
        .arg(PRIVILEGED_BOOTSTRAP)
        .arg(helper)
        .arg(payload)
        .arg(digest)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", "/var/empty")
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "PLAN_INSTALL_PERMISSION_DENIED")?;
    let end = Instant::now() + Duration::from_secs(300);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < end => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("PLAN_INSTALL_APPROVAL_TIMEOUT");
            }
        }
    }
    let result = child
        .wait_with_output()
        .map_err(|_| "PLAN_INSTALL_PERMISSION_DENIED")?;
    if result.status.success() && result.stdout == b"ORT_CODEX_INSTALLED\n" {
        return Ok(());
    }
    let text = String::from_utf8_lossy(&result.stderr);
    for code in [
        "PLAN_INSTALL_UNSAFE_PATH",
        "PLAN_INSTALL_VERIFY_FAILED",
        "PLAN_INSTALL_IO_FAILED",
    ] {
        if text.contains(code) {
            return Err(code);
        }
    }
    if text.contains("(-128)") {
        return Err("PLAN_INSTALL_CANCELLED");
    }
    Err("PLAN_INSTALL_PERMISSION_DENIED")
}

#[cfg(test)]
#[path = "codex_install_tests.rs"]
mod tests;
