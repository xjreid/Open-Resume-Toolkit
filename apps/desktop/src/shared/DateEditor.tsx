import { useId, useState } from "react";
import type {
  CalendarDate,
  ResumeDate,
  ResumeEntry,
} from "@ort/contracts/resume";
import { createEntityId } from "./resume-editor";
import { dateText, reversedDate } from "./resume-dates";

function emptyDate(order: number): ResumeDate {
  return { id: createEntityId(), order, label: "", start: null, end: null };
}

function hasInvalidYear(date: ResumeDate): boolean {
  return [date.start, date.end?.kind === "date" ? date.end.value : null].some(
    (value) =>
      value !== null &&
      (!Number.isInteger(value.year) || value.year < 1 || value.year > 9999),
  );
}

export function DateEditor({
  entry,
  canAdd,
  onChange,
}: {
  entry: ResumeEntry;
  canAdd: boolean;
  onChange: (entry: ResumeEntry) => void;
}) {
  const [replacing, setReplacing] = useState(false);
  const dates = entry.dates ?? [];
  const legacy = Boolean(entry.dateRange.trim());
  function update(id: string, change: Partial<ResumeDate>) {
    onChange({
      ...entry,
      dates: dates.map((date) =>
        date.id === id ? { ...date, ...change } : date,
      ),
    });
  }
  return (
    <section className="date-editor" aria-label="Entry dates">
      <h3>Dates</h3>
      <p>
        Use a year alone or add a month. Leave an endpoint blank when it is
        unknown.
      </p>
      {legacy ? (
        <>
          <p>Existing date text: {entry.dateRange}</p>
          {replacing ? (
            <div
              className="date-replacement"
              role="group"
              aria-label="Replace existing date text"
            >
              <p>
                Replace this text with empty date fields? Enter the dates
                yourself; no dates will be guessed. You can undo this content
                change.
              </p>
              <button
                type="button"
                disabled={!canAdd}
                onClick={() => {
                  onChange({ ...entry, dateRange: "", dates: [emptyDate(0)] });
                  setReplacing(false);
                }}
              >
                Replace date text
              </button>
              <button type="button" onClick={() => setReplacing(false)}>
                Keep date text
              </button>
            </div>
          ) : (
            <button
              type="button"
              disabled={!canAdd}
              onClick={() => setReplacing(true)}
            >
              Use date fields instead
            </button>
          )}
        </>
      ) : (
        <>
          {dates.map((date, index) => (
            <fieldset
              data-validation-path={`date.${date.id}`}
              className="date-record"
              key={date.id}
            >
              <legend>Date {index + 1}</legend>
              <label
                data-validation-path={`date.${date.id}.label`}
                className="field"
              >
                Date label (optional)
                <input
                  value={date.label}
                  maxLength={2000}
                  onChange={(event) =>
                    update(date.id, { label: event.target.value })
                  }
                />
              </label>
              <CalendarFields
                label="Start or single date"
                value={date.start}
                onChange={(start) => update(date.id, { start })}
              />
              <label className="field">
                End date
                <select
                  value={date.end?.kind ?? "none"}
                  onChange={(event) =>
                    update(date.id, {
                      end:
                        event.target.value === "present"
                          ? { kind: "present" }
                          : event.target.value === "date"
                            ? {
                                kind: "date",
                                value: {
                                  year: 0,
                                  month: null,
                                  expected: false,
                                },
                              }
                            : null,
                    })
                  }
                >
                  <option value="none">No end date</option>
                  <option value="date">Year or month and year</option>
                  <option value="present">Present / current</option>
                </select>
              </label>
              {date.end?.kind === "date" ? (
                <CalendarFields
                  label="End"
                  value={date.end.value}
                  onChange={(value) =>
                    update(date.id, {
                      end: value ? { kind: "date", value } : null,
                    })
                  }
                />
              ) : null}
              <p aria-live="polite">
                {!hasInvalidYear(date) && reversedDate(date)
                  ? "Check this range: the end is earlier than the start. You can still save it."
                  : ""}
              </p>
              <p>
                Date text:{" "}
                {hasInvalidYear(date)
                  ? "Enter a valid year to preview this date."
                  : dateText(date) || "Not included until a date is entered."}
              </p>
              <div className="move-controls">
                {([-1, 1] as const).map((direction) => (
                  <button
                    key={direction}
                    type="button"
                    disabled={
                      index + direction < 0 || index + direction >= dates.length
                    }
                    aria-label={`Move date ${index + 1} ${direction < 0 ? "up" : "down"}`}
                    onClick={() => {
                      const reordered = [...dates];
                      [reordered[index], reordered[index + direction]] = [
                        reordered[index + direction],
                        reordered[index],
                      ];
                      onChange({ ...entry, dates: reordered });
                    }}
                  >
                    {direction < 0 ? "↑" : "↓"}
                  </button>
                ))}
                <button
                  type="button"
                  className="button--danger"
                  onClick={() =>
                    onChange({
                      ...entry,
                      dates: dates.filter((value) => value.id !== date.id),
                    })
                  }
                >
                  Remove date {index + 1}
                </button>
              </div>
            </fieldset>
          ))}
          <button
            type="button"
            disabled={!canAdd}
            onClick={() =>
              onChange({ ...entry, dates: [...dates, emptyDate(dates.length)] })
            }
          >
            Add date
          </button>
        </>
      )}
    </section>
  );
}

function CalendarFields({
  label,
  value,
  onChange,
}: {
  label: string;
  value: CalendarDate | null;
  onChange: (value: CalendarDate | null) => void;
}) {
  const id = useId();
  const current = value ?? { year: 0, month: null, expected: false };
  const invalid =
    value !== null &&
    (!Number.isInteger(value.year) || value.year < 1 || value.year > 9999);
  function update(change: Partial<CalendarDate>) {
    const next = { ...current, ...change };
    onChange(
      next.year === 0 && next.month === null && !next.expected ? null : next,
    );
  }
  return (
    <fieldset className="calendar-fields">
      <legend>{label}</legend>
      <div className="field-grid">
        <label className="field">
          {label} year
          <input
            type="number"
            min={1}
            max={9999}
            step={1}
            value={current.year || ""}
            aria-invalid={invalid || undefined}
            aria-describedby={invalid ? `${id}-error` : undefined}
            onChange={(event) =>
              update({
                year:
                  event.target.value === "" ? 0 : Number(event.target.value),
              })
            }
          />
        </label>
        <label className="field">
          {label} month (optional)
          <select
            value={current.month ?? ""}
            onChange={(event) =>
              update({
                month:
                  event.target.value === "" ? null : Number(event.target.value),
              })
            }
          >
            <option value="">Year only</option>
            {[
              "January",
              "February",
              "March",
              "April",
              "May",
              "June",
              "July",
              "August",
              "September",
              "October",
              "November",
              "December",
            ].map((month, index) => (
              <option key={month} value={index + 1}>
                {month}
              </option>
            ))}
          </select>
        </label>
      </div>
      <label>
        <input
          type="checkbox"
          checked={current.expected}
          onChange={(event) => update({ expected: event.target.checked })}
        />{" "}
        Expected {label.toLowerCase()}
      </label>
      {invalid ? (
        <p id={`${id}-error`} className="field-error">
          Enter a year from 1 to 9999, or clear this date.
        </p>
      ) : null}
      {value ? (
        <button
          type="button"
          className="button--quiet"
          onClick={() => onChange(null)}
        >
          Clear {label.toLowerCase()}
        </button>
      ) : null}
    </fieldset>
  );
}
