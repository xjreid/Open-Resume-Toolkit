import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { PublishedResume } from "../src/shared/App";
import {
  createEntry,
  createNamedField,
  createResumeDocument,
  createSection,
} from "../src/shared/resume-editor";

describe("published snapshot review", () => {
  it("escapes content and displays links without creating navigation authority", () => {
    const document = createResumeDocument();
    document.title = "<script>alert('synthetic')</script>";
    document.contact.links = [{ label: "Website", url: "https://example.com" }];
    const section = createSection(0);
    const entry = createEntry(0);
    entry.fields = [
      {
        ...createNamedField(0),
        label: "Language",
        value: "Rust",
        isSkill: true,
      },
    ];
    section.entries = [entry];
    document.sections = [section];
    const html = renderToStaticMarkup(<PublishedResume document={document} />);
    expect(html).toContain("&lt;script&gt;");
    expect(html).not.toContain("<script>");
    expect(html).not.toContain("href=");
    expect(html).toContain("https://example.com");
    expect(html).toContain("Language (skill)");
    expect(html).toContain("Rust");
  });
});

it("omits empty content, skill flags and dangling link separators from live reading", () => {
  const document = createResumeDocument();
  const section = createSection(0);
  const entry = createEntry(0);
  entry.fields = [
    { ...createNamedField(0), label: "Language", value: "Rust", isSkill: true },
    { ...createNamedField(1), label: " ", value: " " },
  ];
  entry.bullets = [{ id: "synthetic", order: 0, text: " " }];
  entry.links = [{ label: "", url: "mailto:synthetic@example.org" }];
  section.entries = [entry];
  document.sections = [section];
  const original = JSON.stringify(document);
  const html = renderToStaticMarkup(
    <PublishedResume document={document} onSelect={() => {}} />,
  );
  expect(html).not.toContain("(skill)");
  expect(html).not.toContain("<li>");
  expect(html).not.toContain(": mailto:");
  expect(html).toContain("mailto:synthetic@example.org");
  expect(JSON.stringify(document)).toBe(original);
});

it("keeps entry actions in the editable view without inventing document content", () => {
  const document = createResumeDocument();
  const section = createSection(0);
  section.heading = "Education";
  section.entries = [createEntry(0)];
  document.sections = [section];
  const before = JSON.stringify(document);
  const editable = renderToStaticMarkup(
    <PublishedResume
      document={document}
      onSelect={() => {}}
      onSelectEntry={() => {}}
      onAddEntry={() => {}}
    />,
  );
  expect(editable).toContain("Edit untitled entry 1");
  expect(editable).toContain("Add entry to Education");
  expect(editable).toContain('disabled=""');
  for (const pdf of [false, true]) {
    const readOnly = renderToStaticMarkup(
      <PublishedResume document={document} pdf={pdf} />,
    );
    expect(readOnly).not.toContain("<button");
    expect(readOnly).not.toContain("Edit untitled");
    expect(readOnly).not.toContain("Add entry");
  }
  expect(JSON.stringify(document)).toBe(before);
});
