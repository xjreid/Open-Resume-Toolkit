import { useState, type ReactNode } from "react";
import { STARTING_PROFILES, type StartingProfile } from "./starting-profiles";

export function ResumeStart({
  disabled,
  onBuild,
  importAction,
}: {
  disabled: boolean;
  importAction?: ReactNode;
  onBuild: (profile: StartingProfile) => void;
}) {
  const [profile, setProfile] = useState<StartingProfile>("general");
  const selected = STARTING_PROFILES.find(
    (candidate) => candidate.id === profile,
  )!;
  return (
    <section className="resume-start" aria-labelledby="resume-start-heading">
      <p className="eyebrow">Your master resume</p>
      <h2 id="resume-start-heading">How would you like to start?</h2>
      <p>
        Keep your experience in one resume. Build and edit it offline, then
        publish it when you’re ready.
      </p>
      <div className="resume-start-options">
        <div>
          <h3>Build from scratch</h3>
          <p>
            Start with your contact information. Choose suggested sections, or
            create your own.
          </p>
          <label htmlFor="starting-profile">Starting profile (optional)</label>
          <select
            id="starting-profile"
            value={profile}
            disabled={disabled}
            onChange={(event) => {
              const next = STARTING_PROFILES.find(
                (candidate) => candidate.id === event.target.value,
              );
              if (next) setProfile(next.id);
            }}
          >
            {STARTING_PROFILES.map((candidate) => (
              <option key={candidate.id} value={candidate.id}>
                {candidate.label}
              </option>
            ))}
          </select>
          <p className="starting-section-list" aria-live="polite">
            {selected.sections.length
              ? `Suggested sections: ${selected.sections.join(", ")}.`
              : "Begin with contact information and add your own sections."}
          </p>
          <p>
            You can add, rename, remove, and reorder any section later. No
            example experience is added.
          </p>
          <button
            type="button"
            disabled={disabled}
            onClick={() => onBuild(profile)}
          >
            Build from scratch
          </button>
        </div>
        <div>
          {importAction ?? (
            <>
              <h3>Import an existing resume</h3>
              <p>
                PDF and Word resume import is not available yet. You can build
                manually or restore an encrypted ORT backup below.
              </p>
              <button
                type="button"
                className="button--secondary"
                disabled
                aria-describedby="import-unavailable"
              >
                Import an existing resume
              </button>
              <p id="import-unavailable">
                Coming in a later development update.
              </p>
            </>
          )}
        </div>
      </div>
    </section>
  );
}
