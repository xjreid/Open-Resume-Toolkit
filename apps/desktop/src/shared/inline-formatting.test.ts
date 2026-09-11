import { describe, expect, it } from "vitest";
import { linkSelection, toggleBoldSelection } from "./inline-formatting";

describe("inline canvas formatting", () => {
  it("toggles bold around the highlighted text", () => {
    const bold = toggleBoldSelection("Senior engineer", 0, 6)!;
    expect(bold).toEqual({
      value: "**Senior** engineer",
      selectionStart: 2,
      selectionEnd: 8,
    });
    expect(
      toggleBoldSelection(bold.value, bold.selectionStart, bold.selectionEnd),
    ).toEqual({
      value: "Senior engineer",
      selectionStart: 0,
      selectionEnd: 6,
    });
  });

  it("links only the highlighted text and preserves its selection", () => {
    expect(
      linkSelection("See portfolio", 4, 13, " https://example.com "),
    ).toEqual({
      value: "See [portfolio](https://example.com)",
      selectionStart: 5,
      selectionEnd: 14,
    });
    expect(
      linkSelection("Nothing selected", 3, 3, "https://example.com"),
    ).toBeNull();
  });
});
