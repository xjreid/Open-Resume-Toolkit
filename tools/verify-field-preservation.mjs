// Independently parse synthetic PDF artifacts, including mixed body representations.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import { join } from "node:path";
const require = createRequire(
  new URL("../apps/desktop/package.json", import.meta.url),
);
const { getDocument } = await import(
  pathToFileURL(require.resolve("pdfjs-dist/legacy/build/pdf.mjs")).href
);
const root = process.argv[2];
assert(root, "provide synthetic artifact directory");
const markers = JSON.parse(readFileSync(join(root, "markers.json"), "utf8"));
for (const style of ["plain", "technical", "professional", "modern"]) {
  const task = getDocument({
    data: new Uint8Array(readFileSync(join(root, `${style}.pdf`))),
  });
  const pdf = await task.promise;
  let text = "";
  for (let page = 1; page <= pdf.numPages; page++)
    text += (await (await pdf.getPage(page)).getTextContent()).items
      .map((item) => item.str)
      .join(" ");
  for (const marker of markers)
    assert(
      text.replace(/\s/g, "").includes(marker),
      `${style}: missing ${marker}`,
    );
  await task.destroy();
}
console.log("All seven retained markers survived every PDF style.");
