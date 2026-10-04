import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import {
  DOCUMENT_LIMITS,
  type ResumeDocument,
  type ResumeSection,
} from "@ort/contracts/resume";
import { createSection, moveItem } from "./resume-editor";
import { SUGGESTED_SECTIONS } from "./starting-profiles";

// The master editor and import review share the same section controls.
export function ResumeSectionNavigator({
  document,
  disabled,
  hidden = false,
  onChange,
}: {
  document: ResumeDocument;
  disabled: boolean;
  hidden?: boolean;
  onChange: (update: (current: ResumeDocument) => ResumeDocument) => void;
}) {
  const changeDocument = onChange;
  const [suggestedSection, setSuggestedSection] =
    useState<string>("Custom Section");
  const [renamingSection, setRenamingSection] = useState<string | null>(null);
  const [draggingSection, setDraggingSection] = useState<string | null>(null);
  const [sectionDragOrder, setSectionDragOrder] = useState<string[] | null>(
    null,
  );
  const [trashActive, setTrashActive] = useState(false);
  const [pendingTrashSection, setPendingTrashSection] = useState<string | null>(
    null,
  );
  const sectionPointerDrag = useRef<{
    id: string;
    pointerId: number;
    startX: number;
    startY: number;
    dragging: boolean;
    offsetX: number;
    offsetY: number;
    width: number;
    height: number;
  } | null>(null);
  const suppressRenameClick = useRef(false);
  const sectionDragOrderRef = useRef<string[] | null>(null);
  const sectionLayoutBefore = useRef<Map<string, DOMRect>>(new Map());
  const sectionCards = useRef<Map<string, HTMLDivElement>>(new Map());
  const sectionList = useRef<HTMLDivElement>(null);
  const sectionTrash = useRef<HTMLDivElement>(null);
  const sectionNavigator = useRef<HTMLElement>(null);
  const deleteDialog = useRef<HTMLDialogElement>(null);
  const cancelDelete = useRef<HTMLButtonElement>(null);
  const deleteFocusSection = useRef<string | null>(null);
  const dialogTitleId = useId();
  const dialogDescriptionId = useId();
  const keyboardHelpId = useId();
  const [sectionDragPosition, setSectionDragPosition] = useState<{
    x: number;
    y: number;
    label: string;
  } | null>(null);
  const pendingSection = document.sections.find(
    (section) => section.id === pendingTrashSection,
  );

  useEffect(() => {
    const dialog = deleteDialog.current;
    if (!dialog) return;
    if (pendingSection && !disabled && !hidden) {
      if (!dialog.open) {
        deleteFocusSection.current = pendingSection.id;
        dialog.showModal();
        cancelDelete.current?.focus();
      }
    } else {
      if (dialog.open) {
        dialog.close();
        const title = sectionCards.current
          .get(deleteFocusSection.current ?? "")
          ?.querySelector<HTMLButtonElement>(".section-nav-title");
        const fallback = sectionNavigator.current?.querySelector<HTMLElement>(
          ".section-nav-title:not(:disabled), select:not(:disabled)",
        );
        if (!hidden && !disabled) (title ?? fallback)?.focus();
        deleteFocusSection.current = null;
      }
      if (pendingTrashSection) setPendingTrashSection(null);
    }
  }, [pendingSection, pendingTrashSection, disabled, hidden]);
  useLayoutEffect(() => {
    if (!sectionDragOrder) return;
    for (const [id, previous] of sectionLayoutBefore.current) {
      const card = sectionCards.current.get(id);
      if (!card || id === draggingSection) continue;
      card.getAnimations().forEach((animation) => animation.cancel());
      const current = card.getBoundingClientRect();
      const delta = previous.top - current.top;
      if (Math.abs(delta) < 1) continue;
      card.animate(
        [
          { transform: `translateY(${delta}px)` },
          { transform: "translateY(0)" },
        ],
        { duration: 190, easing: "cubic-bezier(.2,.8,.2,1)" },
      );
    }
    sectionLayoutBefore.current.clear();
  }, [sectionDragOrder, draggingSection]);
  function captureSectionLayout() {
    sectionLayoutBefore.current = new Map(
      [...sectionCards.current].map(([id, card]) => [
        id,
        card.getBoundingClientRect(),
      ]),
    );
  }

  function updateSectionDragOrder(sourceId: string, clientY: number) {
    const list = sectionList.current;
    const currentOrder = sectionDragOrderRef.current;
    if (!list || !currentOrder) return;
    const otherIds = currentOrder.filter((id) => id !== sourceId);
    const localY = clientY - list.getBoundingClientRect().top;
    let destination = otherIds.length;
    for (let index = 0; index < otherIds.length; index += 1) {
      const card = sectionCards.current.get(otherIds[index]);
      if (!card) continue;
      const center = card.offsetTop - list.offsetTop + card.offsetHeight / 2;
      if (localY < center) {
        destination = index;
        break;
      }
    }
    const nextOrder = [...otherIds];
    nextOrder.splice(destination, 0, sourceId);
    if (nextOrder.every((id, index) => id === currentOrder[index])) return;
    captureSectionLayout();
    sectionDragOrderRef.current = nextOrder;
    setSectionDragOrder(nextOrder);
  }

  function commitSectionDragOrder(order: string[]) {
    changeDocument((current) => {
      const byId = new Map(
        current.sections.map((section) => [section.id, section]),
      );
      const sections = order
        .map((id) => byId.get(id))
        .filter((section): section is ResumeSection => Boolean(section))
        .map((section, sectionOrder) => ({
          ...section,
          order: sectionOrder,
        }));
      if (sections.length !== current.sections.length) return current;
      return { ...current, sections };
    });
  }

  function clearSectionDrag() {
    window.document.body.classList.remove("is-section-sorting");
    sectionPointerDrag.current = null;
    sectionDragOrderRef.current = null;
    setSectionDragOrder(null);
    setDraggingSection(null);
    setSectionDragPosition(null);
    setTrashActive(false);
  }

  useEffect(() => {
    if (disabled || hidden) {
      clearSectionDrag();
      return;
    }
    const activeDocument = document;
    function moveSectionPointer(event: PointerEvent) {
      const gesture = sectionPointerDrag.current;
      if (!gesture || gesture.pointerId !== event.pointerId) return;
      if (!gesture.dragging) {
        const distance = Math.hypot(
          event.clientX - gesture.startX,
          event.clientY - gesture.startY,
        );
        if (distance < 6) return;
        gesture.dragging = true;
        const order = activeDocument.sections.map((section) => section.id);
        sectionDragOrderRef.current = order;
        setSectionDragOrder(order);
        setDraggingSection(gesture.id);
        window.document.body.classList.add("is-section-sorting");
      }
      event.preventDefault();
      const navigatorBounds = sectionNavigator.current?.getBoundingClientRect();
      const listBounds = sectionList.current?.getBoundingClientRect();
      const trashBounds = sectionTrash.current?.getBoundingClientRect();
      if (!navigatorBounds || !listBounds || !trashBounds) return;
      const navigator = sectionNavigator.current;
      if (navigator) {
        if (event.clientY < navigatorBounds.top + 28) navigator.scrollTop -= 8;
        else if (event.clientY > navigatorBounds.bottom - 28)
          navigator.scrollTop += 8;
      }
      const minimumX = navigatorBounds.left + 8;
      const maximumX = Math.max(
        minimumX,
        navigatorBounds.right - gesture.width - 8,
      );
      const minimumY = Math.max(navigatorBounds.top + 8, listBounds.top);
      const maximumY = Math.max(
        minimumY,
        Math.min(navigatorBounds.bottom - 8, trashBounds.bottom) -
          gesture.height,
      );
      const boundedX = Math.min(
        maximumX,
        Math.max(minimumX, event.clientX - gesture.offsetX),
      );
      const boundedY = Math.min(
        maximumY,
        Math.max(minimumY, event.clientY - gesture.offsetY),
      );
      const draggedSection = activeDocument.sections.find(
        (section) => section.id === gesture.id,
      );
      setSectionDragPosition({
        x: boundedX,
        y: boundedY,
        label: draggedSection?.heading || "Untitled section",
      });
      const overTrash =
        event.clientX >= trashBounds.left &&
        event.clientX <= trashBounds.right &&
        event.clientY >= trashBounds.top &&
        event.clientY <= trashBounds.bottom;
      setTrashActive(overTrash);
      if (overTrash) return;
      updateSectionDragOrder(
        gesture.id,
        Math.min(listBounds.bottom, Math.max(listBounds.top, event.clientY)),
      );
    }

    function finishSectionPointer(event: PointerEvent) {
      const gesture = sectionPointerDrag.current;
      if (!gesture || gesture.pointerId !== event.pointerId) return;
      const trashBounds = sectionTrash.current?.getBoundingClientRect();
      const droppedOnTrash = Boolean(
        trashBounds &&
          event.clientX >= trashBounds.left &&
          event.clientX <= trashBounds.right &&
          event.clientY >= trashBounds.top &&
          event.clientY <= trashBounds.bottom,
      );
      if (gesture.dragging) {
        suppressRenameClick.current = true;
        if (droppedOnTrash) setPendingTrashSection(gesture.id);
        else if (sectionDragOrderRef.current)
          commitSectionDragOrder(sectionDragOrderRef.current);
      } else {
        setRenamingSection(gesture.id);
      }
      clearSectionDrag();
    }

    function cancelSectionPointer(event: PointerEvent) {
      if (sectionPointerDrag.current?.pointerId === event.pointerId)
        clearSectionDrag();
    }

    function cancelSectionPointerOnBlur() {
      if (sectionPointerDrag.current) clearSectionDrag();
    }

    window.addEventListener("pointermove", moveSectionPointer, {
      passive: false,
    });
    window.addEventListener("pointerup", finishSectionPointer);
    window.addEventListener("pointercancel", cancelSectionPointer);
    window.addEventListener("blur", cancelSectionPointerOnBlur);
    return () => {
      window.document.body.classList.remove("is-section-sorting");
      window.removeEventListener("pointermove", moveSectionPointer);
      window.removeEventListener("pointerup", finishSectionPointer);
      window.removeEventListener("pointercancel", cancelSectionPointer);
      window.removeEventListener("blur", cancelSectionPointerOnBlur);
    };
  }, [document, disabled, hidden]);

  const navigationSections = (
    sectionDragOrder ?? document.sections.map((section) => section.id)
  )
    .map((id) => document.sections.find((section) => section.id === id))
    .filter((section): section is ResumeSection => Boolean(section));
  return (
    <>
      <nav
        ref={sectionNavigator}
        className={`document-navigator${draggingSection ? " document-navigator--sorting" : ""}`}
        aria-label="Resume section navigation"
        hidden={hidden}
      >
        <div className="resume-navigation">
          <p id={keyboardHelpId} className="visually-hidden">
            Activate a section name to rename it. Use Alt and the up or down
            arrow to reorder a focused section, or Delete to request deletion.
          </p>
          <div className="contact-nav-item">
            <span>Contact</span>
          </div>
          <div className="section-sort-list" ref={sectionList}>
            {navigationSections.map((section) => (
              <div
                className={`section-nav-card${draggingSection === section.id ? " section-nav-card--dragging" : ""}`}
                key={section.id}
                data-section-id={section.id}
                ref={(card) => {
                  if (card) sectionCards.current.set(section.id, card);
                  else sectionCards.current.delete(section.id);
                }}
                onPointerDown={(event) => {
                  if (
                    disabled ||
                    event.button !== 0 ||
                    renamingSection === section.id ||
                    (event.target as HTMLElement).closest("input")
                  )
                    return;
                  event.preventDefault();
                  suppressRenameClick.current = false;
                  const bounds = event.currentTarget.getBoundingClientRect();
                  sectionPointerDrag.current = {
                    id: section.id,
                    pointerId: event.pointerId,
                    startX: event.clientX,
                    startY: event.clientY,
                    dragging: false,
                    offsetX: event.clientX - bounds.left,
                    offsetY: event.clientY - bounds.top,
                    width: bounds.width,
                    height: bounds.height,
                  };
                }}
              >
                <div className="section-nav-row">
                  {renamingSection === section.id ? (
                    <input
                      disabled={disabled}
                      aria-label={`Section name ${section.heading || "untitled"}`}
                      autoFocus
                      value={section.heading}
                      onBlur={() => setRenamingSection(null)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter" || event.key === "Escape") {
                          setRenamingSection(null);
                          event.currentTarget.blur();
                        }
                      }}
                      onChange={(event) =>
                        changeDocument((current) => ({
                          ...current,
                          sections: current.sections.map((item) =>
                            item.id === section.id
                              ? { ...item, heading: event.target.value }
                              : item,
                          ),
                        }))
                      }
                    />
                  ) : (
                    <button
                      type="button"
                      className="section-nav-title"
                      disabled={disabled}
                      aria-label={`Rename ${section.heading || "untitled section"}`}
                      aria-describedby={keyboardHelpId}
                      onKeyDown={(event) => {
                        if (event.key === "Delete") {
                          event.preventDefault();
                          setPendingTrashSection(section.id);
                        } else if (
                          event.altKey &&
                          (event.key === "ArrowUp" || event.key === "ArrowDown")
                        ) {
                          event.preventDefault();
                          const direction = event.key === "ArrowUp" ? -1 : 1;
                          changeDocument((current) => ({
                            ...current,
                            sections: moveItem(
                              current.sections,
                              section.id,
                              direction,
                            ),
                          }));
                        }
                      }}
                      onClick={(event) => {
                        if (!suppressRenameClick.current || event.detail === 0)
                          setRenamingSection(section.id);
                      }}
                    >
                      {section.heading || "Untitled section"}
                    </button>
                  )}
                </div>
              </div>
            ))}
          </div>
          {sectionDragPosition ? (
            <div
              className="section-drag-ghost"
              style={{
                left: sectionDragPosition.x,
                top: sectionDragPosition.y,
              }}
              aria-hidden="true"
            >
              {sectionDragPosition.label}
            </div>
          ) : null}
          <div
            ref={sectionTrash}
            className={`section-trash${trashActive ? " section-trash--active" : ""}`}
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M8 4h8l1 2h4v2H3V6h4l1-2Zm-2 6h12l-1 10H7L6 10Zm3 2v6h2v-6H9Zm4 0v6h2v-6h-2Z" />
            </svg>
            <span>Drag a section here to delete</span>
          </div>
          <div className="section-add-control">
            <label>
              Add section
              <select
                disabled={disabled}
                value={suggestedSection}
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
              onClick={() => {
                const next = {
                  ...createSection(document.sections.length),
                  heading: suggestedSection,
                };
                changeDocument((current) => ({
                  ...current,
                  sections: [...current.sections, next],
                }));
              }}
            >
              Add
            </button>
          </div>
        </div>
      </nav>
      <dialog
        ref={deleteDialog}
        className="section-delete-dialog"
        aria-labelledby={dialogTitleId}
        aria-describedby={dialogDescriptionId}
        onCancel={(event) => {
          event.preventDefault();
          setPendingTrashSection(null);
        }}
      >
        {pendingSection ? (
          <>
            <h2 id={dialogTitleId}>Delete this section?</h2>
            <p id={dialogDescriptionId}>
              {pendingSection?.heading || "This section"} and all of its items
              will be removed. You can undo this change afterward.
            </p>
            <div className="section-delete-dialog__actions">
              <button
                ref={cancelDelete}
                type="button"
                className="button--secondary"
                onClick={() => setPendingTrashSection(null)}
              >
                Cancel
              </button>
              <button
                type="button"
                className="button--danger"
                disabled={disabled || !pendingSection}
                onClick={() => {
                  const sectionId = pendingTrashSection;
                  setPendingTrashSection(null);
                  changeDocument((current) => ({
                    ...current,
                    sections: current.sections
                      .filter((section) => section.id !== sectionId)
                      .map((section, order) => ({ ...section, order })),
                  }));
                }}
              >
                Delete section
              </button>
            </div>
          </>
        ) : null}
      </dialog>
    </>
  );
}
