import { useEffect, useRef, useState } from "react";
import type { SavedKey } from "./AiWorkspace";

export function AiKeyMenu({
  saved,
  blocked,
  onTest,
  onPause,
  onRemove,
}: {
  saved: SavedKey;
  blocked: boolean;
  onTest: () => void;
  onPause: () => void;
  onRemove: () => void;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!open) return;
    root.current
      ?.querySelector<HTMLButtonElement>('[role="menuitem"]:not(:disabled)')
      ?.focus();
    const dismiss = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", dismiss);
    return () => document.removeEventListener("pointerdown", dismiss);
  }, [open]);
  useEffect(() => {
    if (blocked) setOpen(false);
  }, [blocked]);
  function act(action: () => void) {
    action();
    setOpen(false);
    trigger.current?.focus();
  }
  return (
    <div
      className="ai-key-menu"
      ref={root}
      onBlur={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget)) setOpen(false);
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          setOpen(false);
          trigger.current?.focus();
          event.stopPropagation();
        }
        if (
          open &&
          ["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)
        ) {
          event.preventDefault();
          const items = [
            ...event.currentTarget.querySelectorAll<HTMLButtonElement>(
              '[role="menuitem"]:not(:disabled)',
            ),
          ];
          const index = items.indexOf(
            document.activeElement as HTMLButtonElement,
          );
          const next =
            event.key === "Home"
              ? 0
              : event.key === "End"
                ? items.length - 1
                : (index + (event.key === "ArrowUp" ? -1 : 1) + items.length) %
                  items.length;
          items[next]?.focus();
        }
      }}
    >
      <button
        ref={trigger}
        type="button"
        className="ai-key-menu-trigger"
        aria-label={`Key #${saved.identificationNumber} options`}
        aria-haspopup="menu"
        aria-expanded={open}
        disabled={blocked}
        onClick={() => setOpen(!open)}
      >
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="currentColor"
          aria-hidden="true"
        >
          <circle cx="12" cy="5" r="2" />
          <circle cx="12" cy="12" r="2" />
          <circle cx="12" cy="19" r="2" />
        </svg>
      </button>
      {open && (
        <div
          className="ai-key-menu-popover"
          role="menu"
          aria-label={`Key #${saved.identificationNumber} actions`}
        >
          <button
            type="button"
            role="menuitem"
            aria-label={`Test key #${saved.identificationNumber}`}
            disabled={saved.cleanupRequired}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => act(onTest)}
          >
            Test key
          </button>
          <button
            type="button"
            role="menuitem"
            aria-label={`${saved.paused ? "Unpause" : "Pause"} key #${saved.identificationNumber}`}
            disabled={saved.cleanupRequired}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => act(onPause)}
          >
            {saved.paused ? "Unpause" : "Pause"}
          </button>
          <button
            type="button"
            role="menuitem"
            className="ai-key-menu-remove"
            aria-label={`Remove key #${saved.identificationNumber}`}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => act(onRemove)}
          >
            Remove
          </button>
        </div>
      )}
    </div>
  );
}
