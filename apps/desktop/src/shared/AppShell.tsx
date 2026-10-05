import { useEffect, useRef, type ReactNode } from "react";
import { WorkspaceIcon } from "./WorkspaceIcon";
import logo from "../assets/open-folio-reversed.svg";

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
  const content = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (content.current) {
      content.current.scrollTop = 0;
      content.current.scrollLeft = 0;
    }
  }, [destination]);
  return (
    <main className="shell shell--editor">
      <aside className="workspace-rail" aria-label="Your workspace">
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
              <WorkspaceIcon name={item.id} />
              <span>{item.label}</span>
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
          <WorkspaceIcon name="overlay" />
          <span>{overlayVisible ? "Hide overlay" : "Show overlay"}</span>
        </button>
        <div className="rail-storage-status">
          <WorkspaceIcon name="lock" />
          <div>
            <span>Encrypted storage</span>
            {status}
          </div>
        </div>
      </aside>
      <div className="workspace-main">
        <header className="workspace-page-heading">
          <h2>
            {destination === "import"
              ? "Master resume"
              : WORKSPACE_DESTINATIONS.find((item) => item.id === destination)
                  ?.label}
          </h2>
          <p>
            {destination === "resume" || destination === "import"
              ? "Tailoring uses your published resume"
              : "Local workspace"}
          </p>
        </header>
        <div className="app-content" ref={content}>
          {children}
        </div>
      </div>
    </main>
  );
}
