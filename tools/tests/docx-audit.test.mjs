import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import test from "node:test";

test("DOCX audit expects style casing and preserves metadata-only header rows", () => {
  const source = {
    contact: {
      fullName: "Candidate",
      email: "",
      phone: "",
      location: "",
      links: [],
    },
    sections: [
      {
        heading: "Expérience / Ελληνικά",
        entries: [
          {
            heading: "",
            subheading: "",
            location: "  Remote  ",
            dateRange: "  2023–2026  ",
            fields: [{ label: "Extra", value: "  Award  " }],
            bullets: [{ text: "  Built reliable software.  " }],
            links: [],
          },
        ],
      },
    ],
  };
  const code = `import importlib.util,json,sys
spec=importlib.util.spec_from_file_location("audit", "tools/verify-docx-fixtures.py")
module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
source=json.load(sys.stdin)
print(json.dumps({style:module.expected_paragraphs(source,style)[0] for style in ("plain","technical","professional","modern")}))`;
  const actual = JSON.parse(
    execFileSync(
      process.platform === "win32" ? "python" : "python3",
      ["-B", "-c", code],
      { input: JSON.stringify(source), encoding: "utf8" },
    ),
  );
  for (const style of ["plain", "technical", "professional", "modern"]) {
    assert.deepEqual(actual[style], [
      ["Candidate", "Title", false],
      [
        style === "technical"
          ? "EXPÉRIENCE / ΕΛΛΗΝΙΚΆ"
          : "Expérience / Ελληνικά",
        "Heading1",
        false,
      ],
      [" \tRemote", "Heading2", false],
      [" \t2023–2026", "Normal", false],
      [" \tAward", "Normal", false],
      ["Built reliable software.", "ListParagraph", true],
    ]);
  }
});
