import type { ContactDetails, ResumeDocument } from "@ort/contracts/resume";

export type TrackerEntry = {
  company: string;
  title: string;
  location: string;
  dateApplied: string;
  status: string;
  customStatus: string;
  sourceUrl: string;
  resume: ResumeDocument | null;
  coverLetter: string | null;
  coverContact: ContactDetails | null;
  answers: { question: string; answer: string }[];
  style: "technical" | "professional" | "modern" | "plain";
};

export const emptyTrackerEntry = (): TrackerEntry => ({
  company: "",
  title: "",
  location: "",
  dateApplied: "",
  status: "applied",
  customStatus: "",
  sourceUrl: "",
  resume: null,
  coverLetter: null,
  coverContact: null,
  answers: [],
  style: "technical",
});

const statuses = [
  ["applied", "Applied"],
  ["online_assessment", "Online Assessment"],
  ["interview", "Interview"],
  ["accepted", "Accepted"],
  ["rejected", "Rejected"],
  ["withdrawn", "Withdrawn"],
  ["other", "Other"],
] as const;

export function TrackerFields({
  entry,
  onChange,
}: {
  entry: TrackerEntry;
  onChange: (entry: TrackerEntry) => void;
}) {
  function edit<K extends keyof TrackerEntry>(key: K, value: TrackerEntry[K]) {
    onChange({ ...entry, [key]: value });
  }
  return (
    <div className="tracker-fields">
      <label>
        Company
        <input
          maxLength={200}
          value={entry.company}
          onChange={(event) => edit("company", event.target.value)}
        />
      </label>
      <label>
        Job title
        <input
          maxLength={200}
          value={entry.title}
          onChange={(event) => edit("title", event.target.value)}
        />
      </label>
      <label>
        Location
        <input
          maxLength={200}
          value={entry.location}
          onChange={(event) => edit("location", event.target.value)}
        />
      </label>
      <label>
        Date applied
        <input
          type="date"
          value={entry.dateApplied}
          onChange={(event) => edit("dateApplied", event.target.value)}
        />
      </label>
      <label>
        Status
        <select
          value={entry.status}
          onChange={(event) =>
            onChange({
              ...entry,
              status: event.target.value,
              customStatus:
                event.target.value === "other" ? entry.customStatus : "",
            })
          }
        >
          {statuses.map(([value, label]) => (
            <option key={value} value={value}>
              {label}
            </option>
          ))}
          {!statuses.some(([value]) => value === entry.status) && (
            <option value={entry.status}>{entry.status}</option>
          )}
        </select>
      </label>
      {entry.status === "other" && (
        <label>
          Custom status
          <input
            maxLength={80}
            value={entry.customStatus}
            onChange={(event) => edit("customStatus", event.target.value)}
          />
        </label>
      )}
      <label>
        Link or source
        <input
          type="text"
          maxLength={4096}
          value={entry.sourceUrl}
          onChange={(event) => edit("sourceUrl", event.target.value)}
        />
      </label>
    </div>
  );
}
