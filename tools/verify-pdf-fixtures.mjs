// Independent parser audit of synthetic output only. Never an import path.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import { join } from "node:path";
const require = createRequire(
  new URL("../apps/desktop/package.json", import.meta.url),
);
const { getDocument } = await import(
  pathToFileURL(require.resolve("pdfjs-dist/legacy/build/pdf.mjs")).href
);
const directory = process.argv[2];
assert(directory, "provide synthetic PDF fixture directory");
const goldens = JSON.parse(
  readFileSync(
    new URL("../fixtures/documents/pdf-v1.sha256.json", import.meta.url),
    "utf8",
  ),
);
const normalized = (value) =>
  value
    .replace(/^\s*- /gm, "")
    .replaceAll("•", "")
    .replace(/\s+/g, "");
const expectedManifest = {
  documentSchemaVersion: 1,
  rendererVersion: "typst-0.15.1/ort-1",
  templateId: "plain_pdf_v1",
  templateSha256:
    "8074983903239c57a2373fbd542b10c7bef70a890a7ef9bb6e98a1b9be799bc3",
  fontBundleId: "libertinus-serif/typst-assets-0.15.1",
  fontBundleSha256:
    "98b4ba1306ed79918244fb630cbc653c70671c9299ee98177addc6f560e3fdcf",
};
for (const kind of ["standard", "sparse", "unicode", "hostile", "dense"]) {
  const bytes = readFileSync(join(directory, `${kind}.pdf`));
  const receipt = JSON.parse(
    readFileSync(join(directory, `${kind}.json`), "utf8"),
  );
  const digest = createHash("sha256").update(bytes).digest("hex");
  assert.equal(digest, goldens[kind], `${kind}: reviewed PDF golden changed`);
  assert.equal(digest, receipt.pdfSha256);
  assert.equal(bytes.length, receipt.byteCount);
  for (const [key, value] of Object.entries(expectedManifest))
    assert.equal(receipt[key], value, `${kind}: reviewed render tuple changed`);
  assert(bytes.length <= 4 * 1024 * 1024);
  const task = getDocument({
    data: new Uint8Array(bytes),
    disableFontFace: true,
    useSystemFonts: false,
    useWorkerFetch: false,
    useWasm: false,
    stopAtErrors: true,
  });
  try {
    const pdf = await task.promise;
    assert.equal(pdf.numPages, receipt.pageCount);
    assert(pdf.numPages > 0 && pdf.numPages <= 5);
    assert.equal(await pdf.getJSActions(), null);
    assert.equal(await pdf.getAttachments(), null);
    assert.equal(await pdf.getFieldObjects(), null);
    assert.equal(await pdf.getOpenAction(), null);
    assert.equal((await pdf.getMarkInfo()).get("Marked"), true);
    const metadata = await pdf.getMetadata();
    assert.equal(metadata.info.Title, "Resume");
    assert(!JSON.stringify(metadata).includes("INTERNAL_SYNTHETIC"));
    let text = "";
    const urls = [];
    for (let number = 1; number <= pdf.numPages; number++) {
      const page = await pdf.getPage(number);
      assert.deepEqual(page.view, [0, 0, 612, 792]);
      assert(await page.getStructTree(), "tagged reading structure");
      const content = await page.getTextContent();
      for (const item of content.items) {
        if (!("str" in item)) continue;
        text += item.str + (item.hasEOL ? "\n" : " ");
        assert(
          item.transform[4] >= 55 && item.transform[4] + item.width <= 557,
          `${kind}: text outside horizontal safe area`,
        );
        assert(
          item.transform[5] >= 55 && item.transform[5] <= 737,
          `${kind}: text outside vertical safe area`,
        );
      }
      assert.equal(await page.getJSActions(), null);
      for (const annotation of await page.getAnnotations()) {
        assert.equal(annotation.subtype, "Link");
        assert(
          !annotation.action && !annotation.dest && !annotation.attachment,
        );
        assert(/^(https?:|mailto:)/.test(annotation.url));
        urls.push(annotation.url);
      }
    }
    const expected = readFileSync(join(directory, `${kind}.txt`), "utf8");
    assert.equal(
      normalized(text),
      normalized(expected),
      `${kind}: visible text/order parity`,
    );
    assert.deepEqual(
      urls,
      kind === "sparse"
        ? []
        : ["https://example.org/work?a=1&b=2", "https://example.org/project"],
    );
    console.log(
      `${kind}: exact golden, ${pdf.numPages} page(s), text/order, geometry, tags, safe links, no active content`,
    );
  } finally {
    await task.destroy();
  }
}
