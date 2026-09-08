import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import test from "node:test";

// Hand-written expectations keep the independent audit from merely trusting
// the text emitted by the Rust renderer under test.
test("output audit preserves partial, expected, present and reversed dates", () => {
  const calendar = (year, month = null, expected = false) => ({
    year,
    month,
    expected,
  });
  const cases = [
    [{ label: "Ignored", start: null, end: null }, ""],
    [{ label: "", start: calendar(2020), end: null }, "2020"],
    [
      { label: "Period", start: calendar(2020), end: { kind: "present" } },
      "Period: 2020–Present",
    ],
    [
      {
        label: "Graduation",
        start: null,
        end: { kind: "date", value: calendar(2027, 6, true) },
      },
      "Graduation: Expected Jun 2027",
    ],
    [
      {
        label: "",
        start: calendar(2024),
        end: { kind: "date", value: calendar(2023) },
      },
      "2024–2023",
    ],
    [
      {
        label: "  Term  ",
        start: calendar(1, 1),
        end: { kind: "date", value: calendar(9999, 12) },
      },
      "Term: Jan 1–Dec 9999",
    ],
    [
      { label: "", start: calendar(2027, null, true), end: null },
      "Expected 2027",
    ],
  ];
  const code = `import importlib.util,json,sys
spec=importlib.util.spec_from_file_location("dates", "tools/output-audit-dates.py")
module=importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
print(json.dumps([module.date_text(item) for item in json.load(sys.stdin)]))`;
  const actual = execFileSync(
    process.platform === "win32" ? "python" : "python3",
    ["-c", code],
    {
      input: JSON.stringify(cases.map(([input]) => input)),
      encoding: "utf8",
    },
  );
  assert.deepEqual(
    JSON.parse(actual),
    cases.map(([, expected]) => expected),
  );
});
