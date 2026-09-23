const trackingKey =
  /^(utm_[a-z0-9_]+|gclid|fbclid|msclkid|mc_cid|mc_eid|token|access_token|auth|session|code)$/i;

export function sanitizeCaptureUrl(input: string): string {
  if (!input.trim()) return "";
  const url = new URL(input.trim());
  if (!new Set(["http:", "https:"]).has(url.protocol) || !url.hostname) {
    throw new Error("JOB_URL_INVALID");
  }
  url.username = "";
  url.password = "";
  url.hash = "";
  for (const key of [...url.searchParams.keys()]) {
    if (trackingKey.test(key)) url.searchParams.delete(key);
  }
  const result = url.toString();
  if (new TextEncoder().encode(result).length > 4096) {
    throw new Error("JOB_URL_INVALID");
  }
  return result;
}
