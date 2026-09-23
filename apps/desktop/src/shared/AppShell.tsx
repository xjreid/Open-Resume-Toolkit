import type { ReactNode } from "react";
import logo from "../assets/open-frame-icon.svg";

// Add a destination only when its workspace is implemented. Feature state stays
// with its owner; changing destinations must not discard an editing session.
export const WORKSPACE_DESTINATIONS = [
  { id: "resume", label: "Master resume" },
  { id: "import", label: "Import resume" },
  { id: "ai", label: "AI & monitoring" },
  { id: "tracker", label: "Application tracker" },
  { id: "settings", label: "Settings" },
] as const;
export type WorkspaceDestination =
  (typeof WORKSPACE_DESTINATIONS)[number]["id"];

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
  navigationBlocked,
  status,
  children,
}: {
  destination: WorkspaceDestination;
  onNavigate: (destination: WorkspaceDestination) => void;
  onOpenApplication: () => void;
  navigationBlocked: boolean;
  status: ReactNode;
  children: ReactNode;
}) {
  return (
    <main className="shell shell--editor">
      <header className="masthead masthead--workspace">
        <Brand />
        {status}
        <nav className="workspace-shortcuts" aria-label="Workspace areas">
          <button
            type="button"
            className="button--secondary"
            disabled={navigationBlocked}
            onClick={onOpenApplication}
          >
            Application workspace
          </button>
          {WORKSPACE_DESTINATIONS.map((item) => (
            <button
              key={item.id}
              type="button"
              className="button--secondary"
              aria-current={destination === item.id ? "page" : undefined}
              disabled={navigationBlocked}
              onClick={() => onNavigate(item.id)}
            >
              {item.label}
            </button>
          ))}
        </nav>
      </header>
      <div className="app-content">{children}</div>
    </main>
  );
}
