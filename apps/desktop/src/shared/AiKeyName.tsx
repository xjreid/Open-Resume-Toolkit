import { invoke } from "@tauri-apps/api/core";
import { useRef, useState } from "react";
import { keyDisplayName } from "./AiKeyPresentation";
import type { KeyRegistry, SavedKey } from "./AiWorkspace";

export function AiKeyName({
  saved,
  blocked,
  workspaceBlocked,
  setWorking,
  onSaved,
}: {
  saved: SavedKey;
  blocked: boolean;
  workspaceBlocked: boolean;
  setWorking: (value: boolean) => void;
  onSaved: (id: string, name: string | null) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState(false);
  const pending = useRef<string | null>(null);
  const saving = useRef(false);
  const activeName = useRef<string | null>(null);
  const confirmed = useRef(saved.name ?? "");
  const label = keyDisplayName(saved);

  // Serialize writes and coalesce intermediate keystrokes so a late reply
  // cannot overwrite a newer name. Other mutations wait until this queue drains.
  async function drain() {
    if (saving.current) return;
    saving.current = true;
    setWorking(true);
    try {
      while (pending.current !== null) {
        const name = pending.current;
        pending.current = null;
        activeName.current = name;
        try {
          const response = await invoke<
            { ok: true; value: KeyRegistry } | { ok: false }
          >("rename_ai_key", {
            request: { credentialId: saved.credentialId, name },
          });
          const updated = response.ok
            ? response.value.keys.find(
                (key) =>
                  key.credentialId === saved.credentialId && !key.removed,
              )
            : null;
          if (updated) {
            confirmed.current = updated.name ?? "";
            onSaved(saved.credentialId, updated.name ?? null);
            setError(false);
          } else setError(true);
        } catch {
          setError(true);
        }
      }
    } finally {
      activeName.current = null;
      saving.current = false;
      setWorking(false);
    }
  }
  function queue(value: string) {
    if (workspaceBlocked) return;
    const name = value.trim();
    if (
      pending.current === name ||
      (saving.current &&
        pending.current === null &&
        activeName.current === name)
    )
      return;
    if (!saving.current && name === confirmed.current) return;
    pending.current = name;
    void drain();
  }
  return (
    <div className="ai-key-name-editor">
      {editing ? (
        <input
          autoFocus
          aria-label={`Name for ${label}`}
          className="ai-key-name-input"
          value={draft}
          maxLength={80}
          placeholder="Name this key"
          disabled={workspaceBlocked}
          onChange={(event) => {
            setDraft(event.target.value);
            queue(event.target.value);
          }}
          onBlur={() => {
            queue(draft);
            setEditing(false);
          }}
          onKeyDown={(event) => {
            if (event.key === "Enter" || event.key === "Escape")
              event.currentTarget.blur();
          }}
        />
      ) : (
        <button
          type="button"
          className="ai-key-name"
          aria-label={`Rename ${label}`}
          aria-description={label}
          title="Rename key"
          disabled={blocked}
          onClick={() => {
            if (!error) {
              setDraft(saved.name ?? "");
              confirmed.current = saved.name ?? "";
            }
            setEditing(true);
          }}
        >
          {label}
        </button>
      )}
      {error && (
        <span className="ai-key-inline-error" role="alert">
          Name not saved. Click the name to retry.
        </span>
      )}
    </div>
  );
}
