import { useEffect, useRef, useState } from "react";
import { DOCUMENT_LIMITS, type ResumeDocument } from "@ort/contracts/resume";
import { createSection, moveItem } from "./resume-editor";
import { SUGGESTED_SECTIONS } from "./starting-profiles";

export function PopupSectionNavigator({
  document,
  disabled,
  onChange,
  accessibleControls = false,
}: {
  document: ResumeDocument;
  disabled: boolean;
  accessibleControls?: boolean;
  onChange: (document: ResumeDocument) => void;
}) {
  const [suggestedSection, setSuggestedSection] = useState("Custom Section");
  const [renaming, setRenaming] = useState<string | null>(null);
  const [dragging, setDragging] = useState<string | null>(null);
  const [dragOrder, setDragOrder] = useState<string[] | null>(null);
  const [ghost, setGhost] = useState<{
    x: number;
    y: number;
    label: string;
  } | null>(null);
  const [trashActive, setTrashActive] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<string | null>(null);
  const documentRef = useRef(document);
  documentRef.current = document;
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;
  const orderRef = useRef<string[] | null>(null);
  const navigator = useRef<HTMLElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const trash = useRef<HTMLDivElement>(null);
  const cards = useRef(new Map<string, HTMLDivElement>());
  const gesture = useRef<{
    id: string;
    pointerId: number;
    startX: number;
    startY: number;
    offsetX: number;
    offsetY: number;
    width: number;
    height: number;
    dragging: boolean;
  } | null>(null);

  useEffect(() => {
    function inside(element: HTMLElement | null, x: number, y: number) {
      if (!element) return false;
      const bounds = element.getBoundingClientRect();
      return (
        x >= bounds.left &&
        x <= bounds.right &&
        y >= bounds.top &&
        y <= bounds.bottom
      );
    }
    function clear() {
      gesture.current = null;
      orderRef.current = null;
      setDragOrder(null);
      setDragging(null);
      setGhost(null);
      setTrashActive(false);
      window.document.body.classList.remove("is-section-sorting");
    }
    function move(event: PointerEvent) {
      const current = gesture.current;
      if (!current || current.pointerId !== event.pointerId || disabled) return;
      if (!current.dragging) {
        if (
          Math.hypot(
            event.clientX - current.startX,
            event.clientY - current.startY,
          ) < 6
        )
          return;
        current.dragging = true;
        orderRef.current = documentRef.current.sections.map(
          (section) => section.id,
        );
        setDragOrder(orderRef.current);
        setDragging(current.id);
        window.document.body.classList.add("is-section-sorting");
      }
      event.preventDefault();
      const nav = navigator.current?.getBoundingClientRect();
      const listBounds = list.current?.getBoundingClientRect();
      if (!nav || !listBounds) return;
      if (event.clientY < nav.top + 28) navigator.current!.scrollTop -= 8;
      else if (event.clientY > nav.bottom - 28)
        navigator.current!.scrollTop += 8;
      setGhost({
        x: Math.min(
          Math.max(event.clientX - current.offsetX, nav.left + 8),
          nav.right - current.width - 8,
        ),
        y: Math.min(
          Math.max(
            event.clientY - current.offsetY,
            Math.max(nav.top + 8, listBounds.top),
          ),
          nav.bottom - current.height - 8,
        ),
        label:
          documentRef.current.sections.find(
            (section) => section.id === current.id,
          )?.heading || "Untitled section",
      });
      const overTrash = inside(trash.current, event.clientX, event.clientY);
      setTrashActive(overTrash);
      if (overTrash || !orderRef.current) return;
      const others = orderRef.current.filter((id) => id !== current.id);
      let destination = others.length;
      for (let index = 0; index < others.length; index += 1) {
        const card = cards.current.get(others[index]);
        if (
          card &&
          event.clientY <
            card.getBoundingClientRect().top + card.offsetHeight / 2
        ) {
          destination = index;
          break;
        }
      }
      const next = [...others];
      next.splice(destination, 0, current.id);
      if (!next.every((id, index) => id === orderRef.current![index])) {
        orderRef.current = next;
        setDragOrder(next);
      }
    }
    function finish(event: PointerEvent) {
      const current = gesture.current;
      if (!current || current.pointerId !== event.pointerId) return;
      if (disabled) {
        clear();
        return;
      }
      if (current.dragging) {
        if (inside(trash.current, event.clientX, event.clientY))
          setPendingDelete(current.id);
        else if (orderRef.current) {
          const byId = new Map(
            documentRef.current.sections.map((section) => [
              section.id,
              section,
            ]),
          );
          onChangeRef.current({
            ...documentRef.current,
            sections: orderRef.current.map((id, order) => ({
              ...byId.get(id)!,
              order,
            })),
          });
        }
      } else setRenaming(current.id);
      clear();
    }
    function cancel(event: PointerEvent) {
      if (gesture.current?.pointerId === event.pointerId) clear();
    }
    window.addEventListener("pointermove", move, { passive: false });
    window.addEventListener("pointerup", finish);
    window.addEventListener("pointercancel", cancel);
    window.addEventListener("blur", clear);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", finish);
      window.removeEventListener("pointercancel", cancel);
      window.removeEventListener("blur", clear);
      window.document.body.classList.remove("is-section-sorting");
    };
  }, [disabled]);

  const ordered = (dragOrder ?? document.sections.map((section) => section.id))
    .map((id) => document.sections.find((section) => section.id === id))
    .filter((section) => section !== undefined);
  return (
    <nav
      ref={navigator}
      className={`application-popup__navigator document-navigator${dragging ? " document-navigator--sorting" : ""}`}
      aria-label="Resume section navigation"
    >
      <div>
        <div className="contact-nav-item">
          <span>Contact</span>
        </div>
        <div ref={list} className="section-sort-list">
          {ordered.map((section, index) => (
            <div
              key={section.id}
              ref={(card) => {
                if (card) cards.current.set(section.id, card);
                else cards.current.delete(section.id);
              }}
              className={`section-nav-card${dragging === section.id ? " section-nav-card--dragging" : ""}`}
              onPointerDown={(event) => {
                if (
                  disabled ||
                  event.button !== 0 ||
                  renaming === section.id ||
                  (event.target as HTMLElement).closest("input, button")
                )
                  return;
                event.preventDefault();
                const bounds = event.currentTarget.getBoundingClientRect();
                gesture.current = {
                  id: section.id,
                  pointerId: event.pointerId,
                  startX: event.clientX,
                  startY: event.clientY,
                  offsetX: event.clientX - bounds.left,
                  offsetY: event.clientY - bounds.top,
                  width: bounds.width,
                  height: bounds.height,
                  dragging: false,
                };
              }}
            >
              <div className="section-nav-row">
                {renaming === section.id ? (
                  <input
                    autoFocus
                    aria-label={`Section name ${section.heading || "untitled"}`}
                    disabled={disabled}
                    value={section.heading}
                    onBlur={() => setRenaming(null)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter" || event.key === "Escape")
                        event.currentTarget.blur();
                    }}
                    onChange={(event) =>
                      onChange({
                        ...document,
                        sections: document.sections.map((item) =>
                          item.id === section.id
                            ? { ...item, heading: event.target.value }
                            : item,
                        ),
                      })
                    }
                  />
                ) : accessibleControls ? (
                  <button
                    type="button"
                    className="section-nav-title button--quiet"
                    disabled={disabled}
                    onClick={() => setRenaming(section.id)}
                    aria-label={`Rename ${section.heading || "untitled section"}`}
                  >
                    {section.heading || "Untitled section"}
                  </button>
                ) : (
                  <span className="section-nav-title">
                    {section.heading || "Untitled section"}
                  </span>
                )}
              </div>
              {accessibleControls && (
                <div className="import-section-actions">
                  <button
                    type="button"
                    className="button--quiet button--compact"
                    aria-label={`Move ${section.heading} up`}
                    disabled={disabled || index === 0}
                    onClick={() =>
                      onChange({
                        ...document,
                        sections: moveItem(document.sections, section.id, -1),
                      })
                    }
                  >
                    ↑
                  </button>
                  <button
                    type="button"
                    className="button--quiet button--compact"
                    aria-label={`Move ${section.heading} down`}
                    disabled={disabled || index === ordered.length - 1}
                    onClick={() =>
                      onChange({
                        ...document,
                        sections: moveItem(document.sections, section.id, 1),
                      })
                    }
                  >
                    ↓
                  </button>
                  <button
                    type="button"
                    className="button--quiet button--compact"
                    aria-label={`Delete ${section.heading}`}
                    disabled={disabled}
                    onClick={() => setPendingDelete(section.id)}
                  >
                    ×
                  </button>
                </div>
              )}
            </div>
          ))}
        </div>
        {ghost && (
          <div
            className="section-drag-ghost"
            style={{ left: ghost.x, top: ghost.y }}
            aria-hidden="true"
          >
            {ghost.label}
          </div>
        )}
        <div
          ref={trash}
          className={`section-trash${trashActive ? " section-trash--active" : ""}`}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M8 4h8l1 2h4v2H3V6h4l1-2Zm-2 6h12l-1 10H7L6 10Zm3 2v6h2v-6H9Zm4 0v6h2v-6h-2Z" />
          </svg>
          <span>Drag a section here to delete</span>
        </div>
        {pendingDelete && (
          <div
            className="section-trash-confirmation"
            role="group"
            aria-label="Confirm section deletion"
          >
            <strong>Delete this section?</strong>
            <p>
              {document.sections.find((section) => section.id === pendingDelete)
                ?.heading || "This section"}{" "}
              and all of its items will be removed.
            </p>
            <div>
              <button
                type="button"
                className="button--secondary button--compact"
                onClick={() => setPendingDelete(null)}
              >
                Cancel
              </button>
              <button
                type="button"
                className="button--danger button--compact"
                disabled={disabled}
                onClick={() => {
                  onChange({
                    ...document,
                    sections: document.sections
                      .filter((section) => section.id !== pendingDelete)
                      .map((section, order) => ({ ...section, order })),
                  });
                  setPendingDelete(null);
                }}
              >
                Delete section
              </button>
            </div>
          </div>
        )}
        <div className="section-add-control">
          <label>
            Add section
            <select
              value={suggestedSection}
              disabled={disabled}
              onChange={(event) => setSuggestedSection(event.target.value)}
            >
              {SUGGESTED_SECTIONS.map((heading) => (
                <option key={heading} value={heading}>
                  {heading}
                </option>
              ))}
            </select>
          </label>
          <button
            type="button"
            className="button--secondary button--compact"
            disabled={
              disabled || document.sections.length >= DOCUMENT_LIMITS.sections
            }
            onClick={() =>
              onChange({
                ...document,
                sections: [
                  ...document.sections,
                  {
                    ...createSection(document.sections.length),
                    heading: suggestedSection,
                  },
                ],
              })
            }
          >
            Add
          </button>
        </div>
      </div>
    </nav>
  );
}
