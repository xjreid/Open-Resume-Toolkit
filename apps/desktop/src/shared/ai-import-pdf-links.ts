import type { PDFPageProxy } from "pdfjs-dist";

export type PdfHyperlink = {
  page: number;
  rect: [number, number, number, number];
  url: string;
};

// Read only URI link annotations; never execute actions or fetch destinations.
// PDF.js has already resolved page rotation and crop offsets in this viewport.
export async function pageHyperlinks(
  page: PDFPageProxy,
  pageNumber: number,
  signal: AbortSignal,
): Promise<PdfHyperlink[]> {
  const viewport = page.getViewport({ scale: 1 });
  const result: PdfHyperlink[] = [];
  for (const annotation of await page.getAnnotations({ intent: "display" })) {
    signal.throwIfAborted();
    if (annotation.subtype !== "Link" || typeof annotation.url !== "string")
      continue;
    const url = annotation.url as string;
    try {
      const parsed = new URL(url);
      if (
        !["https:", "http:", "mailto:"].includes(parsed.protocol) ||
        parsed.username ||
        parsed.password ||
        /[\s\u0000-\u001f\u007f]/u.test(url)
      )
        continue;
    } catch {
      continue;
    }
    if (url.length > 2000) throw new Error("AI_PDF_LINK_LIMIT");
    if (
      !Array.isArray(annotation.rect) ||
      annotation.rect.length !== 4 ||
      !annotation.rect.every(Number.isFinite)
    )
      continue;
    const rect = [
      ...viewport.convertToViewportPoint(
        annotation.rect[0],
        annotation.rect[1],
      ),
      ...viewport.convertToViewportPoint(
        annotation.rect[2],
        annotation.rect[3],
      ),
    ];
    const normalized = [
      Math.min(rect[0], rect[2]) / viewport.width,
      Math.min(rect[1], rect[3]) / viewport.height,
      Math.max(rect[0], rect[2]) / viewport.width,
      Math.max(rect[1], rect[3]) / viewport.height,
    ].map((value) => Math.round(Math.max(0, Math.min(1, value)) * 1000));
    if (
      normalized.every(Number.isFinite) &&
      normalized[0] < normalized[2] &&
      normalized[1] < normalized[3]
    )
      result.push({
        page: pageNumber,
        rect: normalized as PdfHyperlink["rect"],
        url,
      });
  }
  return result;
}

export function checkHyperlinkLimits(links: PdfHyperlink[]) {
  if (
    links.length > 200 ||
    new TextEncoder().encode(JSON.stringify(links)).length > 48_000
  )
    throw new Error("AI_PDF_LINK_LIMIT");
}
