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
const style = process.argv[3] ?? "plain";
const schemaV2 = process.argv[4] === "schema-v2";
assert(
  process.argv.length <= 5 && (process.argv[4] === undefined || schemaV2),
  "explicit schema-v2 audit mode only",
);
assert(
  ["plain", "technical", "professional", "modern"].includes(style),
  "known bundled style only",
);
assert(directory, "provide synthetic PDF fixture directory");
const goldens = JSON.parse(
  readFileSync(
    new URL("../fixtures/documents/pdf-v1.sha256.json", import.meta.url),
    "utf8",
  ),
);
const textGoldens = JSON.parse(
  readFileSync(
    new URL("../fixtures/documents/text-v1.sha256.json", import.meta.url),
    "utf8",
  ),
);
const kinds = [
  "standard",
  "sparse",
  "unicode",
  "hostile",
  "dense",
  "optional",
  "structured",
  "paginated",
];
const expectedPageCounts = {
  standard: 1,
  sparse: 1,
  unicode: 1,
  hostile: 1,
  dense: 3,
  optional: 1,
  structured: 1,
  paginated: 1,
};
const safeArea = {
  plain: { left: 55, right: 557, bottom: 55, top: 737 },
  // Allow up to 3pt of font-glyph overhang beyond the configured margins.
  technical: { left: 35.25, right: 576.75, bottom: 30.75, top: 761.25 },
  professional: { left: 41.25, right: 570.75, bottom: 30.75, top: 761.25 },
  modern: { left: 39.75, right: 572.25, bottom: 30.75, top: 761.25 },
}[style];
assert.deepEqual(Object.keys(goldens).sort(), [...kinds].sort());
assert.deepEqual(Object.keys(textGoldens).sort(), [...kinds].sort());
const normalized = (value) =>
  value
    .replace(/^\s*- /gm, "")
    .replaceAll("•", "")
    .replace(/\s+/g, "");
const visibleLink = (link) => link.label.trim() || link.url.trim();
const visibleDate = (date) => {
  const months = [
    "Jan.",
    "Feb.",
    "Mar.",
    "Apr.",
    "May.",
    "Jun.",
    "Jul.",
    "Aug.",
    "Sep.",
    "Oct.",
    "Nov.",
    "Dec.",
  ];
  const calendar = (value) => {
    if (!value) return "";
    const text =
      value.month === null
        ? `${value.year}`
        : `${months[value.month - 1]} ${value.year}`;
    return value.expected ? `Expected ${text}` : text;
  };
  const start = calendar(date.start);
  const end =
    date.end?.kind === "present" ? "Present" : calendar(date.end?.value);
  const value = [start, end].filter(Boolean).join("–");
  return value && date.label.trim() ? `${date.label.trim()}: ${value}` : value;
};
const expectedVisibleText = (source, documentStyle) => {
  const parts = [];
  const add = (value) => {
    if (value?.trim()) parts.push(value.trim());
  };
  add(source.contact.fullName);
  add(source.contact.email);
  add(source.contact.phone);
  add(source.contact.location);
  source.contact.links.forEach((link) => add(visibleLink(link)));
  for (const section of source.sections) {
    if (!section.entries.length) continue;
    add(
      documentStyle === "modern"
        ? section.heading.toUpperCase()
        : section.heading,
    );
    for (const entry of section.entries) {
      const details = entry.fields.find(
        (field) =>
          field.label !== "__ort_body_paragraph__" &&
          field.label.trim().toLowerCase() !== "extra" &&
          field.value.trim(),
      );
      add(
        [entry.heading, details?.value]
          .filter((value) => value?.trim())
          .join(" | "),
      );
      const dates = [entry.dateRange, ...(entry.dates ?? []).map(visibleDate)]
        .filter((value) => value?.trim())
        .join("\n");
      const extras = entry.fields
        .filter((field) => field.label.trim().toLowerCase() === "extra")
        .map((field) => field.value)
        .filter((value) => value.trim())
        .join("\n");
      const rightRows = [entry.location, dates, extras].filter((value) =>
        value.trim(),
      );
      if (documentStyle === "plain") {
        add(rightRows.shift());
        add(entry.subheading);
        rightRows.forEach(add);
      } else {
        add(entry.subheading);
        rightRows.forEach(add);
      }
      const body = entry.fields.find(
        (field) =>
          field.label === "__ort_body_paragraph__" && field.value.trim(),
      );
      if (body) add(body.value);
      else entry.bullets.forEach((bullet) => add(bullet.text));
      entry.links.forEach((link) => add(visibleLink(link)));
    }
  }
  return parts.join("\n");
};
const expectedManifest = {
  documentSchemaVersion: schemaV2 ? 2 : 1,
  rendererVersion: "typst-0.15.1/ort-1",
  templateId: `${style}_pdf_v1`,
  templateSha256:
    style === "plain"
      ? "ebffeff9632a3aa59f9b4667de003506e91413c24124d551b928beafb66ee3a5"
      : createHash("sha256")
          .update(
            readFileSync(
              new URL(
                `../templates/resume/${style}_pdf_v1.typ`,
                import.meta.url,
              ),
            ),
          )
          .digest("hex"),
  fontBundleId: {
    plain: "libertinus-serif/typst-assets-0.15.1",
    technical: "liberation-serif/2.1.5",
    professional: "gelasio/7ab20e7e5c42+liberation-serif/2.1.5",
    modern: "liberation-sans/pdfjs-6.3.289",
  }[style],
  fontBundleSha256: {
    plain: "98b4ba1306ed79918244fb630cbc653c70671c9299ee98177addc6f560e3fdcf",
    technical:
      "31ced13201af120eda0affb7fa96197044221d11bf6f05c832bfba01f22b1416",
    professional:
      "4182a6a5e2f5ef08dee0ae208d0db4a95b25304324dda8af37a78d10328180b6",
    modern: "1117d564dbe2e60bd59e80fb38b4b3a84223c2a7beaa4dd7af0d1b4ee2ef9f57",
  }[style],
};
for (const kind of kinds) {
  const bytes = readFileSync(join(directory, `${kind}.pdf`));
  const source = JSON.parse(
    readFileSync(join(directory, `${kind}.source.json`), "utf8"),
  );
  const expectedBytes = readFileSync(join(directory, `${kind}.txt`));
  const expected = expectedBytes.toString("utf8");
  assert(!expectedBytes.includes(13), `${kind}: text contains CR bytes`);
  assert(expected.endsWith("\n") && !expected.endsWith("\n\n"));
  assert(!expected.includes(source.title));
  assert(!expected.includes(source.documentId));
  assert.equal(source.schemaVersion, schemaV2 ? 2 : 1);
  if (!schemaV2)
    assert.equal(
      createHash("sha256").update(expectedBytes).digest("hex"),
      textGoldens[kind],
      `${kind}: reviewed plain-text golden changed`,
    );
  const receipt = JSON.parse(
    readFileSync(join(directory, `${kind}.json`), "utf8"),
  );
  assert.equal(
    receipt.documentSha256,
    createHash("sha256").update(JSON.stringify(source)).digest("hex"),
    `${kind}: structured source identity`,
  );
  const digest = createHash("sha256").update(bytes).digest("hex");
  if (style === "plain" && !schemaV2)
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
    if (style === "plain" && !schemaV2)
      assert.equal(pdf.numPages, expectedPageCounts[kind]);
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
    const structureRoles = new Set();
    const collectRoles = (node) => {
      if (node?.role) structureRoles.add(node.role);
      for (const child of node?.children ?? [])
        if (typeof child === "object") collectRoles(child);
    };
    for (let number = 1; number <= pdf.numPages; number++) {
      const page = await pdf.getPage(number);
      assert.deepEqual(page.view, [0, 0, 612, 792]);
      const structure = await page.getStructTree();
      assert(structure, "tagged reading structure");
      collectRoles(structure);
      const content = await page.getTextContent();
      for (const item of content.items) {
        if (!("str" in item)) continue;
        text += item.str + (item.hasEOL ? "\n" : " ");
        assert(
          item.transform[4] >= safeArea.left &&
            item.transform[4] + item.width <= safeArea.right,
          `${kind}: text outside horizontal safe area (${item.transform[4]}..${item.transform[4] + item.width}; expected ${safeArea.left}..${safeArea.right})`,
        );
        assert(
          item.transform[5] >= safeArea.bottom &&
            item.transform[5] <= safeArea.top,
          `${kind}: text outside vertical safe area (${item.transform[5]}; expected ${safeArea.bottom}..${safeArea.top})`,
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
    assert.equal(
      normalized(text),
      normalized(expectedVisibleText(source, style)),
      `${kind}: visible text/order parity`,
    );
    const expectedUrls = [
      ...source.contact.links.map((link) => link.url),
      ...source.sections.flatMap((section) =>
        section.entries.flatMap((entry) => entry.links.map((link) => link.url)),
      ),
    ];
    assert.deepEqual(urls, expectedUrls);
    assert(structureRoles.has("Root") && structureRoles.has("Document"));
    const entries = source.sections.flatMap((section) => section.entries);
    if (entries.length) {
      assert(structureRoles.has("H1") && structureRoles.has("H2"));
    }
    if (entries.some((entry) => entry.bullets.length)) {
      for (const role of ["L", "LI", "Lbl", "LBody"])
        assert(structureRoles.has(role), `${kind}: missing ${role} list tag`);
    }
    if (expectedUrls.length) assert(structureRoles.has("Link"));
    console.log(
      `${style}/${kind}: ${style === "plain" ? "exact golden" : "bundled style audit (golden/native qualification pending)"}, ${pdf.numPages} page(s), text/order, geometry, tags, safe links, no active content`,
    );
  } finally {
    await task.destroy();
  }
}
