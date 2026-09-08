import { useId, useState } from "react";
import type { ImportChoices } from "@ort/contracts/import";
import {
  initialReview,
  importChoices,
  isReviewContact,
  REVIEW_TARGETS,
  reviewProblem,
  type ReviewChoice,
  type ReviewSession,
} from "./import-review";

// Not mounted in the production app until the native import gate passes.
// Keying the inner component prevents decisions leaking across review sessions.
export function ImportReviewPanel(props: {
  session: ReviewSession;
  currentRevision: number;
  busy: boolean;
  submissionBlocked?: boolean;
  error?: string;
  onCancel: () => void;
  onSubmit: (choices: ImportChoices) => void;
}) {
  return <ReviewContents key={props.session.id} {...props} />;
}

function ReviewContents({
  session,
  currentRevision,
  busy,
  submissionBlocked = false,
  error,
  onCancel,
  onSubmit,
}: Parameters<typeof ImportReviewPanel>[0]) {
  const id = useId();
  const [choices, setChoices] = useState(() => initialReview(session));
  const stale = currentRevision !== session.baseRevision;
  const pending = choices.filter(
    (choice) => choice.status === "pending",
  ).length;
  const kept = choices.filter((choice) => choice.status === "accept").length;
  const problems = choices.map((choice) =>
    reviewProblem(session, choice, choices),
  );
  const overBudget =
    choices.reduce(
      (total, choice) =>
        total +
        (choice.status === "accept"
          ? Array.from(choice.value).length +
            Array.from(choice.newSection).length
          : 0),
      0,
    ) > 100000;
  const ready =
    choices.length > 0 && kept > 0 && !overBudget && problems.every((p) => !p);
  function update(index: number, patch: Partial<ReviewChoice>) {
    setChoices((current) =>
      current.map((choice, position) =>
        position === index ? { ...choice, ...patch } : choice,
      ),
    );
  }
  return (
    <section className="import-review" aria-labelledby={`${id}-heading`}>
      <h2 id={`${id}-heading`}>Review imported content</h2>
      <p>
        Compare each suggestion with the original text. Nothing is kept until
        you choose it. Your saved resume stays unchanged during review.
      </p>
      <p role="status">
        {pending} of {choices.length} blocks still need a decision. {kept} kept.
      </p>
      {stale && (
        <p role="alert">
          Your draft changed. Cancel and restart review against the current
          saved draft.
        </p>
      )}
      {error && <p role="alert">{error}</p>}
      {overBudget && (
        <p role="alert">
          Reviewed content exceeds the 100,000-character review limit. Shorten
          or reject blocks before applying.
        </p>
      )}
      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (ready && !busy && !stale && !submissionBlocked)
            onSubmit(importChoices(choices));
        }}
      >
        <fieldset disabled={busy || stale}>
          <legend>Review every source block</legend>
          {session.blocks.map((block, index) => {
            const choice = choices[index];
            const prefix = `${id}-${index}`;
            const contact = isReviewContact(choice.target);
            return (
              <fieldset key={index} className="import-review-block">
                <legend>
                  Block {index + 1}
                  {block.page !== null ? ` · Page ${block.page}` : ""}
                </legend>
                <details open>
                  <summary>Original extracted text</summary>
                  <pre className="import-review-source">
                    {block.source || "(Empty block)"}
                  </pre>
                </details>
                <p>{block.explanation}</p>
                <label htmlFor={`${prefix}-decision`}>Decision</label>
                <select
                  id={`${prefix}-decision`}
                  value={choice.status}
                  onChange={(event) => {
                    const status = event.target.value;
                    if (
                      status === "pending" ||
                      status === "accept" ||
                      status === "reject"
                    )
                      update(index, { status });
                  }}
                >
                  <option value="pending">Not reviewed</option>
                  <option value="accept">Keep and map</option>
                  <option value="reject">Reject this block</option>
                </select>
                {choice.status !== "reject" && (
                  <>
                    <label htmlFor={`${prefix}-target`}>Use as</label>
                    <select
                      id={`${prefix}-target`}
                      value={choice.target}
                      onChange={(event) => {
                        const target = REVIEW_TARGETS.find(
                          (candidate) => candidate.id === event.target.value,
                        );
                        if (target)
                          update(index, {
                            target: target.id,
                            contactMode: "fillEmpty",
                            proposedSection: null,
                          });
                      }}
                    >
                      {REVIEW_TARGETS.map((target) => (
                        <option key={target.id} value={target.id}>
                          {target.label}
                        </option>
                      ))}
                    </select>
                    <label htmlFor={`${prefix}-value`}>Reviewed value</label>
                    <textarea
                      id={`${prefix}-value`}
                      value={choice.value}
                      maxLength={50000}
                      onChange={(event) =>
                        update(index, { value: event.target.value })
                      }
                    />
                    {contact ? (
                      <>
                        <p>
                          Existing value:{" "}
                          {session.contacts[
                            choice.target as keyof typeof session.contacts
                          ] || "(Empty)"}
                        </p>
                        <label htmlFor={`${prefix}-contact`}>
                          Existing contact information
                        </label>
                        <select
                          id={`${prefix}-contact`}
                          value={choice.contactMode}
                          onChange={(event) => {
                            const contactMode = event.target.value;
                            if (
                              contactMode === "fillEmpty" ||
                              contactMode === "replace" ||
                              contactMode === "keepExisting"
                            )
                              update(index, { contactMode });
                          }}
                        >
                          <option value="fillEmpty">Fill only if empty</option>
                          <option value="replace">
                            Replace with reviewed value
                          </option>
                          <option value="keepExisting">
                            Keep existing value
                          </option>
                        </select>
                      </>
                    ) : (
                      <>
                        <label htmlFor={`${prefix}-section`}>
                          Destination section
                        </label>
                        <select
                          id={`${prefix}-section`}
                          value={
                            choice.proposedSection !== null
                              ? `proposal:${choice.proposedSection}`
                              : choice.sectionId
                                ? `existing:${choice.sectionId}`
                                : ""
                          }
                          onChange={(event) => {
                            const value = event.target.value;
                            update(index, {
                              proposedSection: value.startsWith("proposal:")
                                ? Number(value.slice(9))
                                : null,
                              sectionId: value.startsWith("existing:")
                                ? value.slice(9)
                                : "",
                            });
                          }}
                        >
                          <option value="">New section</option>
                          {choice.proposedSection !== null &&
                            (choices[choice.proposedSection]?.status !==
                              "accept" ||
                              choices[choice.proposedSection]?.target !==
                                "section") && (
                              <option
                                value={`proposal:${choice.proposedSection}`}
                                disabled
                              >
                                Previously selected heading is no longer kept
                              </option>
                            )}
                          {session.sections.map((section) => (
                            <option
                              key={section.id}
                              value={`existing:${section.id}`}
                            >
                              {section.heading || "Untitled section"}
                            </option>
                          ))}
                          {choice.target !== "section" &&
                            choices.map((candidate, position) =>
                              candidate.status === "accept" &&
                              candidate.target === "section" ? (
                                <option
                                  key={`proposal:${position}`}
                                  value={`proposal:${position}`}
                                >
                                  Block {position + 1}:{" "}
                                  {candidate.value || "Untitled section"}
                                </option>
                              ) : null,
                            )}
                        </select>
                        {!choice.sectionId &&
                          choice.proposedSection === null &&
                          choice.target !== "section" && (
                            <>
                              <label htmlFor={`${prefix}-new`}>
                                New section heading
                              </label>
                              <input
                                id={`${prefix}-new`}
                                value={choice.newSection}
                                maxLength={200}
                                onChange={(event) =>
                                  update(index, {
                                    newSection: event.target.value,
                                  })
                                }
                              />
                            </>
                          )}
                      </>
                    )}
                    {choice.status === "accept" && problems[index] && (
                      <p role="status">{problems[index]}</p>
                    )}
                  </>
                )}
              </fieldset>
            );
          })}
        </fieldset>
        <p>
          Kept content will be submitted for validation before a new draft
          revision is saved.
        </p>
        <button
          type="submit"
          disabled={!ready || busy || stale || submissionBlocked}
        >
          {busy ? "Applying reviewed content…" : "Apply reviewed content"}
        </button>
        <button
          type="button"
          className="button--secondary"
          disabled={busy}
          onClick={onCancel}
        >
          Cancel import
        </button>
      </form>
    </section>
  );
}
