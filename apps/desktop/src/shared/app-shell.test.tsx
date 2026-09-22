import { renderToStaticMarkup } from "react-dom/server";
import { expect, it } from "vitest";
import { AppShell, type WorkspaceDestination } from "./AppShell";

it("keeps the two-line application brand across every destination", () => {
  for (const destination of [
    "resume",
    "import",
    "ai",
    "settings",
  ] as WorkspaceDestination[]) {
    const markup = renderToStaticMarkup(
      <AppShell
        destination={destination}
        onNavigate={() => {}}
        navigationBlocked={false}
        status={null}
      >
        <div />
      </AppShell>,
    );
    expect(markup).toContain(
      '<h1 aria-label="Open Resume Toolkit"><span>Open</span> <span>Resume Toolkit</span></h1>',
    );
    expect(markup.match(/<h1\b/g)).toHaveLength(1);
  }
});
