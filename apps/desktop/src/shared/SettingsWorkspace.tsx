import { BrowserConnectionSettings } from "./BrowserConnectionSettings";
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
    <section className="workspace-data" aria-label="Settings">
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
        {section === "browser" && <BrowserConnectionSettings />}
      </div>
    </section>
  );
}
