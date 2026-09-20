import { useEffect, useRef, useState } from "react";
import { dataKeyDescription } from "./AiDataKeyPicker";
import { formatKeyCreatedAt, ProviderLogo } from "./AiKeyPresentation";
import type { Catalog, SavedKey } from "./AiWorkspace";

export function AiRemovedKeyDataDialog({
  open,
  keys,
  catalog,
  disabled,
  onCancel,
  onConfirm,
}: {
  open: boolean;
  keys: SavedKey[];
  catalog: Catalog | null;
  disabled: boolean;
  onCancel: () => void;
  onConfirm: (credentialIds: string[]) => void;
}) {
  const [step, setStep] = useState<"keys" | "confirm">("keys");
  const [selected, setSelected] = useState<string[]>([]);
  const firstChoice = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    setStep("keys");
    setSelected([]);
    firstChoice.current?.focus();
  }, [open]);

  if (!open) return null;

  const chosen = keys.filter((key) => selected.includes(key.credentialId));
  const toggle = (credentialId: string) =>
    setSelected((current) =>
      current.includes(credentialId)
        ? current.filter((id) => id !== credentialId)
        : [...current, credentialId],
    );

  return (
    <div
      className="ai-data-action-backdrop"
      role="presentation"
      onPointerDown={(event) => {
        if (event.target === event.currentTarget && !disabled) onCancel();
      }}
    >
      <div
        className="ai-data-action-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="ai-removed-key-data-title"
        onKeyDown={(event) => {
          if (event.key === "Escape" && !disabled) onCancel();
        }}
      >
        <div className="ai-data-action-heading">
          <div>
            <span>Step {step === "keys" ? "1" : "2"} of 2</span>
            <h3 id="ai-removed-key-data-title">Delete removed key data</h3>
          </div>
          <button
            type="button"
            className="button--quiet button--compact"
            disabled={disabled}
            aria-label="Close"
            onClick={onCancel}
          >
            Close
          </button>
        </div>

        {step === "keys" ? (
          <>
            <div className="ai-data-action-copy">
              <strong>Choose removed keys</strong>
              <span>Only keys already removed from My Keys are available.</span>
            </div>
            {keys.length === 0 ? (
              <p className="ai-data-months-state">
                There are no removed keys with retained data.
              </p>
            ) : (
              <div className="ai-data-action-options">
                {keys.map((key, index) => {
                  const description = dataKeyDescription(key, catalog);
                  const isSelected = selected.includes(key.credentialId);
                  return (
                    <button
                      ref={index === 0 ? firstChoice : undefined}
                      type="button"
                      key={key.credentialId}
                      className={`ai-data-key-option${isSelected ? " ai-data-key-option--selected" : ""}`}
                      aria-label={`Select removed key ${description.title}`}
                      aria-pressed={isSelected}
                      onClick={() => toggle(key.credentialId)}
                    >
                      <span className="ai-data-key-option-logo">
                        <ProviderLogo provider={key.provider} />
                      </span>
                      <span className="ai-data-key-option-copy">
                        <strong>{description.title}</strong>
                        <small>{description.detail}</small>
                      </span>
                      <span className="ai-data-key-option-meta">
                        <small>{formatKeyCreatedAt(key.createdAt)}</small>
                        {isSelected && <span aria-hidden="true">✓</span>}
                      </span>
                    </button>
                  );
                })}
              </div>
            )}
            <div className="ai-data-action-footer">
              <button
                type="button"
                className="button--secondary"
                disabled={disabled}
                onClick={onCancel}
              >
                Cancel
              </button>
              <button
                type="button"
                disabled={disabled || selected.length === 0}
                onClick={() => setStep("confirm")}
              >
                Continue
              </button>
            </div>
          </>
        ) : (
          <>
            <div className="ai-data-action-copy">
              <strong>
                Delete data for {selected.length}{" "}
                {selected.length === 1 ? "removed key" : "removed keys"}?
              </strong>
              <span>
                {chosen
                  .map((key) => dataKeyDescription(key, catalog).title)
                  .join(" · ")}
              </span>
            </div>
            <p className="ai-data-action-warning">
              This cannot be undone. The selected keys will disappear from the
              Data key selector, and their activity will also be removed from
              the All keys data display.
            </p>
            <p className="ai-help">
              Lifetime and general spending totals on My Keys will not change.
            </p>
            <div className="ai-data-action-footer">
              <button
                type="button"
                className="button--secondary"
                disabled={disabled}
                onClick={() => setStep("keys")}
              >
                Back
              </button>
              <button
                type="button"
                className="button--danger"
                disabled={disabled}
                onClick={() => onConfirm(selected)}
              >
                Permanently delete data
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
