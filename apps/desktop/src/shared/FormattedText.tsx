export function FormattedText({
  value,
  bullets = false,
}: {
  value: string;
  bullets?: boolean;
}) {
  if (bullets) {
    return value
      .split("\n")
      .filter((line) => line.trim())
      .map((line, index) => (
        <span className="formatted-bullet" key={index}>
          <span aria-hidden="true">• </span>
          <FormattedText value={line} />
        </span>
      ));
  }
  const parts = value.split(/(\*\*[^*]+\*\*|\*[^*]+\*|\[[^\]]+\]\([^\s)]+\))/g);
  return parts.map((part, index) => {
    if (part.startsWith("**") && part.endsWith("**"))
      return <strong key={index}>{part.slice(2, -2)}</strong>;
    if (part.startsWith("*") && part.endsWith("*"))
      return <em key={index}>{part.slice(1, -1)}</em>;
    const match = part.match(/^\[([^\]]+)\]\(([^\s)]+)\)$/);
    if (match && safeInlineHref(match[2]))
      return (
        <a
          key={index}
          className="formatted-link"
          href={match[2]}
          target="_blank"
          rel="noreferrer"
          onClick={(event) => event.stopPropagation()}
        >
          {match[1]}
        </a>
      );
    return part;
  });
}

export function safeInlineHref(value: string): boolean {
  try {
    return ["http:", "https:", "mailto:"].includes(new URL(value).protocol);
  } catch {
    return false;
  }
}
