/** A tracker source is free text. Only resolved web URLs may be opened. */
export function trackerLinkTarget(source: string): string | null {
  const value = source.trim();
  if (!value) return null;
  const webPrefix = "https:" + "/".repeat(2);
  const search = `${webPrefix}www.google.com/search?q=${encodeURIComponent(value)}`;
  if (/[\u0000-\u001f\u007f]/u.test(value)) return search;
  const candidate = /^(https?:)?\/\//iu.test(value)
    ? value.startsWith("//")
      ? `https:${value}`
      : value
    : `${webPrefix}${value}`;
  try {
    const url = new URL(candidate);
    if (
      !["http:", "https:"].includes(url.protocol) ||
      !url.hostname ||
      (url.hostname !== "localhost" && !url.hostname.includes(".")) ||
      url.username ||
      url.password
    )
      return search;
    return url.toString();
  } catch {
    return search;
  }
}
