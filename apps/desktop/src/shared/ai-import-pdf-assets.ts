// PDF.js requests only these bundled resources. Inline asset modules work with
// the existing CSP without remote URLs, fetch permissions, or system fonts.
const assets = import.meta.glob<string>(
  [
    "../../node_modules/pdfjs-dist/standard_fonts/*.{pfb,ttf}",
    "../../node_modules/pdfjs-dist/cmaps/*.bcmap",
  ],
  { query: "?url&inline", import: "default" },
);

export async function pdfBinaryData(
  kind: string,
  filename: string,
): Promise<Uint8Array> {
  const folder =
    kind === "standardFontDataUrl"
      ? "standard_fonts"
      : kind === "cMapUrl"
        ? "cmaps"
        : null;
  if (!folder || !/^[A-Za-z0-9_.-]+$/.test(filename))
    throw new Error("AI_PDF_RENDER_FAILED");
  const load = assets[`../../node_modules/pdfjs-dist/${folder}/${filename}`];
  if (!load) throw new Error("AI_PDF_RENDER_FAILED");
  const data = await load();
  const marker = ";base64,";
  if (!data.startsWith("data:") || !data.includes(marker))
    throw new Error("AI_PDF_RENDER_FAILED");
  return Uint8Array.from(
    atob(data.slice(data.indexOf(marker) + marker.length)),
    (character) => character.charCodeAt(0),
  );
}
