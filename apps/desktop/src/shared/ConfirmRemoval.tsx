import { useRef, useState } from "react";

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
  return (
    <div className="removal-control">
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
          <p>{description} You can undo this change while editing.</p>
          <button
            type="button"
            className="button--danger"
            onClick={() => {
              setConfirming(false);
              onRemove();
            }}
          >
            Confirm {label.toLowerCase()}
          </button>
          <button
            type="button"
            onClick={() => {
              setConfirming(false);
              trigger.current?.focus();
            }}
          >
            Keep it
          </button>
        </div>
      ) : null}
    </div>
  );
}
