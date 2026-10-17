import { useEffect, useId, useState } from "react";
import type { Link } from "@ort/contracts/resume";
import { safeInlineHref } from "./FormattedText";

// Keep partial edits local. A malformed address must not replace a valid saved
// link while the user types the editor's normal [label](URL) notation.
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
  const original = `[${link.label || link.url}](${link.url})`;
  const [value, setValue] = useState(original);
  const [invalid, setInvalid] = useState(false);
  const errorId = useId();
  useEffect(() => {
    setValue(original);
    setInvalid(false);
  }, [original]);
  function commit() {
    const match = value.match(/^\[([^\]]+)\]\(([^\s]+)\)$/);
    if (
      !match ||
      !safeInlineHref(match[2]) ||
      [...match[1]].length > 2000 ||
      [...match[2]].length > 2000
    ) {
      setInvalid(true);
      return;
    }
    setInvalid(false);
    onChange({ ...link, label: match[1], url: match[2] });
  }
  return (
    <div className="contact-information-editor__item">
      <input
        aria-label={`Contact link ${index + 1}`}
        value={value}
        disabled={disabled}
        aria-invalid={invalid || undefined}
        aria-describedby={invalid ? errorId : undefined}
        onChange={(event) => {
          setValue(event.target.value);
          setInvalid(false);
        }}
        onBlur={commit}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            commit();
          }
        }}
      />
      <button
        type="button"
        className="contact-information-editor__delete button--quiet"
        aria-label={`Delete contact link ${index + 1}`}
        title="Delete contact link"
        disabled={disabled}
        onMouseDown={(event) => event.preventDefault()}
        onClick={onRemove}
      >
        ×
      </button>
      {invalid && (
        <span id={errorId} className="field-error" role="alert">
          Use [label](address) with an http, https, or mailto address.
        </span>
      )}
    </div>
  );
}
