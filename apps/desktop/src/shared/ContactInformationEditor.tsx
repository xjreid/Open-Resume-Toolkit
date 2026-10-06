import { useEffect, useRef, useState } from "react";
import type { ResumeDocument } from "@ort/contracts/resume";
import type { ContactDivider } from "./resume-view-types";
import { FormattedText, safeInlineHref } from "./FormattedText";
import {
  linkSelection,
  toggleBoldSelection,
  toggleItalicSelection,
} from "./inline-formatting";
export function ContactInformationEditor({
  contact,
  divider,
  disabled,
  onChange,
  onDividerChange,
  showDivider = true,
}: {
  contact: ResumeDocument["contact"];
  divider: ContactDivider;
  disabled: boolean;
  onChange: (contact: ResumeDocument["contact"]) => void;
  onDividerChange: (divider: ContactDivider) => void;
  showDivider?: boolean;
}) {
  const details = useRef<HTMLDetailsElement>(null);
  const initialItems = [
    contact.email,
    contact.phone,
    ...contact.location.split("\n"),
  ].filter((value) => value.length > 0);
  const [items, setItems] = useState<string[]>(() =>
    initialItems.length ? initialItems : [""],
  );
  const [activeIndex, setActiveIndex] = useState(0);
  const [selection, setSelection] = useState({ start: 0, end: 0 });
  const [typingFormat, setTypingFormat] = useState({
    bold: false,
    italic: false,
  });
  const [linkOpen, setLinkOpen] = useState(false);
  const [linkUrl, setLinkUrl] = useState("");
  const [linkRange, setLinkRange] = useState({ start: 0, end: 0 });
  const inputs = useRef<Array<HTMLInputElement | null>>([]);
  const activeValue = items[activeIndex] ?? "";
  const linkIsValid = safeInlineHref(linkUrl);

  useEffect(() => {
    const currentContact = {
      email: items[0] ?? "",
      phone: items[1] ?? "",
      location: items.slice(2).join("\n"),
    };
    if (
      currentContact.email === contact.email &&
      currentContact.phone === contact.phone &&
      currentContact.location === contact.location
    )
      return;
    const externalItems = [
      contact.email,
      contact.phone,
      ...contact.location.split("\n"),
    ].filter((value) => value.length > 0);
    setItems(externalItems.length ? externalItems : [""]);
  }, [contact.email, contact.phone, contact.location]);

  function commit(nextItems: string[]) {
    setItems(nextItems);
    onChange({
      ...contact,
      email: nextItems[0] ?? "",
      phone: nextItems[1] ?? "",
      location: nextItems.slice(2).join("\n"),
    });
  }
  function updateItem(index: number, value: string) {
    const next = [...items];
    next[index] = value;
    commit(next);
  }
  function restoreSelection(index: number, start: number, end: number) {
    window.requestAnimationFrame(() => {
      inputs.current[index]?.focus();
      inputs.current[index]?.setSelectionRange(start, end);
      setActiveIndex(index);
      setSelection({ start, end });
    });
  }
  function applyFormat(kind: "bold" | "italic") {
    const marker = kind === "bold" ? "**" : "*";
    const toggle =
      kind === "bold" ? toggleBoldSelection : toggleItalicSelection;
    if (selection.end > selection.start) {
      const result = toggle(activeValue, selection.start, selection.end);
      if (!result) return;
      updateItem(activeIndex, result.value);
      restoreSelection(activeIndex, result.selectionStart, result.selectionEnd);
      return;
    }
    const active = typingFormat[kind];
    const caret = selection.start;
    if (active && activeValue.slice(caret, caret + marker.length) === marker) {
      restoreSelection(
        activeIndex,
        caret + marker.length,
        caret + marker.length,
      );
      setTypingFormat((current) => ({ ...current, [kind]: false }));
      return;
    }
    const nextValue = active
      ? activeValue.slice(0, caret) + marker + activeValue.slice(caret)
      : activeValue.slice(0, caret) +
        marker +
        marker +
        activeValue.slice(caret);
    updateItem(activeIndex, nextValue);
    restoreSelection(activeIndex, caret + marker.length, caret + marker.length);
    setTypingFormat((current) => ({ ...current, [kind]: !active }));
  }
  function applyLink() {
    const result = linkSelection(
      activeValue,
      linkRange.start,
      linkRange.end,
      linkUrl,
    );
    if (!result) return;
    updateItem(activeIndex, result.value);
    setLinkOpen(false);
    setLinkUrl("");
    restoreSelection(activeIndex, result.selectionStart, result.selectionEnd);
  }

  return (
    <>
      <details ref={details} className="contact-information-details">
        <summary aria-label="Edit contact information">
          <span>
            {items.filter(Boolean).map((item, index) => (
              <span key={index}>
                {index > 0
                  ? divider === "bar"
                    ? " | "
                    : divider === "dash"
                      ? " - "
                      : " • "
                  : ""}
                <FormattedText value={item} />
              </span>
            ))}
            {!items.some(Boolean) ? "Add contact information" : null}
          </span>
          <svg
            className="workspace-icon"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path d="m16 3 5 5-13 13H3v-5zM13 6l5 5" />
          </svg>
        </summary>
        <section className="contact-information-editor">
          <div className="contact-information-editor__toolbar">
            <span>Contact information</span>
            <div className="canvas-field__formatting">
              <button
                type="button"
                className="button--quiet button--compact"
                aria-label="Bold contact text"
                aria-pressed={typingFormat.bold}
                disabled={disabled}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => applyFormat("bold")}
              >
                <strong aria-hidden="true">B</strong>
              </button>
              <button
                type="button"
                className="button--quiet button--compact canvas-format-italic"
                aria-label="Italicize contact text"
                aria-pressed={typingFormat.italic}
                disabled={disabled}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => applyFormat("italic")}
              >
                <em aria-hidden="true">I</em>
              </button>
              <button
                type="button"
                className="button--quiet button--compact"
                aria-expanded={linkOpen}
                disabled={disabled}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => {
                  setLinkRange(selection);
                  setLinkOpen((open) => !open);
                }}
              >
                Link
              </button>
            </div>
          </div>
          {linkOpen ? (
            <div className="contact-information-editor__link">
              <span>
                Link text:{" "}
                {linkRange.end > linkRange.start
                  ? activeValue.slice(linkRange.start, linkRange.end)
                  : "Enter Text Here"}
              </span>
              <input
                autoFocus
                type="url"
                aria-label="Contact link address"
                value={linkUrl}
                placeholder={"https:" + "//example.com"}
                onChange={(event) => setLinkUrl(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && linkIsValid) {
                    event.preventDefault();
                    applyLink();
                  }
                }}
              />
              <button type="button" disabled={!linkIsValid} onClick={applyLink}>
                Apply
              </button>
            </div>
          ) : null}
          <div className="contact-information-editor__items">
            {items.map((value, index) => (
              <div className="contact-information-editor__item" key={index}>
                <input
                  ref={(node) => {
                    inputs.current[index] = node;
                  }}
                  aria-label={`Contact information ${index + 1}`}
                  value={value}
                  disabled={disabled}
                  className={`${activeIndex === index && typingFormat.bold ? "is-typing-bold" : ""}${activeIndex === index && typingFormat.italic ? " is-typing-italic" : ""}`}
                  placeholder="Email, phone, location, portfolio…"
                  onFocus={(event) => {
                    setActiveIndex(index);
                    setSelection({
                      start: event.currentTarget.selectionStart ?? 0,
                      end: event.currentTarget.selectionEnd ?? 0,
                    });
                    setTypingFormat({ bold: false, italic: false });
                  }}
                  onSelect={(event) => {
                    setActiveIndex(index);
                    setSelection({
                      start: event.currentTarget.selectionStart ?? 0,
                      end: event.currentTarget.selectionEnd ?? 0,
                    });
                  }}
                  onChange={(event) => updateItem(index, event.target.value)}
                />
                <button
                  type="button"
                  className="contact-information-editor__delete button--quiet"
                  aria-label={`Delete contact information ${index + 1}`}
                  title="Delete contact"
                  disabled={disabled}
                  onClick={() => {
                    const next = items.filter(
                      (_, itemIndex) => itemIndex !== index,
                    );
                    commit(next.length ? next : [""]);
                    setActiveIndex(Math.max(0, index - 1));
                  }}
                >
                  ×
                </button>
              </div>
            ))}
          </div>
        </section>
      </details>
      <div className="contact-information-actions">
        <button
          type="button"
          className="contact-information-editor__add button--secondary button--compact"
          disabled={disabled}
          onClick={() => {
            if (details.current) details.current.open = true;
            commit([...items, ""]);
            setActiveIndex(items.length);
            window.requestAnimationFrame(() =>
              inputs.current[items.length]?.focus(),
            );
          }}
        >
          + Add contact information
        </button>
        {showDivider ? (
          <label className="contact-divider-control">
            Contact divider
            <select
              value={divider}
              disabled={disabled}
              onChange={(event) =>
                onDividerChange(
                  event.target.value === "bar"
                    ? "bar"
                    : event.target.value === "dash"
                      ? "dash"
                      : "dot",
                )
              }
            >
              <option value="dot">Dot •</option>
              <option value="bar">Bar |</option>
              <option value="dash">Dash -</option>
            </select>
          </label>
        ) : null}
      </div>
    </>
  );
}
