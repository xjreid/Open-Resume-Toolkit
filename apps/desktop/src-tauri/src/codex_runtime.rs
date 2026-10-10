//! Supervised stdio protocol. No arbitrary executable, tools, approvals, or
//! provider content is exposed to the renderer or persisted in diagnostics.
use ort_ai::plan::{PlanModel, models_from_catalog};
use serde_json::{Value, json};
#[cfg(test)]
use sha2::{Digest, Sha256};
use std::{
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const MAX_MESSAGE: usize = 2 * 1024 * 1024;
fn compatibility() -> &'static ort_codex_install::Manifest {
    ort_codex_install::manifest()
}
fn require_memory_auth(config: &Value) -> Result<(), &'static str> {
    match config
        .pointer("/config/cli_auth_credentials_store")
        .and_then(Value::as_str)
    {
        Some("ephemeral") => Ok(()),
        Some(_) => Err("PLAN_AUTH_STORAGE_POLICY"),
        None => Err("PLAN_PROTOCOL_INVALID"),
    }
}
fn require_runtime_policy(config: &Value) -> Result<(), &'static str> {
    require_memory_auth(config)?;
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
        if config
            .pointer(&format!("/config/features/{feature}"))
            .and_then(Value::as_bool)
            != Some(false)
        {
            return Err("PLAN_RUNTIME_POLICY_REJECTED");
        }
    }
    for tool in ["update_plan", "experimental_request_user_input"] {
        if !layered_tool_disabled(config, tool) {
            return Err("PLAN_RUNTIME_POLICY_REJECTED");
        }
    }
    if config.pointer("/config/web_search").and_then(Value::as_str) != Some("disabled")
        || ["mcp_servers", "plugins"].iter().any(|key| {
            config
                .pointer(&format!("/config/{key}"))
                .is_some_and(|value| {
                    !value.is_null() && !value.as_object().is_some_and(serde_json::Map::is_empty)
                })
        })
    {
        return Err("PLAN_RUNTIME_POLICY_REJECTED");
    }
    Ok(())
}
fn layered_tool_disabled(value: &Value, tool: &str) -> bool {
    // Pinned v0.162.0 Config.tools exposes only web_search (ToolsV2).
    // config/read's raw layers retain these tool flags, highest precedence
    // first (ConfigService::read). Inspect only these booleans; never log layers.
    let Some(layers) = value
        .get("layers")
        .and_then(Value::as_array)
        .filter(|layers| layers.len() <= 64)
    else {
        return false;
    };
    for layer in layers {
        if layer.get("disabledReason").is_some_and(|v| !v.is_null()) {
            continue;
        }
        let Some(tools) = layer.pointer("/config/tools") else {
            continue;
        };
        if !tools.is_object() {
            return false;
        }
        let Some(entry) = tools.get(tool) else {
            continue;
        };
        if !entry.is_object() {
            return false;
        }
        if let Some(enabled) = entry.get("enabled") {
            return enabled.as_bool() == Some(false);
        }
    }
    false
}
const CANDIDATES: [&str; 3] = [
    "/Library/Application Support/Open Resume Toolkit/Codex/codex",
    "/Applications/CodexCLI.app/Contents/MacOS/codex",
    "/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
];

pub struct Runtime {
    pub path: PathBuf,
}
pub fn discover() -> Result<Runtime, &'static str> {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return Err("PLAN_PLATFORM_UNSUPPORTED");
    }
    let mut failure = "PLAN_RUNTIME_MISSING";
    for candidate in CANDIDATES {
        let path = Path::new(candidate);
        if !path.exists() {
            continue;
        }
        match verify_runtime(path) {
            Ok(()) => return Ok(Runtime { path: path.into() }),
            Err(code) => failure = code,
        }
    }
    Err(failure)
}
pub(crate) fn verify_runtime(path: &Path) -> Result<(), &'static str> {
    if path.canonicalize().ok().as_deref() != Some(path) {
        return Err("PLAN_RUNTIME_UNTRUSTED");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if rustix::fs::access(path, rustix::fs::Access::EXEC_OK).is_err() {
            return Err("PLAN_RUNTIME_UNTRUSTED");
        }
        for parent in path.ancestors().take_while(|p| *p != Path::new("/")) {
            let metadata =
                std::fs::symlink_metadata(parent).map_err(|_| "PLAN_RUNTIME_UNTRUSTED")?;
            if metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
                return Err("PLAN_RUNTIME_UNTRUSTED");
            }
        }
    }
    if ort_codex_install::verify_payload(path).is_err() {
        return Err("PLAN_RUNTIME_INCOMPATIBLE");
    }
    Ok(())
}
fn quoted(path: &Path) -> Result<String, &'static str> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_owned());
    let path = canonical.as_path();
    let path = path.to_str().ok_or("PLAN_RUNTIME_UNAVAILABLE")?;
    if path.bytes().any(|c| c.is_ascii_control()) {
        return Err("PLAN_RUNTIME_UNAVAILABLE");
    }
    Ok(format!(
        "\"{}\"",
        path.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

#[cfg(test)]
#[path = "codex_login_tests.rs"]
mod login_tests;

fn login_failure_code(value: &Value) -> &'static str {
    let error = value
        .pointer("/params/error")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if [
        "error sending request",
        "connection refused",
        "connection reset",
        "timed out",
        "certificate",
        "dns error",
    ]
    .iter()
    .any(|marker| error.contains(marker))
    {
        "PLAN_LOGIN_TRANSPORT_FAILED"
    } else if error.contains("token exchange") || error.contains("token_exchange_failed") {
        "PLAN_LOGIN_TOKEN_EXCHANGE_FAILED"
    } else if error.contains("credentials could not be saved") || error.contains("persist_failed") {
        "PLAN_LOGIN_STORAGE_FAILED"
    } else if error.contains("workspace restriction") || error.contains("codex is not enabled") {
        "PLAN_LOGIN_RESTRICTED"
    } else if ["declined", "cancel", "access_denied"]
        .iter()
        .any(|marker| error.contains(marker))
    {
        "PLAN_LOGIN_DECLINED"
    } else {
        "PLAN_LOGIN_FAILED"
    }
}
/// Default-deny OS boundary. Process forks and every executable except the
/// verified runtime are denied; user roots and arbitrary egress stay unreadable.
pub(crate) fn sandbox_policy(
    executable: &Path,
    auth: &Path,
    scratch: &Path,
    proxy_port: u16,
) -> Result<String, &'static str> {
    Ok(format!(
        r#"(version 1)
(deny default)
(allow file-read-metadata)
(allow file-read* file-map-executable (literal "/") (subpath "/usr/lib") (subpath "/System") (subpath "/private/preboot") (subpath "/Library/Apple") (literal "/dev/null") (literal "/dev/urandom") (literal "/dev/random") (literal "/private/etc/hosts") (literal "/private/etc/resolv.conf") (subpath "/private/etc/ssl") (literal {exe}))
(allow file-read* file-write* file-lock (subpath {auth}) (subpath {scratch}))
(allow file-read-data (literal {exe_parent}) (literal {scratch_parent}))
(allow process-exec (literal {exe}))
(allow sysctl-read)
(allow mach-lookup (global-name "com.apple.SecurityServer") (global-name "com.apple.trustd") (global-name "com.apple.trustd.agent") (global-name "com.apple.system.notification_center") (global-name "com.apple.logd") (global-name "com.apple.system.opendirectoryd.libinfo") (global-name "com.apple.SystemConfiguration.configd"))
(allow ipc-posix*)
(allow network-outbound (remote tcp "localhost:{proxy_port}"))
(allow network-bind network-inbound (local tcp "localhost:1455"))
"#,
        exe = quoted(executable)?,
        exe_parent = quoted(executable.parent().ok_or("PLAN_RUNTIME_UNAVAILABLE")?)?,
        scratch_parent = quoted(scratch.parent().ok_or("PLAN_RUNTIME_UNAVAILABLE")?)?,
        auth = quoted(auth)?,
        scratch = quoted(scratch)?
    ))
}

pub struct Session {
    child: Child,
    stdin: crate::codex_transport::Writer,
    interrupt: Arc<AtomicBool>,
    messages: mpsc::Receiver<Result<Value, &'static str>>,
    next_id: u64,
    pub models: Vec<PlanModel>,
    pub login_id: Option<String>,
    pub login_started: Option<Instant>,
    pub login_error: Option<&'static str>,
    /// Non-secret activity identity, assigned after this process signs in.
    pub connection_id: Option<uuid::Uuid>,
    pub account_plan: Option<String>,
    pub quota: Option<ort_ai::plan::QuotaSnapshot>,
    scratch: tempfile::TempDir,
    _gateway: Option<crate::codex_egress::Gateway>,
    pub last_usage: Option<ort_ai::Usage>,
    pub reported_retries: u32,
    pub failure_http_status: Option<u16>,
    pub failure_provider_reason: Option<String>,
    selected_model: Option<String>,
    selected_thread: Option<String>,
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Session {
    #[cfg(all(test, unix))]
    pub(crate) fn process_id(&self) -> u32 {
        self.child.id()
    }

    pub(crate) fn attach_interrupt(&mut self, interrupt: Arc<AtomicBool>) {
        self.interrupt = interrupt;
    }
    fn cancelled(&self, cancel: &dyn Fn() -> bool) -> bool {
        cancel() || self.interrupt.load(Ordering::Acquire)
    }
    pub(crate) fn terminate(&mut self) {
        let _ = self.child.kill();
    }
    pub(crate) fn begin_pass(&mut self) {
        self.clear_reports();
        self.selected_model = None;
        self.selected_thread = None;
    }
    fn clear_reports(&mut self) {
        self.last_usage = None;
        self.reported_retries = 0;
        self.failure_http_status = None;
        self.failure_provider_reason = None;
    }
    pub(crate) fn is_running(&mut self) -> bool {
        self.child.try_wait().is_ok_and(|status| status.is_none())
    }
    pub fn start(
        auth_root: &Path,
        end: Instant,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Self, &'static str> {
        if cancel() {
            return Err("AI_CANCELLED");
        }
        let runtime = discover()?;
        Self::start_runtime(&runtime, auth_root, end, cancel)
    }
    pub(crate) fn start_runtime(
        runtime: &Runtime,
        auth_root: &Path,
        end: Instant,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Self, &'static str> {
        std::fs::create_dir_all(auth_root).map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
        if auth_root
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
            || auth_root
                .join("config.toml")
                .symlink_metadata()
                .is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
        {
            return Err("PLAN_RUNTIME_UNTRUSTED");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(auth_root, std::fs::Permissions::from_mode(0o700))
                .map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
        }
        let canonical_auth = auth_root
            .canonicalize()
            .map_err(|_| "PLAN_RUNTIME_UNTRUSTED")?;
        let auth_root = canonical_auth.as_path();
        let scratch = tempfile::tempdir().map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
        // Never inherit the user's Codex configuration or API credentials.
        // Ephemeral auth has no persistent fallback, including after restart.
        // Use the explicit proxy environment below. Codex's system-proxy mode
        // can select macOS's DIRECT route ahead of that environment; the OS
        // sandbox correctly denies that route instead of reaching our gateway.
        std::fs::write(
            auth_root.join("config.toml"),
            include_str!("codex-runtime.toml"),
        )
        .map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
        let existing = std::fs::read_to_string(auth_root.join("config.toml"))
            .map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
        std::fs::write(
            auth_root.join("config.toml"),
            format!(
                "log_dir = {}\n{}",
                serde_json::to_string(&scratch.path().join("logs"))
                    .map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?,
                existing
            ),
        )
        .map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
        let gateway = crate::codex_egress::Gateway::start()?;
        let policy = sandbox_policy(&runtime.path, auth_root, scratch.path(), gateway.port)?;
        let proxy = format!("http://127.0.0.1:{}", gateway.port);
        let child = Command::new("/usr/bin/sandbox-exec")
            .args(["-p", &policy])
            .arg(&runtime.path)
            .args(["app-server", "--listen", "stdio://"])
            .env_clear()
            .env("CODEX_HOME", auth_root)
            .env("PATH", "/usr/bin:/bin")
            .env("RUST_LOG", "off")
            .env("HTTPS_PROXY", &proxy)
            .env("https_proxy", &proxy)
            .env("HTTP_PROXY", &proxy)
            .env("http_proxy", &proxy)
            .env("NO_PROXY", "127.0.0.1,localhost")
            .env("TMPDIR", scratch.path())
            .current_dir(scratch.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(
                if cfg!(test) && std::env::var_os("ORT_CODEX_QUALIFY_PATH").is_some() {
                    Stdio::inherit()
                } else {
                    Stdio::null()
                },
            )
            .spawn()
            .map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
        Self::from_child(child, scratch, Some(gateway), end, cancel)
    }
    fn from_child(
        mut child: Child,
        scratch: tempfile::TempDir,
        gateway: Option<crate::codex_egress::Gateway>,
        end: Instant,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Self, &'static str> {
        let stdin = child.stdin.take().ok_or("PLAN_RUNTIME_UNAVAILABLE")?;
        let stdin = match crate::codex_transport::Writer::new(stdin) {
            Ok(writer) => writer,
            Err(code) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(code);
            }
        };
        let stdout = child.stdout.take().ok_or("PLAN_RUNTIME_UNAVAILABLE")?;
        let (send, messages) = mpsc::sync_channel(32);
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = Vec::new();
                let result = (&mut reader)
                    .take((MAX_MESSAGE + 1) as u64)
                    .read_until(b'\n', &mut line);
                let value = match result {
                    Ok(0) | Err(_) => Err("PLAN_RUNTIME_UNAVAILABLE"),
                    Ok(_) if line.len() > MAX_MESSAGE => Err("PLAN_PROTOCOL_INVALID"),
                    Ok(_) => serde_json::from_slice(&line).map_err(|_| "PLAN_PROTOCOL_INVALID"),
                };
                let terminal = value.is_err();
                if send.send(value).is_err() || terminal {
                    break;
                }
            }
        });
        let mut session = Self {
            child,
            stdin,
            interrupt: Arc::default(),
            messages,
            next_id: 0,
            models: Vec::new(),
            login_id: None,
            login_started: None,
            login_error: None,
            connection_id: None,
            account_plan: None,
            quota: None,
            scratch,
            _gateway: gateway,
            last_usage: None,
            reported_retries: 0,
            failure_http_status: None,
            failure_provider_reason: None,
            selected_model: None,
            selected_thread: None,
        };
        let startup_end = end.min(Instant::now() + Duration::from_secs(30));
        session.rpc("initialize",json!({"clientInfo":{"name":"open_resume_toolkit","title":"Open Resume Toolkit","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":false}}),Duration::from_secs(10).min(startup_end.saturating_duration_since(Instant::now())),cancel)?;
        session.send(json!({"method":"initialized"}), startup_end, cancel)?;
        // Machine-managed settings can override config.toml. Refuse any
        // effective persistent store before allowing account commands.
        let config = session.rpc(
            "config/read",
            json!({"includeLayers":true}),
            Duration::from_secs(10).min(startup_end.saturating_duration_since(Instant::now())),
            cancel,
        )?;
        require_runtime_policy(&config)?;
        let mut catalog = Vec::new();
        let mut cursor = None;
        let mut seen = std::collections::HashSet::new();
        for _ in 0..10 {
            let page = session.rpc(
                "model/list",
                json!({"limit":100,"includeHidden":false,"cursor":cursor}),
                Duration::from_secs(10).min(startup_end.saturating_duration_since(Instant::now())),
                cancel,
            )?;
            catalog.extend(
                page.get("data")
                    .and_then(Value::as_array)
                    .ok_or("PLAN_PROTOCOL_INVALID")?
                    .iter()
                    .cloned(),
            );
            cursor = page
                .get("nextCursor")
                .filter(|v| !v.is_null())
                .and_then(Value::as_str)
                .map(str::to_owned);
            if cursor.is_none() {
                session.models = models_from_catalog(&json!({"data":catalog}));
                return Ok(session);
            }
            if !seen.insert(cursor.clone()) {
                return Err("PLAN_PROTOCOL_INVALID");
            }
        }
        Err("PLAN_PROTOCOL_INVALID")
    }
    #[allow(clippy::needless_pass_by_value)]
    fn send(
        &mut self,
        value: Value,
        end: Instant,
        cancel: &dyn Fn() -> bool,
    ) -> Result<(), &'static str> {
        let mut bytes = serde_json::to_vec(&value).map_err(|_| "PLAN_PROTOCOL_INVALID")?;
        if bytes.len() > MAX_MESSAGE {
            return Err("AI_INPUT_TOO_LARGE");
        }
        bytes.push(b'\n');
        let interrupt = &self.interrupt;
        let result = self.stdin.send(&bytes, end, &|| {
            cancel() || interrupt.load(Ordering::Acquire)
        });
        if result.is_err() {
            // A partial frame cannot be safely reused, regardless of its cause.
            let _ = self.child.kill();
        }
        result
    }
    fn receive(&mut self, end: Instant, cancel: &dyn Fn() -> bool) -> Result<Value, &'static str> {
        loop {
            if self.cancelled(cancel) {
                let _ = self.child.kill();
                return Err("AI_CANCELLED");
            }
            if Instant::now() >= end {
                let _ = self.child.kill();
                return Err("PLAN_REQUEST_TIMEOUT");
            }
            match self.messages.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(value)) => return Ok(value),
                Ok(Err(code)) => return Err(code),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => return Err("PLAN_RUNTIME_UNAVAILABLE"),
            }
        }
    }
    #[allow(clippy::needless_pass_by_value)]
    pub fn rpc(
        &mut self,
        method: &str,
        params: Value,
        deadline: Duration,
        cancel: &dyn Fn() -> bool,
    ) -> Result<Value, &'static str> {
        if !matches!(
            method,
            "initialize"
                | "config/read"
                | "model/list"
                | "account/read"
                | "account/login/start"
                | "account/login/cancel"
                | "account/logout"
                | "account/rateLimits/read"
                | "thread/start"
                | "thread/unsubscribe"
                | "turn/start"
                | "turn/interrupt"
        ) {
            return Err("PLAN_CONTAINMENT_VIOLATION");
        }
        if method == "turn/start" {
            self.clear_reports();
        }
        if method == "thread/start" {
            // Rejections occur before the response. Attribute them to the new
            // requested model, never the model from a previous operation.
            self.selected_model = params
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_owned);
        }
        self.next_id += 1;
        let id = self.next_id;
        let end = Instant::now() + deadline;
        self.send(
            json!({"id":id,"method":method,"params":params.clone()}),
            end,
            cancel,
        )?;
        loop {
            let value = self.receive(end, cancel)?;
            if value.get("method").is_some() {
                self.notification(&value)?;
                continue;
            }
            if value.get("id").and_then(Value::as_u64) != Some(id) {
                return Err("PLAN_PROTOCOL_INVALID");
            }
            if let Some(error) = value.get("error") {
                return Err(self.provider_failure(error));
            }
            let result = value
                .get("result")
                .cloned()
                .ok_or("PLAN_PROTOCOL_INVALID")?;
            if method == "thread/start" {
                self.selected_thread = result
                    .pointer("/thread/id")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
            }
            return Ok(result);
        }
    }
    fn provider_failure(&mut self, error: &Value) -> &'static str {
        // Keep only qualified enum identifiers and bounded HTTP status, never
        // provider messages (which can echo resume content or authorization).
        let info_value = error
            .get("codexErrorInfo")
            .or_else(|| error.pointer("/data/codexErrorInfo"));
        let info = info_value
            .and_then(|v| {
                v.as_str().or_else(|| {
                    v.as_object()
                        .filter(|o| o.len() == 1)
                        .and_then(|o| o.keys().next().map(String::as_str))
                })
            })
            .unwrap_or("");
        if matches!(
            info,
            "contextWindowExceeded"
                | "usageLimitExceeded"
                | "httpConnectionFailed"
                | "responseStreamConnectionFailed"
                | "responseStreamDisconnected"
                | "responseTooManyFailedAttempts"
                | "serverOverloaded"
                | "internalServerError"
                | "unauthorized"
                | "badRequest"
                | "threadRollbackFailed"
                | "sandboxError"
                | "other"
                | "cyberPolicy"
                | "misalignmentPolicyViolation"
        ) {
            self.failure_provider_reason = Some(info.into());
        }
        self.failure_http_status = info_value
            .and_then(|v| v.get(info))
            .and_then(|v| v.get("httpStatusCode"))
            .and_then(Value::as_u64)
            .filter(|s| (100..=599).contains(s))
            .and_then(|s| u16::try_from(s).ok());
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .filter(|s| s.len() <= 8192)
            .unwrap_or("")
            .to_ascii_lowercase();
        if message.contains("model")
            && [
                "not supported",
                "does not exist",
                "not available",
                "not found",
                "do not have access",
                "no access",
            ]
            .iter()
            .any(|s| message.contains(s))
        {
            if let Some(model) = self
                .models
                .iter_mut()
                .find(|m| Some(&m.id) == self.selected_model.as_ref())
            {
                model.supported = false;
                model.explanation = Some("Codex rejected access to this model for the connected account. Reconnect after account access changes.".into());
            }
            return "PLAN_MODEL_UNAVAILABLE";
        }
        if let Some(status) = self.failure_http_status {
            match status {
                401 => return "PLAN_AUTH_REQUIRED",
                429 => return "AI_RATE_LIMITED",
                500..=599 => return "AI_PROVIDER_TEMPORARY",
                _ => {}
            }
        }
        match info {
            "unauthorized" => "PLAN_AUTH_REQUIRED",
            "usageLimitExceeded" | "rateLimitExceeded" => "AI_RATE_LIMITED",
            "contextWindowExceeded" => "AI_INPUT_TOO_LARGE",
            "cyberPolicy" | "misalignmentPolicyViolation" => "AI_OUTPUT_BLOCKED",
            "serverOverloaded" | "internalServerError" => "AI_PROVIDER_TEMPORARY",
            _ => "PLAN_PROVIDER_REJECTED",
        }
    }
    fn notification(&mut self, value: &Value) -> Result<(), &'static str> {
        if let Err(code) = notification_policy(value) {
            self.failure_provider_reason = Some(
                if code == "PLAN_CONTAINMENT_VIOLATION" {
                    "prohibitedToolOrPermissionAction"
                } else {
                    "unsupportedProtocolNotification"
                }
                .into(),
            );
            #[cfg(test)]
            if std::env::var_os("ORT_CODEX_QUALIFY_PATH").is_some() {
                eprintln!(
                    "qualification: rejected method {}",
                    value
                        .get("method")
                        .and_then(Value::as_str)
                        .unwrap_or("missing")
                );
            }
            let _ = self.child.kill();
            return Err(code);
        }
        if value.get("method").and_then(Value::as_str) == Some("thread/tokenUsage/updated") {
            let reported_thread = value.pointer("/params/threadId").and_then(Value::as_str);
            if self
                .selected_thread
                .as_deref()
                .is_some_and(|id| Some(id) != reported_thread)
            {
                return Err("PLAN_PROTOCOL_INVALID");
            }
            if let Some(usage) = value.get("params").and_then(ort_ai::plan::turn_usage) {
                self.last_usage = Some(usage);
            }
        }
        if value.get("method").and_then(Value::as_str) == Some("error")
            && value.pointer("/params/willRetry").and_then(Value::as_bool) == Some(true)
        {
            self.reported_retries = self.reported_retries.saturating_add(1);
        }
        if value.get("method").and_then(Value::as_str) == Some("account/login/completed")
            && value.pointer("/params/loginId").and_then(Value::as_str) == self.login_id.as_deref()
        {
            self.login_id = None;
            self.login_started = None;
            self.login_error = (value.pointer("/params/success").and_then(Value::as_bool)
                != Some(true))
            .then(|| login_failure_code(value));
        }
        Ok(())
    }
    pub fn poll_login(&mut self) -> Result<(), &'static str> {
        while let Ok(value) = self.messages.try_recv() {
            self.notification(&value?)?;
        }
        if self
            .login_started
            .is_some_and(|time| time.elapsed() > Duration::from_secs(300))
        {
            self.cancel_login()?;
            self.login_error = Some("PLAN_LOGIN_TIMEOUT");
        }
        Ok(())
    }
    pub fn cancel_login(&mut self) -> Result<(), &'static str> {
        if let Some(id) = self.login_id.take() {
            self.rpc(
                "account/login/cancel",
                json!({"loginId":id}),
                Duration::from_secs(10),
                &|| false,
            )?;
        }
        self.login_started = None;
        Ok(())
    }
    pub fn scratch_path(&self) -> &Path {
        self.scratch.path()
    }
    pub fn release_thread(
        &mut self,
        thread_id: &str,
        end: Instant,
        cancel: &dyn Fn() -> bool,
    ) -> Result<(), &'static str> {
        let result = self.rpc(
            "thread/unsubscribe",
            json!({"threadId":thread_id}),
            Duration::from_secs(2).min(end.saturating_duration_since(Instant::now())),
            cancel,
        );
        if !result.as_ref().is_ok_and(|value| {
            matches!(
                value.get("status").and_then(Value::as_str),
                Some("unsubscribed" | "notLoaded" | "notSubscribed")
            )
        }) {
            let _ = self.child.kill();
            self.failure_provider_reason = Some("ephemeralThreadCleanupFailed".into());
            return Err(if self.cancelled(cancel) {
                "AI_CANCELLED"
            } else {
                "PLAN_RUNTIME_UNAVAILABLE"
            });
        }
        self.selected_thread = None;
        Ok(())
    }
    pub fn run_turn(
        &mut self,
        thread_id: &str,
        turn_id: &str,
        end: Instant,
        cancel: &dyn Fn() -> bool,
    ) -> Result<String, &'static str> {
        let mut output = None;

        (|| {
            loop {
                let value = self.receive(end, cancel)?;
                self.notification(&value)?;
                let params = value.get("params").ok_or("PLAN_PROTOCOL_INVALID")?;
                let method = value
                    .get("method")
                    .and_then(Value::as_str)
                    .ok_or("PLAN_PROTOCOL_INVALID")?;
                // Lifecycle metadata can arrive after a previous ephemeral
                // thread was released. It carries no result or token accounting.
                if matches!(
                    method,
                    "thread/closed" | "thread/status/changed" | "thread/name/updated"
                ) {
                    continue;
                }
                if params
                    .get("threadId")
                    .and_then(Value::as_str)
                    .is_some_and(|id| id != thread_id)
                {
                    return Err("PLAN_PROTOCOL_INVALID");
                }
                if params
                    .get("turnId")
                    .and_then(Value::as_str)
                    .is_some_and(|id| id != turn_id)
                {
                    return Err("PLAN_PROTOCOL_INVALID");
                }
                match method {
                    "item/completed" => {
                        let item = params.get("item").ok_or("PLAN_PROTOCOL_INVALID")?;
                        if item.get("type").and_then(Value::as_str) == Some("agentMessage")
                            && matches!(
                                item.get("phase").and_then(Value::as_str),
                                None | Some("final_answer")
                            )
                        {
                            output = item.get("text").and_then(Value::as_str).map(str::to_owned);
                        }
                    }
                    "error" => {
                        if params.get("willRetry").and_then(Value::as_bool) == Some(true) {
                            // Retry telemetry was captured by notification().
                        } else {
                            return Err(
                                self.provider_failure(params.get("error").unwrap_or(&Value::Null))
                            );
                        }
                    }
                    "turn/completed" => {
                        if params.pointer("/turn/id").and_then(Value::as_str) != Some(turn_id) {
                            return Err("PLAN_PROTOCOL_INVALID");
                        }
                        if params.pointer("/turn/status").and_then(Value::as_str)
                            != Some("completed")
                        {
                            return Err(self.provider_failure(
                                params.pointer("/turn/error").unwrap_or(&Value::Null),
                            ));
                        }
                        return output
                            .filter(|s| !s.trim().is_empty())
                            .ok_or("AI_OUTPUT_INVALID");
                    }
                    _ => {}
                }
            }
        })()
    }
}
fn allowed_notification(value: &Value) -> bool {
    let method = value.get("method").and_then(Value::as_str).unwrap_or("");
    // Qualified runtime broadcasts its disabled remote-control status at startup.
    // Any attempt to connect or expose a remote environment violates containment.
    if method == "remoteControl/status/changed" {
        return value.pointer("/params/status").and_then(Value::as_str) == Some("disabled")
            && value
                .pointer("/params/environmentId")
                .is_none_or(Value::is_null);
    }
    if matches!(method, "item/started" | "item/completed")
        && !matches!(
            value.pointer("/params/item/type").and_then(Value::as_str),
            Some("agentMessage" | "userMessage" | "reasoning")
        )
    {
        return false;
    }
    matches!(
        method,
        "account/login/completed"
            | "account/updated"
            | "account/rateLimits/updated"
            | "thread/started"
            | "thread/status/changed"
            | "turn/started"
            | "turn/completed"
            | "thread/tokenUsage/updated"
            | "item/started"
            | "item/completed"
            | "item/agentMessage/delta"
            | "item/reasoning/textDelta"
            | "item/reasoning/summaryTextDelta"
            | "item/reasoning/summaryPartAdded"
            | "model/verification"
            | "model/safetyBuffering/updated"
            | "modelProvider/authRecoveryStarted"
            | "modelProvider/authRecoveryCompleted"
            | "warning"
            | "configWarning"
            | "deprecationNotice"
            | "thread/name/updated"
            | "thread/closed"
            | "error"
    )
}
fn notification_policy(value: &Value) -> Result<(), &'static str> {
    let method = value.get("method").and_then(Value::as_str).unwrap_or("");
    // Passive status messages are not evidence of filesystem access. Only
    // qualified notifications are accepted; unknown protocol still fails closed.
    if value.get("id").is_some()
        || method.starts_with("item/commandExecution/")
        || method.starts_with("item/fileChange/")
        || method.starts_with("item/mcpToolCall/")
        || matches!(
            method,
            "turn/plan/updated" | "turn/diff/updated" | "item/tool/call" | "item/plan/delta"
        )
        || (matches!(method, "item/started" | "item/completed")
            && matches!(
                value.pointer("/params/item/type").and_then(Value::as_str),
                Some(
                    "commandExecution"
                        | "fileChange"
                        | "mcpToolCall"
                        | "dynamicToolCall"
                        | "webSearch"
                        | "imageGeneration"
                        | "collabAgentToolCall"
                        | "plan"
                )
            ))
        || (method == "remoteControl/status/changed" && !allowed_notification(value))
    {
        return Err("PLAN_CONTAINMENT_VIOLATION");
    }
    if !allowed_notification(value) {
        return Err("PLAN_PROTOCOL_INVALID");
    }
    Ok(())
}
pub fn qualified_version() -> &'static str {
    &compatibility().version
}

#[cfg(test)]
pub(crate) fn test_session(mode: &str) -> Session {
    let child = Command::new("/usr/bin/python3")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codex-app-server.py"))
        .arg(mode)
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
}

#[cfg(all(test, unix))]
pub(crate) fn test_process_is_running(pid: u32) -> bool {
    Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("inspect synthetic server process")
        .success()
}

#[cfg(test)]
#[path = "codex_runtime_tests.rs"]
mod tests;
