export interface InlineFormatResult {
  value: string;
  selectionStart: number;
  selectionEnd: number;
}

export function toggleBoldSelection(
  value: string,
  start: number,
  end: number,
): InlineFormatResult | null {
  if (end <= start) return null;
  const selected = value.slice(start, end);
  if (selected.startsWith("**") && selected.endsWith("**")) {
    return {
      value: value.slice(0, start) + selected.slice(2, -2) + value.slice(end),
      selectionStart: start,
      selectionEnd: end - 4,
    };
  }
  if (
    start >= 2 &&
    value.slice(start - 2, start) === "**" &&
    value.slice(end, end + 2) === "**"
  ) {
    return {
      value: value.slice(0, start - 2) + selected + value.slice(end + 2),
      selectionStart: start - 2,
      selectionEnd: end - 2,
    };
  }
  return {
    value: value.slice(0, start) + `**${selected}**` + value.slice(end),
    selectionStart: start + 2,
    selectionEnd: end + 2,
  };
}

export function toggleItalicSelection(
  value: string,
  start: number,
  end: number,
): InlineFormatResult | null {
  if (end <= start) return null;
  const selected = value.slice(start, end);
  if (
    selected.startsWith("*") &&
    selected.endsWith("*") &&
    !selected.startsWith("**")
  ) {
    return {
      value: value.slice(0, start) + selected.slice(1, -1) + value.slice(end),
      selectionStart: start,
      selectionEnd: end - 2,
    };
  }
  if (
    start >= 1 &&
    value.slice(start - 1, start) === "*" &&
    value.slice(end, end + 1) === "*" &&
    value.slice(start - 2, start) !== "**"
  ) {
    return {
      value: value.slice(0, start - 1) + selected + value.slice(end + 1),
      selectionStart: start - 1,
      selectionEnd: end - 1,
    };
  }
  return {
    value: value.slice(0, start) + `*${selected}*` + value.slice(end),
    selectionStart: start + 1,
    selectionEnd: end + 1,
  };
}

export function linkSelection(
  value: string,
  start: number,
  end: number,
  url: string,
): InlineFormatResult | null {
  const address = url.trim();
  if (!address) return null;
  const selected = end > start ? value.slice(start, end) : "Enter Text Here";
  return {
    value:
      value.slice(0, start) + `[${selected}](${address})` + value.slice(end),
    selectionStart: start + 1,
    selectionEnd: start + 1 + selected.length,
  };
}
