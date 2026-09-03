import { useCallback, useEffect, useState } from "react";
import type { StorageUsage } from "@ort/contracts/storage";
import { requestStorageUsage } from "./command-client";

type UsageState =
  | { kind: "loading" }
  | { kind: "ready"; usage: StorageUsage }
  | { kind: "error" };

export function StoragePanel({ enabled }: { enabled: boolean }) {
  const [state, setState] = useState<UsageState>({ kind: "loading" });

  const refresh = useCallback(async (isCurrent: () => boolean = () => true) => {
    setState({ kind: "loading" });
    const result = await requestStorageUsage();
    if (!isCurrent()) return;
    setState(
      result.ok ? { kind: "ready", usage: result.value } : { kind: "error" },
    );
  }, []);

  useEffect(() => {
    if (!enabled) return;
    let current = true;
    void refresh(() => current);
    return () => {
      current = false;
    };
  }, [enabled, refresh]);

  const usage = state.kind === "ready" ? state.usage : null;
  return (
    <section
      className="editor-panel storage-panel"
      aria-labelledby="storage-heading"
    >
      <div className="section-heading">
        <div>
          <p className="eyebrow">Local data</p>
          <h2 id="storage-heading">Encrypted profile storage</h2>
        </div>
        <button
          type="button"
          className="button--secondary button--compact"
          disabled={!enabled || state.kind === "loading"}
          onClick={() => void refresh()}
        >
          Refresh usage
        </button>
      </div>
      <p className="description">
        This content-free inventory counts the active SQLCipher profile and its
        known journal/manifest files. External exports and backups, OS-vault
        items, and in-memory PDF preview bytes are not included.
      </p>
      {state.kind === "loading" ? (
        <p role="status">Reading encrypted profile usage…</p>
      ) : null}
      {state.kind === "error" ? (
        <p className="notice" role="alert">
          Storage usage is unavailable. The profile was not changed.
        </p>
      ) : null}
      {usage ? (
        <>
          <p className="storage-total" role="status">
            <strong>{formatBytes(usage.totalProfileBytes)}</strong> in the
            active profile · database schema {usage.databaseSchema}
          </p>
          <div className="storage-columns">
            <StorageList
              heading="Saved records"
              values={[
                ["Master drafts", usage.drafts],
                ["Published snapshots", usage.publishedSnapshots],
                ["Portable settings", usage.settings],
                ["PDF render manifests", usage.renderManifests],
                ["Diagnostic events", usage.diagnosticEvents],
              ]}
            />
            <StorageList
              heading="Known local files"
              values={[
                ["Encrypted database", formatBytes(usage.databaseBytes)],
                ["Encrypted write-ahead log", formatBytes(usage.walBytes)],
                ["SQLite shared memory", formatBytes(usage.sharedMemoryBytes)],
                [
                  "Non-secret profile manifest",
                  formatBytes(usage.manifestBytes),
                ],
                ["Recovery metadata", formatBytes(usage.recoveryMetadataBytes)],
              ]}
            />
          </div>
          <p className="description storage-guidance">
            File totals can change after saves and SQLite maintenance.
            Diagnostic events are local troubleshooting metadata and are
            excluded from the current portable backup format.
          </p>
        </>
      ) : null}
    </section>
  );
}

function StorageList({
  heading,
  values,
}: {
  heading: string;
  values: ReadonlyArray<readonly [string, number | string]>;
}) {
  return (
    <section aria-label={heading}>
      <h3>{heading}</h3>
      <dl className="storage-list">
        {values.map(([label, value]) => (
          <div key={label}>
            <dt>{label}</dt>
            <dd>{value}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

export function formatBytes(bytes: number): string {
  if (bytes < 1_024) return `${bytes} ${bytes === 1 ? "byte" : "bytes"}`;
  const units = ["KiB", "MiB", "GiB", "TiB"];
  let value = bytes;
  let unit = -1;
  do {
    value /= 1_024;
    unit += 1;
  } while (value >= 1_024 && unit < units.length - 1);
  const precision = value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(precision)} ${units[unit]} (${bytes} bytes)`;
}
