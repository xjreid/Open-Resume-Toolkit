import { useEffect, useState } from "react";
import type * as Wire from "@ort/contracts/wire";
import { useChatGptPlan } from "./use-chatgpt-plan";
import {
  REASONING_EFFORT_LABELS,
  quotaWindowLabel,
} from "./chatgpt-plan-presentation";

export function ApplicationCodexControls({
  disabled,
  onNotice,
}: {
  disabled: boolean;
  onNotice: (notice: string) => void;
}) {
  const [status, setStatus] = useState<Wire.PlanStatus | null>(null);
  const { working, notice, save } = useChatGptPlan(true, status, setStatus);
  useEffect(() => {
    if (notice) onNotice(notice);
  }, [notice, onNotice]);
  const settings = status?.settings;
  const model = status?.models.find((item) => item.id === settings?.model);
  const locked =
    disabled || working || !status?.connected || status.operationActive;
  if (!status || !status.connected) {
    return (
      <p className="application-ai-disabled" role="status">
        {status
          ? "AI is disabled until an account is connected."
          : "Checking Codex connection…"}
      </p>
    );
  }
  return (
    <>
      <select
        aria-label="Codex model"
        value={settings?.model ?? ""}
        disabled={locked}
        onChange={(event) => {
          const next = status?.models.find(
            (item) => item.id === event.target.value,
          );
          if (!next?.supported || !settings) return;
          void save({
            model: next.id,
            reasoning: next.reasoningEfforts.includes(settings.reasoning)
              ? settings.reasoning
              : next.reasoningEfforts[0],
          });
        }}
      >
        {!settings?.model && <option value="">Checking models…</option>}
        {settings?.model && !model && (
          <option value={settings.model} disabled>
            {settings.model} — Unavailable
          </option>
        )}
        {status?.models.map((item) => (
          <option key={item.id} value={item.id} disabled={!item.supported}>
            {item.name}
            {item.supported ? "" : " — Unavailable"}
          </option>
        ))}
      </select>
      <div className="application-reasoning">
        <label htmlFor="application-codex-reasoning">Reasoning</label>
        <select
          id="application-codex-reasoning"
          aria-label="Codex reasoning"
          value={settings?.reasoning ?? "medium"}
          disabled={locked || !model?.supported}
          onChange={(event) =>
            void save({ reasoning: event.target.value as Wire.ReasoningEffort })
          }
        >
          {Object.entries(REASONING_EFFORT_LABELS).map(([value, label]) => (
            <option
              key={value}
              value={value}
              disabled={
                !model?.reasoningEfforts.includes(value as Wire.ReasoningEffort)
              }
            >
              {label}
            </option>
          ))}
        </select>
      </div>
      {status?.connected && !!status.quota?.windows.length && (
        <ul
          className="application-quota"
          aria-label="Account-wide remaining usage"
        >
          {status.quota.windows.map((window) => (
            <li key={`${window.limitId}-${window.window}`}>
              <span>{quotaWindowLabel(window)}</span>
              <span>
                {window.remainingPercent.toLocaleString(undefined, {
                  maximumFractionDigits: 1,
                })}
                % remaining
              </span>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}
