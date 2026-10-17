import { useId } from "react";
import { DOCUMENT_LIMITS, type Link } from "@ort/contracts/resume";
import { safeInlineHref } from "./FormattedText";

// Both inputs belong to the document draft, including incomplete addresses.
// Normal validation blocks persistence while dirty/undo/close state stays exact.
export function ContactLinkField({
  link,
  index,
  disabled,
  onChange,
  onRemove,
}: {
  link: Link;
  index: number;
  disabled: boolean;
  onChange: (link: Link) => void;
  onRemove: () => void;
}) {
  const errorId = useId();
  const labelTooLong = [...link.label].length > DOCUMENT_LIMITS.fieldCharacters;
  const addressInvalid = !safeInlineHref(link.url);
  const addressTooLong = [...link.url].length > DOCUMENT_LIMITS.fieldCharacters;
  const invalid = labelTooLong || addressInvalid || addressTooLong;
  return (
    <div
      className="contact-information-editor__item"
      role="group"
      aria-label={`Contact link ${index + 1}`}
    >
      <div className="contact-information-editor__link-fields">
        <label>
          Label
          <input
            aria-label={`Contact link ${index + 1} label`}
            value={link.label}
            disabled={disabled}
            aria-invalid={labelTooLong || undefined}
            aria-describedby={labelTooLong ? errorId : undefined}
            onChange={(event) =>
              onChange({ ...link, label: event.target.value })
            }
          />
        </label>
        <label>
          Address
          <input
            aria-label={`Contact link ${index + 1} address`}
            value={link.url}
            disabled={disabled}
            aria-invalid={addressInvalid || addressTooLong || undefined}
            aria-describedby={
              addressInvalid || addressTooLong ? errorId : undefined
            }
            onChange={(event) => onChange({ ...link, url: event.target.value })}
          />
        </label>
      </div>
      <button
        type="button"
        className="contact-information-editor__delete button--quiet"
        aria-label={`Delete contact link ${index + 1}`}
        title="Delete contact link"
        disabled={disabled}
        onClick={onRemove}
      >
        ×
      </button>
      {invalid && (
        <span id={errorId} className="field-error" role="alert">
          {labelTooLong || addressTooLong
            ? `Use at most ${DOCUMENT_LIMITS.fieldCharacters} characters in each field.`
            : "Enter a complete HTTP, HTTPS, or mailto address."}
        </span>
      )}
    </div>
  );
}
