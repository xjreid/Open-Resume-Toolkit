const TRACKING =
  /^(utm_[a-z0-9_]+|gclid|fbclid|msclkid|mc_cid|mc_eid|token|access_token|auth|session|code)$/i;

export function normalizeSelection(value: string): string {
  const text = value.normalize("NFC").replace(/\r\n?/g, "\n").trim();
  if (!text) throw new Error("Select job text or a question first.");
  if (new TextEncoder().encode(text).length > 128 * 1024)
    throw new Error("Selection is too large (128 KiB maximum).");
  return text;
}

export function sanitizeUrl(value: string): string {
  const url = new URL(value);
  if (!new Set(["http:", "https:"]).has(url.protocol))
    throw new Error("This page URL cannot be captured.");
  url.username = "";
  url.password = "";
  url.hash = "";
  for (const key of [...url.searchParams.keys()])
    if (TRACKING.test(key)) url.searchParams.delete(key);
  const result = url.toString();
  if (new TextEncoder().encode(result).length > 4 * 1024)
    throw new Error("Page URL is too long.");
  return result;
}
