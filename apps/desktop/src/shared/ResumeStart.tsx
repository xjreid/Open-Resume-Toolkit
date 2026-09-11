import { useState } from "react";
import { STARTING_PROFILES, type StartingProfile } from "./starting-profiles";

export function ResumeStart({
  disabled,
  onBuild,
}: {
  disabled: boolean;
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
        Start with a few essentials. You can add, reorder, and refine every part
        as you go.
      </p>
      <div className="resume-start-options resume-start-options--single">
        <div>
          <h3>Build from scratch</h3>
          <p>Start with your contact details and a useful set of sections.</p>
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
            Nothing is pre-filled with example experience. Make this resume your
            own.
          </p>
          <button
            type="button"
            disabled={disabled}
            onClick={() => onBuild(profile)}
          >
            Build from scratch
          </button>
        </div>
      </div>
    </section>
  );
}
