import type { ReactNode } from "react";
import logo from "../assets/open-frame-icon.svg";

// Add a destination only when its workspace is implemented. Feature state stays
// with its owner; changing destinations must not discard an editing session.
export const WORKSPACE_DESTINATIONS = [
  { id: "resume", label: "Master resume" },
  { id: "import", label: "Import resume" },
  { id: "settings", label: "Settings" },
] as const;
export type WorkspaceDestination =
  (typeof WORKSPACE_DESTINATIONS)[number]["id"];

export function Brand({ title }: { title: string }) {
  return (
    <div className="brand-lockup">
      <img className="brand-icon" src={logo} alt="" width="36" height="36" />
      <div>
        <p className="eyebrow">Open Resume Toolkit</p>
        <h1>{title}</h1>
      </div>
    </div>
  );
}

export function AppShell({
  destination,
  onNavigate,
  navigationBlocked,
  status,
  children,
}: {
  destination: WorkspaceDestination;
  onNavigate: (destination: WorkspaceDestination) => void;
  navigationBlocked: boolean;
  status: ReactNode;
  children: ReactNode;
}) {
  return (
    <main className="shell shell--editor">
      <header className="masthead masthead--workspace">
        <Brand
          title={
            destination === "resume"
              ? "Resume workspace"
              : destination === "import"
                ? "Import resume"
                : "Settings"
          }
        />
        {status}
        <nav className="workspace-shortcuts" aria-label="Workspace areas">
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
