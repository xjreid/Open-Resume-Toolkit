import type { RuntimeReadiness } from "@ort/contracts/wire";
import { invokeDesktop as invoke } from "./desktop-client";
import { planErrorMessage } from "./chatgpt-plan-presentation";
import { CodexRuntimeInstaller } from "./CodexRuntimeInstaller";

export function CodexRuntimeSetup({
  visible,
  blocked,
  readiness,
  checking,
  error,
  onCheck,
  onInstalled,
  onBusy,
}: {
  visible: boolean;
  blocked: boolean;
  readiness: RuntimeReadiness | null;
  checking: boolean;
  error: string;
  onCheck: () => void;
  onInstalled: () => void;
  onBusy: (busy: boolean) => void;
}) {
  const needed = readiness?.ready === false;
  const supported = readiness?.errorCode !== "PLAN_PLATFORM_UNSUPPORTED";
  return (
    <>
      {!readiness && checking && (
        <p className="ai-help" role="status">
          Checking Codex installation…
        </p>
      )}
      {(error || needed) && (
        <div className="plan-runtime">
          <p className="ai-help" role="alert">
            {error || planErrorMessage(readiness!.errorCode!)}
          </p>
          <button
            type="button"
            className="button--quiet"
            disabled={blocked || checking}
            onClick={onCheck}
          >
            Check installation
          </button>
        </div>
      )}
      <CodexRuntimeInstaller
        visible={visible}
        blocked={blocked || checking}
        needed={needed && supported}
        onBusy={onBusy}
        onComplete={onInstalled}
      />
      {needed && supported && (
        <details className="plan-installation">
          <summary>Runtime installation details</summary>
          <p className="ai-help">
            The optional installer downloads Codex 0.162.0 for macOS Apple
            Silicon and verifies its archive and executable checksums. macOS
            asks an administrator to place the unmodified executable here with
            root ownership and protected folders:
          </p>
          <code>
            /Library/Application Support/Open Resume Toolkit/Codex/codex
          </code>
          <p className="ai-help">
            User-writable Homebrew, download, and temporary locations cannot be
            used. ORT checks the supported release and permissions without
            starting Codex, and verifies it again before startup. Installation
            never runs Codex as administrator or changes another installation.
          </p>
          <button
            type="button"
            className="button--secondary"
            onClick={() => void invoke("open_chatgpt_plan_runtime_guidance")}
          >
            Open official runtime download
          </button>
        </details>
      )}
    </>
  );
}
