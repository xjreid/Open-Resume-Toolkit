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
  const regions = await page.evaluate(() => {
    const regions = [];
    let n = 0;
    function add(el, kind, id) {
      const b = el.getBoundingClientRect();
      const style = getComputedStyle(el);
      if (
        b.width < 3 ||
        b.height < 3 ||
        b.y >= innerHeight ||
        b.x >= innerWidth ||
        style.visibility === "hidden" ||
        style.display === "none" ||
        el.closest("[hidden]")
      )
        return;
      const x = Math.max(0, b.x),
        y = Math.max(0, b.y),
        w = Math.min(innerWidth, b.right) - x,
        h = Math.min(innerHeight, b.bottom) - y;
      if (w < 3 || h < 3) return;
      const words = (
        el.textContent ||
        el.getAttribute("aria-label") ||
        el.getAttribute("alt") ||
        el.getAttribute("placeholder") ||
        ""
      )
        .replace(/\s+/g, " ")
        .trim();
      regions.push({
        id: id || "element-" + ++n,
        kind,
        box: {
          x: x / innerWidth,
          y: y / innerHeight,
          w: w / innerWidth,
          h: h / innerHeight,
        },
        snap: false,
        note:
          (kind === "control"
            ? "UI control: "
            : kind === "chrome"
              ? "Flat UI panel or rule: "
              : "Interface text: ") + (words || el.className || el.tagName),
      });
    }
    add(
      document.querySelector(".workspace-page-heading h2"),
      "text",
      "page-title",
    );
    for (const el of document.querySelectorAll(
      "button,input,textarea,select,summary",
    ))
      add(el, "control");
    for (const el of document.querySelectorAll(
      ".brand-lockup h1,.workspace-page-heading p,.resume-display-header h2,.resume-save-status,.resume-canvas__hint,.contact-nav-item,.section-navigation-help,.resume-navigation h3,.resume-canvas__section h2,.resume-canvas__section h3,.resume-document__section-heading,.rail-storage-status",
    ))
      add(el, "text");
    for (const selector of [
      ".brand-lockup",
      ".workspace-rail",
      ".workspace-page-heading",
      ".workflow-steps",
      ".document-navigator",
    ])
      add(document.querySelector(selector), "chrome");
    add(document.querySelector(".resume-canvas"), "chrome", "paper");
    regions[regions.length - 1].container = true;
    regions[regions.length - 1].note =
      "Flat white document paper container with semantic inline fields";
    return { regions };
  });
  const { writeFileSync } = await import("node:fs");
  writeFileSync(
    resolve(root, ".impeccable/build/implemented-regions.json"),
    JSON.stringify(regions, null, 2),
  );
  console.log(JSON.stringify({ regionCount: regions.regions.length }));
} finally {
  await browser.close();
}
