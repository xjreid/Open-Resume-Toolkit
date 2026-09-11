import type { CalendarDate, ResumeDate } from "@ort/contracts/resume";
const months = [
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
function calendarText(value: CalendarDate): string {
  const date =
    value.month === null
      ? `${value.year}`
      : `${months[value.month - 1]} ${value.year}`;
  return value.expected ? `Expected ${date}` : date;
}
export function dateText(date: ResumeDate): string {
  const parts = [
    date.start ? calendarText(date.start) : null,
    date.end?.kind === "present"
      ? "Present"
      : date.end?.kind === "date"
        ? calendarText(date.end.value)
        : null,
  ].filter((value) => value !== null);
  if (!parts.length) return "";
  return `${date.label.trim() ? `${date.label.trim()}: ` : ""}${parts.join("–")}`;
}
export function reversedDate(date: ResumeDate): boolean {
  if (!date.start || date.end?.kind !== "date") return false;
  const end = date.end.value;
  return (
    date.start.year > end.year ||
    (date.start.year === end.year &&
      (date.start.month ?? 1) > (end.month ?? 12))
  );
}
