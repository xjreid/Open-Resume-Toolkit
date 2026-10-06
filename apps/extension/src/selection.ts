const TRACKING =
  /^(utm_[a-z0-9_]+|gclid|fbclid|msclkid|mc_cid|mc_eid|token|access_token|refresh_token|id_token|auth|authorization|session|sessionid|session_id|code|password|passwd|secret|api_key|apikey)$/i;

export function normalizeSelection(value: string): string {
  if (typeof value !== "string") throw new Error("CAPTURE_INVALID");
  if (/[\uD800-\uDFFF]/u.test(value)) throw new Error("CAPTURE_INVALID");
  const text = value.normalize("NFC").replace(/\r\n?/g, "\n").trim();
  if (!text) throw new Error("EMPTY_SELECTION");
  if (new TextEncoder().encode(text).length > 128 * 1024)
    throw new Error("CAPTURE_TOO_LARGE");
  return text;
}

export function sanitizeUrl(value: string): string {
  if (typeof value !== "string" || /[\uD800-\uDFFF]/u.test(value))
    throw new Error("CAPTURE_INVALID");
  const url = new URL(value);
  if (!new Set(["http:", "https:"]).has(url.protocol))
    throw new Error("PAGE_UNAVAILABLE");
  url.username = "";
  url.password = "";
  url.hash = "";
  for (const key of [...url.searchParams.keys()])
    if (TRACKING.test(key)) url.searchParams.delete(key);
  const result = url.toString();
  if (new TextEncoder().encode(result).length > 4 * 1024)
    throw new Error("CAPTURE_TOO_LARGE");
  return result;
}
