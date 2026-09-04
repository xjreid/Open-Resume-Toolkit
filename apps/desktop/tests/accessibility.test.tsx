import axe, { type AxeResults } from "axe-core";
import { JSDOM } from "jsdom";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { App, PublishedResume } from "../src/shared/App";
import { CloseDialog } from "../src/shared/CloseDialog";
import { PdfPreviewPanel } from "../src/shared/PdfPreview";
import {
  createEntry,
  createResumeDocument,
  createSection,
} from "../src/shared/resume-editor";

async function audit(
  markup: string,
  excludedSelectors: string[] = [],
): Promise<AxeResults> {
  const dom = new JSDOM(
    `<!doctype html><html lang="en"><head><title>Open Resume Toolkit</title></head><body>${markup}</body></html>`,
  );
  for (const selector of excludedSelectors) {
    const elements = dom.window.document.querySelectorAll(selector);
    if (elements.length !== 1) {
      throw new Error(
        `Expected one explicitly excluded ${selector} surface, found ${elements.length}`,
      );
    }
    for (const element of elements) {
      element.remove();
    }
  }

  return axe.run(dom.window.document.documentElement, {
    // jsdom does not calculate layout or resolved colors. Native WebView color
    // and zoom behavior remains part of the manual macOS/Windows matrix.
    rules: { "color-contrast": { enabled: false } },
  });
}

function violationSummary(results: AxeResults): string {
  return results.violations
    .map(
      (violation) =>
        `${violation.id}: ${violation.help}\n${violation.nodes
          .map((node) => `  ${node.target.join(" ")}: ${node.failureSummary}`)
          .join("\n")}`,
    )
    .join("\n\n");
}

async function expectNoViolations(
  markup: string,
  excludedSelectors: string[] = [],
) {
  const results = await audit(markup, excludedSelectors);
  expect(violationSummary(results)).toBe("");
}

describe("M2 reachable desktop accessibility", () => {
  it("fails its positive control when an interactive control has no name", async () => {
    const results = await audit(
      "<main><h1>Broken fixture</h1><button></button></main>",
    );
    expect(results.violations.map((violation) => violation.id)).toContain(
      "button-name",
    );
  });

  it("keeps the medium-routed main shell and overlay free of detectable violations", async () => {
    await expectNoViolations(renderToStaticMarkup(<App surface="main" />), [
      ".backup-panel",
      ".storage-panel",
    ]);
    await expectNoViolations(renderToStaticMarkup(<App surface="overlay" />));
  });

  it("labels the PDF preview and close-decision surfaces", async () => {
    const panels = renderToStaticMarkup(
      <main>
        <h1>Resume workspace checks</h1>
        <PdfPreviewPanel
          saved={null}
          published={null}
          dirty={false}
          blocked={false}
          onBegin={() => true}
          onFinish={() => {}}
          semantic={() => null}
        />
        <CloseDialog
          open
          busy={false}
          resolving={false}
          canSave
          error={null}
          saveError={null}
          onCancel={() => {}}
          onSave={() => {}}
          onDiscard={() => {}}
          onRetry={() => {}}
        />
      </main>,
    ).replace("<dialog", '<dialog open=""');

    await expectNoViolations(panels);
  });

  it("preserves a meaningful heading structure for a populated resume", async () => {
    const document = createResumeDocument();
    const section = createSection(0);
    section.heading = "Experience";
    const entry = createEntry(0);
    entry.heading = "Software engineer";
    section.entries = [entry];
    document.sections = [section];

    await expectNoViolations(
      renderToStaticMarkup(
        <main>
          <h1>Published resume review</h1>
          <PublishedResume document={document} />
        </main>,
      ),
    );
  });
});
