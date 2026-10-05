export function WorkspaceIcon({
  name,
}: {
  name: "resume" | "ai" | "tracker" | "settings" | "overlay" | "lock";
}) {
  return (
    <svg
      className="workspace-icon"
      viewBox="0 0 24 24"
      aria-hidden="true"
      focusable="false"
    >
      {name === "resume" && (
        <>
          <path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z" />
          <path d="M14 3v6h6M8 13h8M8 17h5" />
        </>
      )}
      {name === "ai" && (
        <>
          <circle cx="8" cy="9" r="5" />
          <path d="m12 13 8 8m-2-2 3-3m-6 0 3-3" />
        </>
      )}
      {name === "tracker" && (
        <>
          <rect x="3" y="4" width="18" height="16" rx="2" />
          <path d="M3 10h18M9 10v10M15 10v10" />
        </>
      )}
      {name === "settings" && (
        <>
          <path d="m9 3-1 3-3 1-2 3 2 2-1 3 2 3 3-1 3 2 3-2 3 1 2-3-1-3 2-2-2-3-3-1-1-3z" />
          <circle cx="12" cy="12" r="3" />
        </>
      )}
      {name === "overlay" && (
        <>
          <rect x="3" y="4" width="18" height="16" rx="2" />
          <rect x="12" y="11" width="9" height="9" rx="1" />
        </>
      )}
      {name === "lock" && (
        <>
          <rect x="5" y="10" width="14" height="11" rx="2" />
          <path d="M8 10V6a4 4 0 0 1 8 0v4M12 14v3" />
        </>
      )}
    </svg>
  );
}
