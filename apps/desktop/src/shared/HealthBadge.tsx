import type { HealthResponse } from "@ort/contracts/health";
export type HealthState =
  | { kind: "checking" }
  | { kind: "ready"; health: HealthResponse }
  | { kind: "error"; message: string };

export function HealthBadge({
  state,
  onRetry,
}: {
  state: HealthState;
  onRetry: () => void;
}) {
  if (state.kind === "checking") {
    return (
      <p className="badge badge--pending" role="status">
        Checking
      </p>
    );
  }
  if (state.kind === "error") {
    return (
      <button
        type="button"
        className="badge badge--error"
        onClick={onRetry}
        title="Retry encrypted storage"
      >
        Storage unavailable · Retry
      </button>
    );
  }
  const ready = state.health.storageStatus === "ready";
  return ready ? (
    <p
      className={`badge ${ready ? "badge--ready" : "badge--error"}`}
      role="status"
    >
      {ready ? "Encrypted storage ready" : "Storage unavailable"}
    </p>
  ) : (
    <button
      type="button"
      className="badge badge--error"
      onClick={onRetry}
      title="Retry encrypted storage"
    >
      Storage unavailable · Retry
    </button>
  );
}
