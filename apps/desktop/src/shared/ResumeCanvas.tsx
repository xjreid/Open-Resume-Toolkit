import type { ResumeDocument, ResumeEntry } from "@ort/contracts/resume";
import type { DocumentStyle } from "@ort/contracts/export";
import type { ContactDivider } from "./resume-view-types";
import { CanvasField } from "./CanvasField";
import { CanvasDateField } from "./CanvasDateField";
import { ContactInformationEditor } from "./ContactInformationEditor";
import { ConfirmRemoval } from "./ConfirmRemoval";
import { createEntry, createBullet, createNamedField } from "./resume-editor";
import {
  PARAGRAPH_FIELD_LABEL,
  fieldRole,
  fieldsForRole,
  updateEntryField,
} from "./resume-fields";
export function ResumeCanvas({
  document,
  style,
  contactDivider,
  onContactDividerChange,
  showContactDivider = true,
  onFocusSection,
  disabled,
  canAddEntry,
  onChange,
}: {
  document: ResumeDocument;
  style: DocumentStyle;
  contactDivider: ContactDivider;
  onContactDividerChange: (divider: ContactDivider) => void;
  showContactDivider?: boolean;
  onFocusSection?: (sectionId: string) => void;
  disabled: boolean;
  canAddEntry: boolean;
  onChange: (update: (current: ResumeDocument) => ResumeDocument) => void;
}) {
  function changeContact(
    field: keyof Omit<ResumeDocument["contact"], "links">,
    value: string,
  ) {
    onChange((current) => ({
      ...current,
      contact: { ...current.contact, [field]: value },
    }));
  }
  function changeEntry(
    sectionId: string,
    entryId: string,
    update: (entry: ResumeEntry) => ResumeEntry,
  ) {
    onChange((current) => ({
      ...current,
      sections: current.sections.map((section) =>
        section.id !== sectionId
          ? section
          : {
              ...section,
              entries: section.entries.map((entry) =>
                entry.id === entryId ? update(entry) : entry,
              ),
            },
      ),
    }));
  }
  return (
    <div
      className={`resume-canvas resume-canvas--${style}`}
      aria-label="Editable resume"
    >
      <p className="resume-canvas__hint">
        Select any labeled area to edit your resume. Changes save automatically.
      </p>
      <header
        className="resume-canvas__contact"
        onFocusCapture={() => onFocusSection?.("contact")}
      >
        <CanvasField
          label="Name"
          value={document.contact.fullName}
          bold
          disabled={disabled}
          onChange={(value) => changeContact("fullName", value)}
        />
        <ContactInformationEditor
          contact={document.contact}
          divider={contactDivider}
          disabled={disabled}
          onDividerChange={onContactDividerChange}
          showDivider={showContactDivider}
          onChange={(contact) =>
            onChange((current) => ({ ...current, contact }))
          }
        />
      </header>
      {document.sections.map((section) => (
        <section
          className="resume-canvas__section"
          key={section.id}
          onFocusCapture={() => onFocusSection?.(section.id)}
        >
          <h3>{section.heading || "Untitled section"}</h3>
          {section.entries.map((entry) => (
            <CanvasEntry
              key={entry.id}
              entry={entry}
              disabled={disabled}
              onChange={(next) => changeEntry(section.id, entry.id, () => next)}
              onRemove={() =>
                onChange((current) => ({
                  ...current,
                  sections: current.sections.map((candidate) =>
                    candidate.id === section.id
                      ? {
                          ...candidate,
                          entries: candidate.entries.filter(
                            (item) => item.id !== entry.id,
                          ),
                        }
                      : candidate,
                  ),
                }))
              }
            />
          ))}
          <button
            type="button"
            className="canvas-add-entry button--secondary button--compact"
            disabled={disabled || !canAddEntry}
            onClick={() =>
              onChange((current) => ({
                ...current,
                sections: current.sections.map((candidate) =>
                  candidate.id === section.id
                    ? {
                        ...candidate,
                        entries: [
                          ...candidate.entries,
                          createEntry(
                            candidate.entries.length,
                            current.schemaVersion,
                          ),
                        ],
                      }
                    : candidate,
                ),
              }))
            }
          >
            + Add another item
          </button>
        </section>
      ))}
    </div>
  );
}

function CanvasEntry({
  entry,
  disabled,
  onChange,
  onRemove,
}: {
  entry: ResumeEntry;
  disabled: boolean;
  onChange: (entry: ResumeEntry) => void;
  onRemove: () => void;
}) {
  const detailsFields = fieldsForRole(entry, "details");
  const extraFields = fieldsForRole(entry, "extra");
  const paragraphFields = fieldsForRole(entry, "paragraph");
  const paragraphField = paragraphFields[0];
  function updateVisibleField(
    field: ResumeEntry["fields"][number] | undefined,
    label: string,
    value: string,
  ) {
    onChange(updateEntryField(entry, field, label, value));
  }
  function setBodyMode(mode: "bullets" | "paragraph") {
    if (mode === "paragraph") {
      const value = [
        ...paragraphFields.map((field) => field.value),
        ...entry.bullets.map((bullet) => bullet.text),
      ].join("\n");
      const nextParagraph = paragraphField
        ? { ...paragraphField, value }
        : {
            ...createNamedField(entry.fields.length),
            label: PARAGRAPH_FIELD_LABEL,
            value,
          };
      const fields = [
        ...entry.fields.filter(
          (field) => field.label !== PARAGRAPH_FIELD_LABEL,
        ),
        nextParagraph,
      ].map((field, order) => ({ ...field, order }));
      onChange({ ...entry, fields, bullets: [] });
      return;
    }
    const value = [
      ...paragraphFields.map((field) => field.value),
      ...entry.bullets.map((bullet) => bullet.text),
    ].join("\n");
    onChange({
      ...entry,
      fields: entry.fields
        .filter((field) => field.label !== PARAGRAPH_FIELD_LABEL)
        .map((field, order) => ({ ...field, order })),
      bullets: value.split("\n").map((text, order) => ({
        ...createBullet(order),
        text,
      })),
    });
  }
  return (
    <article className="canvas-entry">
      <ConfirmRemoval
        label="Remove item"
        description="Remove this item and all of its information?"
        onRemove={onRemove}
      />
      <div className="canvas-entry__primary">
        <div className="canvas-entry__title-line">
          <CanvasField
            label="Title"
            value={entry.heading}
            bold
            disabled={disabled}
            onChange={(value) => onChange({ ...entry, heading: value })}
          />
          <span className="entry-title-separator" aria-hidden="true">
            |
          </span>
          {(detailsFields.length ? detailsFields : [undefined]).map(
            (field, index) => (
              <CanvasField
                key={field?.id ?? "details"}
                label={
                  index === 0 ? "Skills / details" : field?.label || "Details"
                }
                value={field?.value ?? ""}
                disabled={disabled}
                onChange={(value) =>
                  updateVisibleField(field, "Details", value)
                }
              />
            ),
          )}
        </div>
        <CanvasField
          label="Role"
          value={entry.subheading}
          multiline
          disabled={disabled}
          onChange={(value) => onChange({ ...entry, subheading: value })}
        />
      </div>
      <div className="canvas-entry__dates">
        <CanvasDateField
          entry={entry}
          disabled={disabled}
          onChange={onChange}
        />
        <CanvasField
          label="Location"
          value={entry.location}
          disabled={disabled}
          onChange={(value) => onChange({ ...entry, location: value })}
        />
        {(extraFields.length ? extraFields : [undefined]).map((field) => (
          <CanvasField
            key={field?.id ?? "extra"}
            label="Extra"
            value={field?.value ?? ""}
            disabled={disabled}
            onChange={(value) => updateVisibleField(field, "Extra", value)}
          />
        ))}
      </div>
      <CanvasField
        label="Information"
        value={
          paragraphField?.value ??
          entry.bullets.map((bullet) => bullet.text).join("\n")
        }
        multiline
        bulk
        bulkMode={paragraphField ? "paragraph" : "bullets"}
        onBulkModeChange={setBodyMode}
        disabled={disabled}
        onChange={(value) => {
          if (paragraphField) {
            onChange({
              ...entry,
              fields: entry.fields.map((field) =>
                field.id === paragraphField.id ? { ...field, value } : field,
              ),
            });
            return;
          }
          onChange({
            ...entry,
            bullets: value.split("\n").map((text, index) => ({
              ...(entry.bullets[index] ?? createBullet(index)),
              text,
              order: index,
            })),
          });
        }}
      />
      {paragraphFields.slice(1).map((field, index) => (
        <CanvasField
          key={field.id}
          label={`Information ${index + 2}`}
          value={field.value}
          multiline
          bulk
          bulkMode="paragraph"
          disabled={disabled}
          onChange={(value) =>
            onChange(
              updateEntryField(entry, field, PARAGRAPH_FIELD_LABEL, value),
            )
          }
        />
      ))}
      {paragraphField && entry.bullets.length > 0 && (
        <CanvasField
          label="Additional information"
          value={entry.bullets.map((bullet) => bullet.text).join("\n")}
          multiline
          bulk
          bulkMode="bullets"
          disabled={disabled}
          onChange={(value) =>
            onChange({
              ...entry,
              bullets: value.split("\n").map((text, order) => ({
                ...(entry.bullets[order] ?? createBullet(order)),
                text,
                order,
              })),
            })
          }
        />
      )}
    </article>
  );
}
