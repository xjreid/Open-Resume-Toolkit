import type { ReactNode } from "react";
import logo from "../assets/open-frame-icon.svg";

// Add a destination only when its workspace is implemented. Feature state stays
// with its owner; changing destinations must not discard an editing session.
export const WORKSPACE_DESTINATIONS = [
  { id: "resume", label: "Master resume" },
  { id: "ai", label: "AI & monitoring" },
  { id: "tracker", label: "Application tracker" },
  { id: "settings", label: "Settings" },
] as const;
export type WorkspaceDestination =
  | (typeof WORKSPACE_DESTINATIONS)[number]["id"]
  | "import";

export function Brand() {
  return (
    <div className="brand-lockup">
      <img className="brand-icon" src={logo} alt="" width="36" height="36" />
      <h1 aria-label="Open Resume Toolkit">
        <span>Open</span> <span>Resume Toolkit</span>
      </h1>
    </div>
  );
}

export function AppShell({
  destination,
  onNavigate,
  onOpenApplication,
  overlayVisible,
  navigationBlocked,
  status,
  children,
}: {
  destination: WorkspaceDestination;
  onNavigate: (destination: WorkspaceDestination) => void;
  onOpenApplication: () => void;
  overlayVisible: boolean;
  navigationBlocked: boolean;
  status: ReactNode;
  children: ReactNode;
}) {
  return (
    <main className="shell shell--editor">
      <header className="masthead masthead--workspace">
        <Brand />
        <nav className="workspace-shortcuts" aria-label="Workspace areas">
          {WORKSPACE_DESTINATIONS.map((item) => (
            <button
              key={item.id}
              type="button"
              className="button--secondary"
              aria-current={
                destination === item.id ||
                (item.id === "resume" && destination === "import")
                  ? "page"
                  : undefined
              }
              disabled={navigationBlocked}
              onClick={() => onNavigate(item.id)}
            >
              {item.label}
            </button>
          ))}
        </nav>
        <button
          type="button"
          className={`overlay-toggle${overlayVisible ? " overlay-toggle--active" : ""}`}
          aria-label={overlayVisible ? "Hide overlay" : "Show overlay"}
          title={overlayVisible ? "Hide overlay" : "Show overlay"}
          aria-pressed={overlayVisible}
          onClick={onOpenApplication}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
            <rect x="3" y="4" width="18" height="16" rx="2" />
            <path d="M8 12h8" />
            {!overlayVisible && <path d="M12 8v8" />}
          </svg>
        </button>
        {status}
      </header>
      <div className="app-content">{children}</div>
    </main>
  );
}
