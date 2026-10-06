import type { ResumeDocument, ResumeEntry } from "@ort/contracts/resume";
import type { DocumentStyle } from "@ort/contracts/export";
import type { ContactDivider } from "./resume-view-types";
import { FormattedText, safeInlineHref } from "./FormattedText";
import { dateText } from "./resume-dates";
import { fieldsForRole } from "./resume-fields";
function DisplayLink({ label, url }: { label: string; url: string }) {
  return safeInlineHref(url) ? (
    <a className="formatted-link" href={url} target="_blank" rel="noreferrer">
      {label.trim() || url}
    </a>
  ) : (
    <span>{label.trim() || url}</span>
  );
}

// Only explicit web and email protocols become navigable links.
export function PublishedResume({
  document,
  pdf = false,
  style = "technical",
  contactDivider = "dot",
  onSelect,
  onSelectEntry,
  onAddEntry,
  canAddEntry = false,
}: {
  document: ResumeDocument;
  pdf?: boolean;
  style?: DocumentStyle;
  contactDivider?: ContactDivider;
  onSelect?: (part: string) => void;
  onSelectEntry?: (sectionId: string, entryId: string) => void;
  onAddEntry?: (sectionId: string) => void;
  canAddEntry?: boolean;
}) {
  const visibleSections = document.sections.filter(
    (section) =>
      onSelect ||
      onAddEntry ||
      section.entries.some((entry) => entryHasVisibleContent(entry)),
  );
  return (
    <article
      className={`published-content resume-document resume-document--${style}`}
      aria-label={
        onSelect
          ? "Live draft resume content"
          : pdf
            ? "PDF resume content"
            : "Published resume content"
      }
    >
      <header
        className={`resume-document__contact resume-document__contact--${contactDivider}`}
      >
        {onSelect ? (
          <button
            type="button"
            className="reading-edit-target"
            onClick={() => onSelect("contact")}
          >
            {document.contact.fullName || "Add contact information"}
          </button>
        ) : (
          <h2
            className={
              document.contact.fullName.trim() ? undefined : "visually-hidden"
            }
          >
            {document.contact.fullName.trim() ? (
              <FormattedText value={document.contact.fullName} />
            ) : (
              "Resume"
            )}
          </h2>
        )}
        {[
          document.contact.email,
          document.contact.phone,
          ...document.contact.location.split("\n"),
        ]
          .filter((value) => value.trim())
          .map((value, index) => (
            <span key={index}>
              <FormattedText value={value} />
            </span>
          ))}
        {document.contact.links
          .filter((link) => link.label.trim() || link.url.trim())
          .map((link, index) => (
            <DisplayLink
              key={link.id ?? index}
              label={link.label}
              url={link.url}
            />
          ))}
      </header>
      {visibleSections.map((section) => (
        <section className="resume-document__section" key={section.id}>
          <h3>
            {onSelect ? (
              <button
                type="button"
                className="reading-edit-target"
                onClick={() => onSelect(section.id)}
              >
                {section.heading || "Untitled section"}
              </button>
            ) : (
              section.heading
            )}
          </h3>
          {section.entries.map((entry, entryIndex) =>
            entryHasVisibleContent(entry) || onSelectEntry ? (
              <div
                className={`resume-document__entry${
                  entryHasTopRowContent(entry)
                    ? ""
                    : " resume-document__entry--without-top-row"
                }`}
                key={entry.id}
              >
                <div className="resume-document__primary">
                  <div className="resume-document__title-line">
                    {entry.heading.trim() ? (
                      <h4>
                        {onSelectEntry ? (
                          <button
                            type="button"
                            className="reading-edit-target"
                            onClick={() => onSelectEntry(section.id, entry.id)}
                          >
                            {entry.heading}
                          </button>
                        ) : (
                          <FormattedText value={entry.heading} />
                        )}
                      </h4>
                    ) : onSelectEntry ? (
                      <button
                        type="button"
                        className="reading-entry-action"
                        onClick={() => onSelectEntry(section.id, entry.id)}
                      >
                        Edit untitled entry {entryIndex + 1}
                      </button>
                    ) : null}
                    {fieldsForRole(entry, "details").some((field) =>
                      field.value.trim(),
                    ) ? (
                      <>
                        {entry.heading.trim() ? (
                          <span
                            className="entry-title-separator"
                            aria-hidden="true"
                          >
                            |
                          </span>
                        ) : null}
                        <span className="resume-document__inline-details">
                          {fieldsForRole(entry, "details")
                            .filter((field) => field.value.trim())
                            .map((field) => (
                              <span key={field.id}>
                                <FormattedText value={field.value} />
                              </span>
                            ))}
                        </span>
                      </>
                    ) : null}
                  </div>
                  {entry.subheading.trim() ? (
                    <p>
                      <FormattedText value={entry.subheading} />
                    </p>
                  ) : null}
                </div>
                <div className="resume-document__meta">
                  {entry.location.trim() ? (
                    <p>
                      <FormattedText value={entry.location} />
                    </p>
                  ) : null}
                  {entry.dateRange.trim() ? (
                    <p>
                      <FormattedText value={entry.dateRange} />
                    </p>
                  ) : null}
                  {entry.dates
                    ?.filter((date) => dateText(date))
                    .map((date) => (
                      <p key={date.id}>{dateText(date)}</p>
                    ))}
                  {fieldsForRole(entry, "extra")
                    .filter((field) => field.value.trim())
                    .map((field) => (
                      <p key={field.id}>
                        <FormattedText value={field.value} />
                      </p>
                    ))}
                </div>
                {fieldsForRole(entry, "paragraph")
                  .filter((field) => field.value.trim())
                  .map((field) => (
                    <p
                      key={field.id}
                      className="resume-document__body resume-document__body--paragraph"
                    >
                      <FormattedText value={field.value} />
                    </p>
                  ))}
                {entry.bullets.some((bullet) => bullet.text.trim()) ? (
                  <ul className="resume-document__body">
                    {entry.bullets
                      .filter((bullet) => bullet.text.trim())
                      .map((bullet) => (
                        <li key={bullet.id}>
                          <FormattedText value={bullet.text} />
                        </li>
                      ))}
                  </ul>
                ) : null}
                {entry.links.some(
                  (link) => link.label.trim() || link.url.trim(),
                ) ? (
                  <p className="resume-document__links">
                    {entry.links
                      .filter((link) => link.label.trim() || link.url.trim())
                      .map((link, index) => (
                        <DisplayLink
                          key={link.id ?? index}
                          label={link.label}
                          url={link.url}
                        />
                      ))}
                  </p>
                ) : null}
              </div>
            ) : null,
          )}
          {onAddEntry ? (
            <button
              type="button"
              className="reading-entry-action"
              disabled={!canAddEntry}
              onClick={() => onAddEntry(section.id)}
            >
              Add entry to {section.heading.trim() || "untitled section"}
            </button>
          ) : null}
        </section>
      ))}
    </article>
  );
}

function entryHasVisibleContent(entry: ResumeEntry): boolean {
  return Boolean(
    entry.heading.trim() ||
      entry.subheading.trim() ||
      entry.dateRange.trim() ||
      entry.location.trim() ||
      entry.dates?.some((date) => dateText(date)) ||
      entry.fields.some((field) => field.value.trim()) ||
      entry.bullets.some((bullet) => bullet.text.trim()) ||
      entry.links.some((link) => link.label.trim() || link.url.trim()),
  );
}

function entryHasTopRowContent(entry: ResumeEntry): boolean {
  return Boolean(
    entry.heading.trim() ||
      entry.subheading.trim() ||
      entry.dateRange.trim() ||
      entry.location.trim() ||
      entry.dates?.some((date) => dateText(date)) ||
      [
        ...fieldsForRole(entry, "details"),
        ...fieldsForRole(entry, "extra"),
      ].some((field) => field.value.trim()),
  );
}
