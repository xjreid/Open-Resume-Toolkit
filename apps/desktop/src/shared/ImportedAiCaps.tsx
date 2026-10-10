import { useEffect, useState } from "react";
import type * as Wire from "@ort/contracts/wire";
import { invokeDesktop } from "./desktop-client";
import { keyDisplayName } from "./AiKeyPresentation";
const GENERAL = "00000000-0000-0000-0000-000000000000";
export function ImportedAiCaps({
  keys,
  blocked,
  refreshRevision,
  onChanged,
}: {
  keys: Wire.SavedAiKey[];
  blocked: boolean;
  refreshRevision: number;
  onChanged: () => void;
}) {
  const [archive, setArchive] = useState<Wire.ImportedAiGuardrails | null>(
    null,
  );
  const [selected, setSelected] = useState("");
  const [target, setTarget] = useState("");
  const [confirmation, setConfirmation] = useState("");
  const [working, setWorking] = useState(false);
  const [notice, setNotice] = useState("");
  useEffect(() => {
    let active = true;
    void invokeDesktop("load_imported_ai_guardrails")
      .then((result) => {
        if (!active) return;
        if (result.ok) setArchive(result.value);
        else
          setNotice(
            "Restored spending caps are unavailable. Reload to try again.",
          );
      })
      .catch(() => {
        if (active)
          setNotice(
            "Restored spending caps are unavailable. Reload to try again.",
          );
      });
    return () => {
      active = false;
    };
  }, [refreshRevision]);
  if (!archive?.policies.length && !notice) return null;
  const policy = archive?.policies.find((item) => item.id === selected);
  async function restore() {
    if (
      !archive ||
      !policy ||
      !target ||
      blocked ||
      working ||
      confirmation !== "RESTORE LIFETIME CAP"
    )
      return;
    setWorking(true);
    try {
      const result = await invokeDesktop("bind_imported_ai_guardrail", {
        expectedProfileId: archive.profileId,
        importId: policy.id,
        credentialId: target,
        confirmation,
      });
      if (!result.ok) {
        setNotice(
          result.error.code === "REVISION_CONFLICT"
            ? "Stop active AI work and remove the target’s existing cap before restoring this one."
            : "The cap could not be restored. Reload and check the selected key and currency.",
        );
        return;
      }
      setArchive(
        (current) =>
          current && {
            ...current,
            policies: current.policies.filter((item) => item.id !== policy.id),
          },
      );
      setSelected("");
      setTarget("");
      setConfirmation("");
      setNotice("Spending cap restored.");
      onChanged();
    } catch {
      setNotice("The cap could not be restored. Reload to check its status.");
    } finally {
      setWorking(false);
    }
  }
  return (
    <section className="ai-panel" aria-label="Restored spending caps">
      <h3>Restored spending caps</h3>
      <p>
        Backup caps are inactive until you choose where to apply them. API keys
        are not included in backups.
      </p>
      <label>
        Archived cap
        <select
          value={selected}
          disabled={blocked || working}
          onChange={(event) => {
            setSelected(event.target.value);
            setTarget("");
            setConfirmation("");
          }}
        >
          <option value="">Choose a restored cap</option>
          {archive?.policies.map((item) => (
            <option key={item.id} value={item.id}>
              {item.credentialId === GENERAL
                ? "General cap"
                : `Previous key ${item.credentialId.slice(0, 8)}`}{" "}
              · {item.limitMicros / 1_000_000} {item.currency} ·{" "}
              {item.period.replaceAll("_", " ")}
            </option>
          ))}
        </select>
      </label>
      {policy && (
        <>
          <p>
            This restores a lifetime cap of {policy.limitMicros / 1_000_000}{" "}
            {policy.currency}. Counted spend and unresolved exposure are
            preserved; previously reserved spend becomes unresolved. The
            baseline also includes higher lifetime totals already tracked for
            the selected target.
          </p>
          <label>
            Apply to
            <select
              value={target}
              disabled={blocked || working}
              onChange={(event) => setTarget(event.target.value)}
            >
              <option value="">Choose a target</option>
              <option value={GENERAL}>General spending</option>
              {keys
                .filter((key) => !key.removed && !key.cleanupRequired)
                .map((key) => (
                  <option key={key.credentialId} value={key.credentialId}>
                    {keyDisplayName(key)}
                  </option>
                ))}
            </select>
          </label>
          <label>
            Type RESTORE LIFETIME CAP
            <input
              value={confirmation}
              disabled={blocked || working}
              onChange={(event) => setConfirmation(event.target.value)}
            />
          </label>
          <button
            type="button"
            disabled={
              blocked ||
              working ||
              !target ||
              confirmation !== "RESTORE LIFETIME CAP"
            }
            onClick={() => void restore()}
          >
            Restore lifetime cap
          </button>
        </>
      )}
      {notice && <p role="status">{notice}</p>}
    </section>
  );
}
