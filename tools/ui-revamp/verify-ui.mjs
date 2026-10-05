import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { homedir } from "node:os";
import { fileURLToPath } from "node:url";
const require = createRequire(import.meta.url);
const { chromium } = require(
  process.env.ORT_PLAYWRIGHT_MODULE ??
    resolve(
      homedir(),
      ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright",
    ),
);
const root = fileURLToPath(new URL("../../", import.meta.url));
const out = root + "/.impeccable/review";
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
  viewport: { width: 1440, height: 1060 },
  deviceScaleFactor: 2,
  serviceWorkers: "block",
  reducedMotion: "reduce",
});
await context.addInitScript(
  ({ document }) => {
    const ok = (value) => ({ ok: true, value });
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
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
            latestPublished: {
              revision: 1,
              document: {
                ...document,
                contact: {
                  ...document.contact,
                  email: "alex.previous@example.org",
                },
              },
            },
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
        if (name === "load_backup_recovery_status")
          return ok({
            safetyCopyAvailable: false,
            restartOperationPending: false,
            safetyCleanupPending: false,
          });
        if (name === "browser_connection_status")
          return ok({ available: true, connected: true });
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
                  requirement: "Kubernetes",
                  jobExcerpt: "Kubernetes required",
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
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("http://127.0.0.1:1420");
  await page
    .getByRole("heading", { name: "Your resume", exact: true })
    .waitFor();
  await page.evaluate(() => document.fonts.ready);
  await page
    .getByRole("button", { name: /^Information/ })
    .first()
    .click();
  await page.mouse.move(0, 0);
  await page.evaluate(
    () =>
      new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))),
  );
  await page.screenshot({ path: resolve(out, "hero-repro.png") });
  if (process.env.ORT_CAPTURE_STAGE === "hero") {
    console.log(JSON.stringify({ errors }));
  } else {
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({ path: resolve(out, "edit-desktop.png") });
    await page.getByRole("button", { name: "View", exact: true }).click();
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({ path: resolve(out, "view-desktop.png") });
    await page
      .getByRole("button", { name: "Application tracker", exact: true })
      .click();
    await page
      .getByRole("heading", { name: "Application tracker", exact: true })
      .first()
      .waitFor();
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({ path: resolve(out, "tracker-desktop.png") });
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({ path: resolve(out, "settings-desktop.png") });
    await page
      .getByRole("button", { name: "Storage and deletion", exact: true })
      .click();
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({ path: resolve(out, "settings-storage.png") });
    await page
      .getByRole("button", { name: "Browser connections", exact: true })
      .click();
    await page
      .getByText("Development connection enabled.", { exact: true })
      .waitFor();
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({ path: resolve(out, "settings-browser.png") });
    await page.goto("http://127.0.0.1:1420/preview/ai.html");
    await page
      .getByRole("heading", { name: "API keys", exact: true })
      .waitFor();
    await page.evaluate(() => document.fonts.ready);
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({ path: resolve(out, "keys-desktop.png") });
    await page.getByRole("button", { name: "Data", exact: true }).click();
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({ path: resolve(out, "data-desktop.png") });
    await page.setViewportSize({ width: 1080, height: 760 });
    await page.goto("http://127.0.0.1:1420");
    await page
      .getByRole("heading", { name: "Your resume", exact: true })
      .waitFor();
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({ path: resolve(out, "edit-native-default.png") });
    await page.getByRole("button", { name: "Settings", exact: true }).click();
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({
      path: resolve(out, "settings-native-default.png"),
    });
    await page.goto("http://127.0.0.1:1420/preview/ai.html");
    await page
      .getByRole("heading", { name: "API keys", exact: true })
      .waitFor();
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({ path: resolve(out, "keys-native-default.png") });
    await page.getByRole("button", { name: "Data", exact: true }).click();
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({ path: resolve(out, "data-native-default.png") });
    await page.setViewportSize({ width: 720, height: 520 });
    await page.goto("http://127.0.0.1:1420");
    await page
      .getByRole("heading", { name: "Your resume", exact: true })
      .waitFor();
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({ path: resolve(out, "edit-native-minimum.png") });
    await page.setViewportSize({ width: 360, height: 760 });
    await page.goto("http://127.0.0.1:1420/overlay.html");
    await page
      .getByRole("heading", { name: "Example Studio", exact: true })
      .waitFor();
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({ path: resolve(out, "overlay-native.png") });
    await page
      .locator(".application-content")
      .evaluate((el) => (el.scrollTop = el.scrollHeight));
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
    await page.screenshot({ path: resolve(out, "overlay-native-lower.png") });
    console.log(JSON.stringify({ errors }));
  }
} finally {
  await browser.close();
}
