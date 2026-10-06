import { useEffect, useRef, useState } from "react";
import { FormattedText } from "./FormattedText";
import {
  linkSelection,
  toggleBoldSelection,
  toggleItalicSelection,
} from "./inline-formatting";
export function CanvasField({
  label,
  value,
  onChange,
  disabled,
  multiline = false,
  bulk = false,
  bulkMode = "bullets",
  onBulkModeChange,
  bold = false,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  disabled: boolean;
  multiline?: boolean;
  bulk?: boolean;
  bulkMode?: "bullets" | "paragraph";
  onBulkModeChange?: (mode: "bullets" | "paragraph") => void;
  bold?: boolean;
}) {
  const [editing, setEditing] = useState(false);
  const [linkUrl, setLinkUrl] = useState("");
  const [linkOpen, setLinkOpen] = useState(false);
  const [selection, setSelection] = useState({ start: 0, end: 0 });
  const [linkSelectionRange, setLinkSelectionRange] = useState<{
    start: number;
    end: number;
  } | null>(null);
  const [typingFormat, setTypingFormat] = useState({
    bold: false,
    italic: false,
  });
  const editButton = useRef<HTMLButtonElement>(null);
  const editor = useRef<HTMLInputElement | HTMLTextAreaElement>(null);
  const container = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!editing) return;
    function closeOnOutsideClick(event: PointerEvent) {
      if (!container.current?.contains(event.target as Node)) {
        setEditing(false);
        setLinkOpen(false);
        setTypingFormat({ bold: false, italic: false });
      }
    }
    window.addEventListener("pointerdown", closeOnOutsideClick);
    const frame = window.requestAnimationFrame?.(() => {
      container.current?.scrollIntoView?.({ block: "nearest" });
    });
    return () => {
      if (frame !== undefined) window.cancelAnimationFrame?.(frame);
      window.removeEventListener("pointerdown", closeOnOutsideClick);
    };
  }, [editing, linkOpen]);
  const hasSelection = selection.end > selection.start;
  const bullets = bulk && bulkMode === "bullets";
  const selectedText = value.slice(selection.start, selection.end);
  const selectionIsBold =
    (selectedText.startsWith("**") && selectedText.endsWith("**")) ||
    (selection.start >= 2 &&
      value.slice(selection.start - 2, selection.start) === "**" &&
      value.slice(selection.end, selection.end + 2) === "**");
  const selectionIsItalic =
    (selectedText.startsWith("*") &&
      selectedText.endsWith("*") &&
      !selectedText.startsWith("**")) ||
    (selection.start >= 1 &&
      value.slice(selection.start - 1, selection.start) === "*" &&
      value.slice(selection.end, selection.end + 1) === "*" &&
      value.slice(selection.start - 2, selection.start) !== "**");
  const boldPressed = hasSelection ? selectionIsBold : typingFormat.bold;
  const italicPressed = hasSelection ? selectionIsItalic : typingFormat.italic;
  const linkIsValid = (() => {
    try {
      return ["http:", "https:", "mailto:"].includes(new URL(linkUrl).protocol);
    } catch {
      return false;
    }
  })();
  function restoreSelection(start: number, end: number) {
    window.requestAnimationFrame(() => {
      editor.current?.focus();
      editor.current?.setSelectionRange(start, end);
      setSelection({ start, end });
    });
  }
  function toggleTypingFormat(kind: "bold" | "italic") {
    const marker = kind === "bold" ? "**" : "*";
    const active = typingFormat[kind];
    const caret = selection.start;
    if (active) {
      if (value.slice(caret, caret + marker.length) === marker) {
        restoreSelection(caret + marker.length, caret + marker.length);
      } else {
        onChange(value.slice(0, caret) + marker + value.slice(caret));
        restoreSelection(caret + marker.length, caret + marker.length);
      }
      setTypingFormat((current) => ({ ...current, [kind]: false }));
      return;
    }
    onChange(
      value.slice(0, caret) + marker + marker + value.slice(selection.end),
    );
    restoreSelection(caret + marker.length, caret + marker.length);
    setTypingFormat((current) => ({ ...current, [kind]: true }));
  }
  function decorate(kind: "bold" | "italic" | "link") {
    const { start, end } =
      kind === "link" && linkSelectionRange ? linkSelectionRange : selection;
    if (kind === "bold") {
      if (end === start) {
        toggleTypingFormat("bold");
        return;
      }
      const result = toggleBoldSelection(value, start, end);
      if (!result) return;
      onChange(result.value);
      restoreSelection(result.selectionStart, result.selectionEnd);
      setTypingFormat((current) => ({ ...current, bold: false }));
      return;
    }
    if (kind === "italic") {
      if (end === start) {
        toggleTypingFormat("italic");
        return;
      }
      const result = toggleItalicSelection(value, start, end);
      if (!result) return;
      onChange(result.value);
      restoreSelection(result.selectionStart, result.selectionEnd);
      setTypingFormat((current) => ({ ...current, italic: false }));
      return;
    }
    const result = linkSelection(value, start, end, linkUrl);
    if (!result) return;
    onChange(result.value);
    restoreSelection(result.selectionStart, result.selectionEnd);
    setLinkUrl("");
    setLinkOpen(false);
    setLinkSelectionRange(null);
  }
  function finishEditing() {
    setEditing(false);
    setLinkOpen(false);
    setLinkSelectionRange(null);
    setTypingFormat({ bold: false, italic: false });
    window.requestAnimationFrame(() => editButton.current?.focus());
  }
  function handleEditorKey(
    event: React.KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>,
  ) {
    if (event.key === "Escape") {
      event.preventDefault();
      finishEditing();
    } else if (
      (event.metaKey || event.ctrlKey) &&
      ["b", "i"].includes(event.key.toLowerCase())
    ) {
      event.preventDefault();
      decorate(event.key.toLowerCase() === "b" ? "bold" : "italic");
    }
  }
  function rememberSelection() {
    const target = editor.current;
    setSelection({
      start: target?.selectionStart ?? 0,
      end: target?.selectionEnd ?? 0,
    });
  }
  return (
    <div
      ref={container}
      className={`canvas-field${!value ? " canvas-field--empty" : ""}${editing ? " canvas-field--editing" : ""}${multiline ? " canvas-field--multiline" : " canvas-field--singleline"}${bold ? " canvas-field--bold" : ""}${bullets ? " canvas-field--bullets" : ""}`}
    >
      {editing ? (
        <>
          <div className="canvas-field__toolbar">
            <span>{label}</span>
            <span className="canvas-field__formatting">
              <button
                type="button"
                className="button--quiet button--compact canvas-format-clear"
                aria-label={`Clear ${label.toLowerCase()}`}
                title="Clear all text"
                disabled={disabled || !value}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => {
                  onChange("");
                  setTypingFormat({ bold: false, italic: false });
                  setLinkOpen(false);
                  setLinkSelectionRange(null);
                  restoreSelection(0, 0);
                }}
              >
                <span aria-hidden="true">×</span>
              </button>
              <button
                type="button"
                className="button--quiet button--compact"
                aria-label={boldPressed ? "Turn bold off" : "Turn bold on"}
                title={boldPressed ? "Turn bold off" : "Bold"}
                disabled={disabled}
                aria-pressed={boldPressed}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => decorate("bold")}
              >
                <strong aria-hidden="true">B</strong>
              </button>
              <button
                type="button"
                className="button--quiet button--compact canvas-format-italic"
                aria-label={
                  italicPressed ? "Turn italics off" : "Turn italics on"
                }
                title={italicPressed ? "Turn italics off" : "Italic"}
                disabled={disabled}
                aria-pressed={italicPressed}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => decorate("italic")}
              >
                <em aria-hidden="true">I</em>
              </button>
              <button
                type="button"
                className="button--quiet button--compact"
                onMouseDown={(event) => event.preventDefault()}
                disabled={disabled}
                aria-expanded={linkOpen}
                onClick={() => {
                  if (!linkOpen) setLinkSelectionRange(selection);
                  else setLinkSelectionRange(null);
                  setLinkOpen((open) => !open);
                }}
              >
                Link
              </button>
            </span>
            {bulk ? (
              <button
                type="button"
                className="button--quiet button--compact"
                onClick={() =>
                  onBulkModeChange?.(bullets ? "paragraph" : "bullets")
                }
              >
                {bullets ? "Paragraph" : "Bullet points"}
              </button>
            ) : null}
          </div>
          {linkOpen ? (
            <div className="canvas-field__link-popover">
              <span className="canvas-field__link-selection">
                Link text:{" "}
                <mark>
                  {linkSelectionRange &&
                  linkSelectionRange.end > linkSelectionRange.start
                    ? value.slice(
                        linkSelectionRange.start,
                        linkSelectionRange.end,
                      )
                    : "Enter Text Here"}
                </mark>
              </span>
              <input
                autoFocus
                type="url"
                aria-label="Link address"
                value={linkUrl}
                placeholder={"https:" + "//example.com"}
                onFocus={() => {
                  if (linkSelectionRange)
                    editor.current?.setSelectionRange(
                      linkSelectionRange.start,
                      linkSelectionRange.end,
                    );
                }}
                onChange={(event) => setLinkUrl(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && linkIsValid) {
                    event.preventDefault();
                    decorate("link");
                  }
                  if (event.key === "Escape") setLinkOpen(false);
                }}
              />
              <button
                type="button"
                disabled={!linkIsValid}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => decorate("link")}
              >
                Apply link
              </button>
            </div>
          ) : null}
          {multiline ? (
            <textarea
              ref={editor as React.RefObject<HTMLTextAreaElement>}
              className={`${typingFormat.bold ? "is-typing-bold" : ""}${typingFormat.italic ? " is-typing-italic" : ""}`}
              autoFocus
              aria-label={label}
              value={value}
              disabled={disabled}
              placeholder={
                bullets ? "One bullet per line" : `Add ${label.toLowerCase()}`
              }
              onChange={(event) => onChange(event.target.value)}
              onKeyDown={handleEditorKey}
              onSelect={rememberSelection}
              onMouseUp={rememberSelection}
              onKeyUp={rememberSelection}
            />
          ) : (
            <input
              ref={editor as React.RefObject<HTMLInputElement>}
              className={`${typingFormat.bold ? "is-typing-bold" : ""}${typingFormat.italic ? " is-typing-italic" : ""}`}
              autoFocus
              aria-label={label}
              value={value}
              disabled={disabled}
              placeholder={`Add ${label.toLowerCase()}`}
              onChange={(event) => onChange(event.target.value)}
              onKeyDown={handleEditorKey}
              onSelect={rememberSelection}
              onMouseUp={rememberSelection}
              onKeyUp={rememberSelection}
            />
          )}
        </>
      ) : (
        <button
          type="button"
          ref={editButton}
          className="canvas-field__button"
          disabled={disabled}
          onClick={() => setEditing(true)}
        >
          <span className="canvas-field__label">{label}</span>
          <span
            className={`canvas-field__value${bulk && bullets ? " canvas-field__value--bullets" : ""}`}
          >
            {value ? (
              <FormattedText value={value} bullets={bulk && bullets} />
            ) : (
              `Add ${label.toLowerCase()}`
            )}
          </span>
        </button>
      )}
    </div>
  );
}
