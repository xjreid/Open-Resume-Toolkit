import { useEffect, useId, useState } from "react";
import type { HealthResponse } from "@ort/contracts/health";
import { requestHealth } from "./command-client";

type Surface = "main" | "overlay";
type HealthState =
  | { kind: "checking" }
  | { kind: "ready"; health: HealthResponse }
  | { kind: "error"; message: string };

export function App({ surface }: { surface: Surface }) {
  const [health, setHealth] = useState<HealthState>({ kind: "checking" });
  const statusId = useId();

  async function checkHealth() {
    setHealth({ kind: "checking" });
    const result = await requestHealth();
    if (result.ok) {
      setHealth({ kind: "ready", health: result.value });
    } else {
      setHealth({ kind: "error", message: result.error.messageKey });
    }
  }

  useEffect(() => {
    void checkHealth();
  }, []);

  const isOverlay = surface === "overlay";

  return (
    <main className={isOverlay ? "shell shell--overlay" : "shell"}>
      <header className="masthead">
        <div className="mark" aria-hidden="true">
          ORT
        </div>
        <div>
          <p className="eyebrow">Development profile</p>
          <h1>{isOverlay ? "Application workspace" : "Open Resume Toolkit"}</h1>
        </div>
      </header>

      <section className="status-card" aria-labelledby={statusId}>
        <div>
          <p className="eyebrow">M1 encrypted-core check</p>
          <h2 id={statusId}>Desktop command boundary</h2>
        </div>
        <HealthBadge state={health} />
        <p className="description">
          The encrypted storage core has passed local synthetic tests. Runtime
          persistence remains gated until native vault and cross-platform tests
          pass; import, AI, browser connection, and updates remain unavailable.
        </p>
        {health.kind === "ready" ? (
          <dl className="facts">
            <div>
              <dt>Application version</dt>
              <dd>{health.health.appVersion}</dd>
            </div>
            <div>
              <dt>Profile</dt>
              <dd>{health.health.profile}</dd>
            </div>
            <div>
              <dt>Contract</dt>
              <dd>v{health.health.contractVersion}</dd>
            </div>
            <div>
              <dt>Storage</dt>
              <dd>{health.health.storageStatus}</dd>
            </div>
          </dl>
        ) : null}
        <button
          type="button"
          onClick={() => void checkHealth()}
          disabled={health.kind === "checking"}
        >
          {health.kind === "checking" ? "Checking…" : "Check again"}
        </button>
      </section>
    </main>
  );
}

function HealthBadge({ state }: { state: HealthState }) {
  if (state.kind === "checking") {
    return (
      <p className="badge badge--pending" role="status">
        Checking
      </p>
    );
  }

  if (state.kind === "error") {
    return (
      <p className="badge badge--error" role="alert" title={state.message}>
        Unavailable
      </p>
    );
  }

  return (
    <p className="badge badge--ready" role="status">
      Healthy
    </p>
  );
}
