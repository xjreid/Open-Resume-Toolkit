import { useLayoutEffect, useRef, useState, type MouseEvent } from "react";
import { trackerLinkTarget } from "./tracker-link";

export function TrackerSourceLink({
  source,
  label,
  onClick,
  onEdit,
}: {
  source: string;
  label: string;
  onClick: (event: MouseEvent<HTMLAnchorElement>) => void;
  onEdit: () => void;
}) {
  const container = useRef<HTMLDivElement>(null);
  const link = useRef<HTMLAnchorElement>(null);
  const [expanded, setExpanded] = useState(false);
  const [overflows, setOverflows] = useState(false);

  useLayoutEffect(() => {
    if (expanded) return;
    const measure = () => {
      if (container.current && link.current) {
        // Compare with the whole cell, so a toggle does not itself cause overflow.
        setOverflows(link.current.scrollWidth > container.current.clientWidth);
      }
    };
    measure();
    const observer =
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(measure);
    if (container.current) observer?.observe(container.current);
    if (link.current) observer?.observe(link.current);
    return () => observer?.disconnect();
  }, [source, expanded]);

  return (
    <div
      ref={container}
      className={
        "tracker-source" + (expanded ? " tracker-source--expanded" : "")
      }
    >
      <a
        ref={link}
        className="tracker-source-link"
        href={trackerLinkTarget(source) ?? "#"}
        target="_blank"
        rel="noopener noreferrer"
        title="Click to open; double-click to edit"
        onClick={onClick}
        onDoubleClick={(event) => {
          event.preventDefault();
          onEdit();
        }}
      >
        {source}
      </a>
      {(overflows || expanded) && (
        <button
          type="button"
          className="tracker-source-toggle"
          aria-label={(expanded ? "Compress" : "Expand") + " link for " + label}
          aria-expanded={expanded}
          title={expanded ? "Compress link" : "Expand link"}
          onClick={() => setExpanded((current) => !current)}
        >
          {expanded ? "Compress" : "Expand"}
        </button>
      )}
    </div>
  );
}
