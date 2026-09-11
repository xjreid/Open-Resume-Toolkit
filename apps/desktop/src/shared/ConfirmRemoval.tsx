import { useEffect, useRef, useState } from "react";

export function ConfirmRemoval({
  label,
  description,
  onRemove,
}: {
  label: string;
  description: string;
  onRemove: () => void;
}) {
  const [confirming, setConfirming] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const container = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!confirming) return;
    function close(event: PointerEvent) {
      if (!container.current?.contains(event.target as Node)) {
        setConfirming(false);
      }
    }
    function closeWithEscape(event: KeyboardEvent) {
      if (event.key === "Escape") {
        setConfirming(false);
        trigger.current?.focus();
      }
    }
    window.addEventListener("pointerdown", close);
    window.addEventListener("keydown", closeWithEscape);
    return () => {
      window.removeEventListener("pointerdown", close);
      window.removeEventListener("keydown", closeWithEscape);
    };
  }, [confirming]);
  return (
    <div ref={container} className="removal-control">
      <button
        ref={trigger}
        type="button"
        className="button--danger button--compact"
        aria-expanded={confirming}
        onClick={() => setConfirming(true)}
      >
        {label}
      </button>
      {confirming ? (
        <div
          role="group"
          aria-label={`Confirm ${label.toLowerCase()}`}
          className="removal-confirmation"
        >
          <div className="removal-confirmation__heading">
            <span aria-hidden="true">!</span>
            <strong>Remove this item?</strong>
          </div>
          <p>{description} You can undo this change while editing.</p>
          <div className="removal-confirmation__actions">
            <button
              type="button"
              className="button--secondary"
              onClick={() => {
                setConfirming(false);
                trigger.current?.focus();
              }}
            >
              Cancel
            </button>
            <button
              type="button"
              className="button--danger"
              onClick={() => {
                setConfirming(false);
                onRemove();
              }}
            >
              Remove
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
}
