import type * as Wire from "@ort/contracts/wire";

export const planErrorMessage = (code: string) =>
  (
    ({
      PLAN_DISABLED: "Enable Codex before connecting your ChatGPT account.",
      PLAN_RUNTIME_MISSING:
        "Install the optional Codex runtime below to connect your ChatGPT account.",
      PLAN_RUNTIME_CHECK_FAILED:
        "Codex installation could not be checked. Choose Check installation to try again.",
      PLAN_INSTALL_DOWNLOAD_FAILED:
        "The official runtime download failed, timed out, or had an unexpected size. Check your connection and retry. Nothing was installed.",
      PLAN_INSTALL_VERIFY_FAILED:
        "The downloaded runtime failed its size, checksum, or executable checks. Installation stopped. Retry the official download.",
      PLAN_INSTALL_ARCHIVE_INVALID:
        "The runtime archive did not match the approved single-file layout. Installation stopped before any protected files were changed.",
      PLAN_INSTALL_UNSAFE_PATH:
        "A runtime path is a link, has unsafe permissions, or is not a regular file. Installation stopped. An administrator must check the protected runtime directory before you retry.",
      PLAN_INSTALL_IO_FAILED:
        "The runtime could not be read or written. Check free disk space and directory access, then refresh connection before retrying.",
      PLAN_INSTALL_HELPER_MISSING:
        "This development build is missing its verified installer helper. Reinstall the complete development app, then retry.",
      PLAN_INSTALL_PERMISSION_DENIED:
        "macOS did not authorize runtime installation. Retry and approve the administrator prompt.",
      PLAN_INSTALL_APPROVAL_TIMEOUT:
        "The macOS installation prompt timed out. Refresh connection to check the runtime before retrying.",
      PLAN_INSTALL_APPROVAL_PENDING:
        "Finish or cancel the macOS administrator prompt. The protected installation cannot be cancelled from this page.",
      PLAN_INSTALL_CANCELLED:
        "Runtime installation was cancelled. You can try again when ready.",
      PLAN_LOGIN_PENDING:
        "Finish or cancel browser sign-in before installing the runtime.",
      PLAN_RUNTIME_UNTRUSTED:
        "The installed Codex runtime could not be verified. Use the official release in the protected runtime location described below, then refresh.",
      PLAN_RUNTIME_INCOMPATIBLE:
        "This Codex version has not been qualified for ORT. Use the supported runtime shown below; ORT will never substitute another runtime or model.",
      PLAN_RUNTIME_UNAVAILABLE:
        "Codex stopped or could not start. Check its installation and refresh. API keys are unavailable while Codex is enabled.",
      PLAN_PLATFORM_UNSUPPORTED:
        "Codex connections currently support macOS on Apple Silicon.",
      PLAN_LOGIN_DECLINED:
        "Sign-in was declined or cancelled. Connect again when ready.",
      PLAN_LOGIN_TRANSPORT_FAILED:
        "Codex could not reach OpenAI’s authentication service through its restricted connection. Check your network, VPN or proxy, then connect again. No account was connected.",
      PLAN_LOGIN_TOKEN_EXCHANGE_FAILED:
        "OpenAI did not complete the sign-in token exchange. Connect again for a fresh browser sign-in; an expired or already-used link cannot be reused.",
      PLAN_LOGIN_STORAGE_FAILED:
        "Codex could not retain authorization for this memory-only session. Refresh the connection and sign in again. No credential file or Keychain fallback is used.",
      PLAN_LOGIN_RESTRICTED:
        "Codex sign-in is restricted for this workspace. Check your account’s Codex access with the workspace administrator, or connect an eligible account.",
      PLAN_LOGIN_FAILED:
        "Codex could not complete sign-in. Connect again for a fresh browser login and check the browser’s error details if it fails again.",
      PLAN_LOGIN_TIMEOUT:
        "Sign-in timed out after five minutes. Connect again to start a new browser login.",
      PLAN_LOGIN_BROWSER_FAILED:
        "The sign-in browser could not open. Check your default browser and connect again.",
      PLAN_AUTH_REQUIRED:
        "Your memory-only ChatGPT session has ended or authorization is unavailable. Sign in again to use Codex. Disable Codex to use API keys.",
      PLAN_AUTH_STORAGE_POLICY:
        "This Mac’s Codex configuration requires persistent credential storage. Memory-only sign-in cannot start. Ask your administrator to allow ephemeral credential storage; ORT will not fall back to a credential file or Keychain.",
      PLAN_RUNTIME_POLICY_REJECTED:
        "Codex’s effective configuration does not meet ORT’s restricted tool and network policy. The runtime was stopped before sign-in or AI work. Check managed Codex settings with your administrator, then refresh.",
      PLAN_QUOTA_UNAVAILABLE:
        "Codex did not report usable quota. Refresh usage or try later. The enabled reserve blocks new passes until quota can be checked.",
      PLAN_QUOTA_INVALID:
        "Codex returned invalid quota data. Refresh usage or update to a qualified runtime. No pass was dispatched.",
      PLAN_RESERVE_REJECTED:
        "Account-wide remaining usage is below your reserve. Wait for a reset or change the reserve on this page.",
      PLAN_MODEL_UNAVAILABLE:
        "The selected model or reasoning level is unavailable. Choose a supported combination; ORT does not substitute models.",
      PLAN_PROVIDER_REJECTED:
        "Codex rejected the request. Check account access and the selected model, then try again.",
      PLAN_REQUEST_TIMEOUT:
        "Codex exceeded the request deadline. The operation stopped; reported tokens remain in Data.",
      PLAN_PROTOCOL_INVALID:
        "Codex returned an unsupported protocol message. ORT stopped the session safely. This does not establish that Codex accessed files outside its sandbox. Reconnect and try again.",
      PLAN_CONTAINMENT_VIOLATION:
        "Codex emitted a prohibited tool or permission action. ORT stopped the sandboxed process and saved no generated material. Because sign-in is memory-only, sign in again to try again.",
      PLAN_CREDENTIAL_CLEANUP_REQUIRED:
        "The Codex runtime has stopped, but its session folder could not be cleared. Retry sign-out before signing in again.",
      PLAN_SETTINGS_INVALID:
        "Choose a supported model and a whole-number reserve from 1 to 100.",
      REVISION_CONFLICT:
        "These settings changed elsewhere. Refresh the page and try again.",
      AI_BUSY:
        "An AI operation is in progress. Stop it or wait for it to finish before changing these settings.",
    }) as Record<string, string>
  )[code] ??
  "The Codex connection could not complete this action. Refresh and try again.";

export const REASONING_EFFORT_LABELS: Record<Wire.ReasoningEffort, string> = {
  low: "Light",
  medium: "Medium",
  high: "High",
  xhigh: "Extra high",
};

export function quotaWindowLabel(window: Wire.QuotaWindow): string {
  const minutes = window.windowDurationMinutes;
  return minutes
    ? minutes % 1_440 === 0
      ? `${minutes / 1_440} days`
      : minutes >= 60
        ? `${minutes / 60} hours`
        : `${minutes} minutes`
    : window.name;
}
