import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
const require = createRequire(import.meta.url);
const {
  chromium,
} = require("/Users/xavierreid/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright");
const root = "/Users/xavierreid/Open-Resume-Toolkit";
const out =
  "/Users/xavierreid/Open-Resume-Toolkit/Aesthetic/previous refrence/current-ui-2026-10-04";
const document = JSON.parse(
  readFileSync(
    resolve(root, "Aesthetic/Resume-Designs/representative.source.json"),
    "utf8",
  ),
);
const browser = await chromium.launch({
  headless: true,
  executablePath:
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
});
const context = await browser.newContext({
  viewport: { width: 1440, height: 1000 },
  serviceWorkers: "block",
});
await context.addInitScript(
  ({ document }) => {
    const ok = (value) => ({ ok: true, value });
    window.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: "main" },
        currentWebview: { label: "main" },
      },
      transformCallback: () => 1,
      unregisterCallback: () => {},
      invoke: async (name, args) => {
        if (name === "health")
          return ok({
            status: "ok",
            appVersion: "0.0.0-dev",
            profile: "development",
            storageStatus: "ready",
            contractVersion: 2,
          });
        if (name === "load_resume")
          return ok({
            draft: { revision: 2, document },
            latestPublished: { revision: 1, document },
          });
        if (name === "save_resume")
          return ok({ revision: 3, document: args.request.payload.document });
        if (name === "close_status") return ok({ pendingAttempt: null });
        if (name.startsWith("plugin:")) return 1;
        if (name === "application_overlay_visibility") return ok(false);
        if (name === "list_tracker_entries")
          return ok([
            {
              id: "sample-1",
              revision: 1,
              value: {
                company: "Example Studio",
                title: "Senior Software Engineer",
                location: "Portland, OR",
                dateApplied: "2026-10-01",
                status: "interview",
                customStatus: "",
                sourceUrl: "https://example.org/careers",
                resume: null,
                coverLetter: null,
                coverContact: null,
                answers: [],
                style: "technical",
              },
            },
          ]);
        if (name === "backup_recovery_status")
          return ok({
            safetyCopyAvailable: false,
            restartOperationPending: false,
            safetyCleanupPending: false,
          });
        if (name === "application_context")
          return ok({
            publishedRevision: 1,
            aiLabel: "Balanced: Sample model",
            aiReady: true,
            aiBusy: false,
            selectedKeyId: "preview-key-1",
            preset: "balanced",
            presetOptions: [
              {
                preset: "balanced",
                label: "Balanced: Sample model",
                model: "sample",
              },
            ],
            browserConnected: true,
          });
        if (name === "load_application_workspace")
          return ok({
            revision: 1,
            workspace: {
              schemaVersion: 1,
              publishedRevision: 1,
              jobDescription: "Synthetic job description",
              jobUrl: "https://example.org/careers",
              roleInfo: {
                company: "Example Studio",
                title: "Senior Software Engineer",
                location: "Portland, OR",
              },
              resume: document,
              changePoints: [
                "Prioritized documented TypeScript experience.",
                "Moved the accessible interface project earlier.",
              ],
              alerts: [
                {
                  id: "one",
                  kind: "not_found",
                  category: "named_skill_or_technology",
                  requirement: "Python",
                  jobExcerpt: "Python required",
                  resumeEvidence: null,
                },
              ],
              alertsTruncated: false,
              dismissedAlertIds: [],
              ignoreAllAlerts: false,
              coverLetter: null,
              question: "",
              answer: "",
              approvedAnswers: [],
              style: "technical",
            },
          });
        if (name === "application_capture_status")
          return ok({ phase: "idle", sessionId: null, error: null });
        if (name.startsWith("load_application_")) return ok(null);
        if (name === "prepare_application_exports") return ok({});
        return {
          ok: false,
          error: {
            code: "COMMAND_UNAVAILABLE",
            messageKey: "Preview fixture unavailable",
          },
        };
      },
    };
  },
  { document },
);
try {
  const page = await context.newPage();
  await page.goto("http://127.0.0.1:1420");
  await page
    .getByRole("heading", { name: "Your resume", exact: true })
    .waitFor();
  await page.screenshot({ path: resolve(out, "edit.png") });
  await page.getByRole("button", { name: "View", exact: true }).click();
  await page.screenshot({ path: resolve(out, "view.png") });
  await page
    .getByRole("button", { name: "Application tracker", exact: true })
    .click();
  await page
    .getByRole("heading", { name: "Application tracker", exact: true })
    .waitFor();
  await page.screenshot({ path: resolve(out, "tracker.png") });
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.screenshot({ path: resolve(out, "settings.png") });
  await page.goto("http://127.0.0.1:1420/preview/ai.html");
  await page.getByRole("heading", { name: "API keys", exact: true }).waitFor();
  await page.screenshot({ path: resolve(out, "keys.png") });
  await page.getByRole("button", { name: "Data", exact: true }).click();
  await page.screenshot({ path: resolve(out, "data.png") });
  await page.setViewportSize({ width: 400, height: 960 });
  await page.goto("http://127.0.0.1:1420/overlay.html");
  await page
    .getByRole("heading", { name: "Example Studio", exact: true })
    .waitFor();
  await page.screenshot({ path: resolve(out, "overlay.png") });
  console.log("Captured all six current surfaces with synthetic fixtures.");
} finally {
  await browser.close();
}
