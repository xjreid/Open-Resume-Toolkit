import { useEffect, useRef, useState } from "react";
import type {
  ResumeEntry,
  ResumeDate,
  CalendarDate,
} from "@ort/contracts/resume";
import { createEntityId } from "./resume-editor";
import { dateText } from "./resume-dates";
const DATE_MONTHS = [
  "Jan.",
  "Feb.",
  "Mar.",
  "Apr.",
  "May.",
  "Jun.",
  "Jul.",
  "Aug.",
  "Sep.",
  "Oct.",
  "Nov.",
  "Dec.",
];

export function CanvasDateField({
  entry,
  disabled,
  onChange,
}: {
  entry: ResumeEntry;
  disabled: boolean;
  onChange: (entry: ResumeEntry) => void;
}) {
  const [editing, setEditing] = useState(false);
  const container = useRef<HTMLDivElement>(null);
  const date: ResumeDate = entry.dates?.[0] ?? {
    id: createEntityId(),
    order: 0,
    label: "",
    start: null,
    end: null,
  };
  const display = entry.dateRange.trim() || dateText(date);
  useEffect(() => {
    if (!editing) return;
    function closeOnOutsideClick(event: PointerEvent) {
      if (!container.current?.contains(event.target as Node)) setEditing(false);
    }
    window.addEventListener("pointerdown", closeOnOutsideClick);
    return () => window.removeEventListener("pointerdown", closeOnOutsideClick);
  }, [editing]);
  function updateDate(next: ResumeDate) {
    onChange({ ...entry, dateRange: "", dates: [next] });
  }
  function updateStart(start: CalendarDate | null) {
    updateDate({ ...date, start });
  }
  function updateEnd(end: ResumeDate["end"]) {
    updateDate({ ...date, end });
  }
  return (
    <div
      ref={container}
      className={`canvas-field canvas-date-field${!display ? " canvas-field--empty" : ""}${editing ? " canvas-field--editing" : ""}`}
    >
      {editing ? (
        <div className="canvas-date-editor">
          <div className="canvas-date-editor__header">
            <strong>Date</strong>
            <button
              type="button"
              className="button--quiet button--compact"
              aria-label="Clear date"
              title="Clear date"
              disabled={disabled || !display}
              onClick={() => onChange({ ...entry, dateRange: "", dates: [] })}
            >
              Clear date
            </button>
            <button
              type="button"
              className="button--quiet button--compact"
              onClick={() => setEditing(false)}
            >
              Done
            </button>
          </div>
          {entry.dateRange.trim() ? (
            <p className="canvas-date-editor__legacy">
              Choosing a date below replaces “{entry.dateRange}”.
            </p>
          ) : null}
          <CompactCalendarFields
            label="Start"
            value={date.start}
            onChange={updateStart}
          />
          <label className="canvas-date-editor__end-mode">
            End
            <select
              value={date.end?.kind ?? "none"}
              onChange={(event) => {
                if (event.target.value === "present") {
                  updateEnd({ kind: "present" });
                  return;
                }
                if (event.target.value === "date") {
                  updateEnd({
                    kind: "date",
                    value: {
                      year: new Date().getFullYear(),
                      month: null,
                      expected: false,
                    },
                  });
                  return;
                }
                updateEnd(null);
              }}
            >
              <option value="none">No end date</option>
              <option value="date">Choose end date</option>
              <option value="present">Present (ongoing)</option>
            </select>
          </label>
          {date.end?.kind === "date" ? (
            <CompactCalendarFields
              label="End"
              value={date.end.value}
              onChange={(value) =>
                updateEnd(value ? { kind: "date", value } : null)
              }
            />
          ) : null}
          <p className="canvas-date-editor__preview">
            {dateText(date) || "Select a start date"}
          </p>
        </div>
      ) : (
        <button
          type="button"
          className="canvas-field__button"
          disabled={disabled}
          onClick={() => setEditing(true)}
        >
          <span className="canvas-field__label">Date</span>
          <span className="canvas-field__value">{display || "Add date"}</span>
        </button>
      )}
    </div>
  );
}

function CompactCalendarFields({
  label,
  value,
  onChange,
}: {
  label: string;
  value: CalendarDate | null;
  onChange: (value: CalendarDate | null) => void;
}) {
  return (
    <fieldset className="compact-calendar-fields">
      <legend>{label}</legend>
      <select
        aria-label={`${label} month`}
        value={value?.month ?? ""}
        disabled={!value}
        onChange={(event) => {
          if (!value) return;
          onChange({
            ...value,
            month: event.target.value ? Number(event.target.value) : null,
          });
        }}
      >
        <option value="">Year only</option>
        {DATE_MONTHS.map((month, index) => (
          <option key={month} value={index + 1}>
            {month}
          </option>
        ))}
      </select>
      <input
        type="number"
        min={1}
        max={9999}
        step={1}
        aria-label={`${label} year`}
        placeholder="Year"
        value={value?.year || ""}
        onChange={(event) => {
          const year = Number(event.target.value);
          onChange(
            event.target.value
              ? {
                  year,
                  month: value?.month ?? null,
                  expected: false,
                }
              : null,
          );
        }}
      />
    </fieldset>
  );
}
