import { useState, type ReactNode } from "react";

// Settings sections accept feature-owned panels. Keep them mounted when changing
// sections so an in-progress operation or confirmation cannot be reset by navigation.
export function SettingsWorkspace({
  backup,
  storage,
  blocked,
}: {
  backup: ReactNode;
  storage: ReactNode;
  blocked: boolean;
}) {
  const [section, setSection] = useState<"backup" | "storage" | "browser">(
    "backup",
  );
  return (
    <section className="workspace-data" aria-labelledby="workspace-data-title">
      <h2 id="workspace-data-title">Your local workspace</h2>
      <p>Manage saved data, create a backup, or recover an earlier profile.</p>
      <nav className="settings-navigation" aria-label="Settings sections">
        <button
          type="button"
          className="button--secondary"
          aria-current={section === "backup" ? "page" : undefined}
          disabled={blocked}
          onClick={() => setSection("backup")}
        >
          Backup and recovery
        </button>
        <button
          type="button"
          className="button--secondary"
          aria-current={section === "storage" ? "page" : undefined}
          disabled={blocked}
          onClick={() => setSection("storage")}
        >
          Storage and deletion
        </button>
        <button
          type="button"
          className="button--secondary"
          aria-current={section === "browser" ? "page" : undefined}
          disabled={blocked}
          onClick={() => setSection("browser")}
        >
          Browser connections
        </button>
      </nav>
      <div hidden={section !== "backup"}>{backup}</div>
      <div hidden={section !== "storage"}>{storage}</div>
      <div hidden={section !== "browser"}>
        <h3>Chrome and Edge</h3>
        <p>
          Browser capture is unavailable in this unsigned development preview.
          Both browsers are disconnected; no native host is registered.
        </p>
        <p>
          Paste selected text into the application overlay to continue working.
          Browser connection setup requires a signed desktop and native host
          with verified Keychain access.
        </p>
      </div>
    </section>
  );
}
