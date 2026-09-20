import { useEffect, useRef, useState } from "react";
import type { Catalog, SavedKey } from "./AiWorkspace";

const provider = (value: SavedKey["provider"]) =>
  value === "openai"
    ? "OpenAI"
    : value === "anthropic"
      ? "Anthropic"
      : "Gemini";
const title = (key: SavedKey) =>
  key.name ||
  (key.identificationNumber
    ? `Key #${key.identificationNumber}`
    : `Archived key · ${key.credentialId.slice(0, 8)}`);
const model = (key: SavedKey, catalog: Catalog | null) =>
  catalog?.entries.find(
    (entry) =>
      entry.provider ===
        (key.provider === "openai" ? "open_ai" : key.provider) &&
      entry.preset === key.preset &&
      !entry.disabled,
  )?.model ?? "Model unavailable";
const preset = (value: SavedKey["preset"]) =>
  value[0].toUpperCase() + value.slice(1);

export function AiDataKeyPicker({
  keys,
  catalog,
  value,
  disabled,
  onChange,
}: {
  keys: SavedKey[];
  catalog: Catalog | null;
  value: string;
  disabled: boolean;
  onChange: (value: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!open) return;
    const dismiss = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", dismiss);
    return () => document.removeEventListener("pointerdown", dismiss);
  }, [open]);
  useEffect(() => {
    if (disabled) setOpen(false);
  }, [disabled]);
  function choose(next: string) {
    onChange(next);
    setOpen(false);
    trigger.current?.focus();
  }
  return (
    <div
      className="ai-data-key-picker"
      ref={root}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          setOpen(false);
          trigger.current?.focus();
        }
      }}
    >
      <button
        ref={trigger}
        type="button"
        className="button--secondary ai-data-key-trigger"
        aria-haspopup="dialog"
        aria-expanded={open}
        disabled={disabled}
        onClick={() => setOpen(!open)}
      >
        Choose view
      </button>
      {open && (
        <div
          className="ai-data-key-popover"
          role="dialog"
          aria-modal="false"
          aria-label="Choose activity view"
        >
          <div className="ai-data-picker-heading">
            <strong>Activity view</strong>
            <span>Choose all activity or one API key.</span>
          </div>
          <button
            type="button"
            className={`ai-data-key-option${value === "" ? " ai-data-key-option--selected" : ""}`}
            aria-label="View activity for all keys"
            aria-pressed={value === ""}
            onClick={() => choose("")}
          >
            <span className="ai-data-key-option-icon">∑</span>
            <span>
              <strong>All keys</strong>
              <small>General activity · Every provider and model</small>
            </span>
            {value === "" && <span aria-hidden="true">✓</span>}
          </button>
          <div className="ai-data-key-options">
            {keys.map((key) => (
              <button
                type="button"
                key={key.credentialId}
                className={`ai-data-key-option${value === key.credentialId ? " ai-data-key-option--selected" : ""}`}
                aria-label={`View activity for ${title(key)}`}
                aria-pressed={value === key.credentialId}
                onClick={() => choose(key.credentialId)}
              >
                <span className="ai-data-key-option-icon">
                  {key.identificationNumber || "–"}
                </span>
                <span>
                  <strong>{title(key)}</strong>
                  <small>
                    {provider(key.provider)} · {preset(key.preset)}:{" "}
                    {model(key, catalog)}
                    {key.removed ? " · Removed" : ""}
                  </small>
                </span>
                {value === key.credentialId && (
                  <span aria-hidden="true">✓</span>
                )}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export function dataKeyDescription(
  key: SavedKey | undefined,
  catalog: Catalog | null,
) {
  return key
    ? {
        title: title(key),
        detail: `${provider(key.provider)} · ${preset(key.preset)}: ${model(key, catalog)}${key.removed ? " · Removed" : ""}`,
      }
    : {
        title: "All keys",
        detail: "General activity · Every provider and model",
      };
}
