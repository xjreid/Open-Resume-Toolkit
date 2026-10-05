import { createRequire } from "node:module";
import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
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
const out = root + "/.impeccable/review/ui-fixes";
mkdirSync(out, { recursive: true });
const stage = process.env.ORT_UI_FIX_STAGE ?? "before";
const document = JSON.parse(
  readFileSync(
    resolve(root, "Aesthetic/Resume-Designs/representative.source.json"),
    "utf8",
  ),
);
Object.assign(document.sections[0].entries[0], {
  subheading: "",
  dateRange: "",
  location: "",
  fields: [],
});
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
  const geometry = {};
  page.on("pageerror", (e) => errors.push(e.message));
  const settle = async () => {
    await page.evaluate(() => document.fonts.ready);
    await page.mouse.move(0, 0);
    await page.evaluate(
      () =>
        new Promise((r) =>
          requestAnimationFrame(() => requestAnimationFrame(r)),
        ),
    );
  };
  const shot = async (name) => {
    await settle();
    await page.screenshot({ path: resolve(out, `${stage}-${name}.png`) });
  };
  const measure = async () =>
    page.evaluate(() => {
      const first = document.querySelector(".canvas-entry");
      const rect = (el) => {
        if (!el) return null;
        const r = el.getBoundingClientRect();
        const c = getComputedStyle(el);
        return {
          x: r.x,
          y: r.y,
          width: r.width,
          height: r.height,
          font: c.font,
          color: c.color,
        };
      };
      const fields = [...first.querySelectorAll(".canvas-field")].map((el) => ({
        label: el.querySelector(".canvas-field__label")?.textContent,
        ...rect(el.querySelector("button")),
      }));
      return {
        scrollport: rect(document.querySelector(".app-content")),
        navigation: rect(document.querySelector(".workflow-steps")),
        fields,
      };
    });
  await page.setViewportSize({ width: 1080, height: 760 });
  await page.goto("http://127.0.0.1:1420");
  await page
    .getByRole("heading", { name: "Master resume", exact: true })
    .waitFor();
  await page.locator(".canvas-entry").first().waitFor();
  await shot("editor");
  geometry.editor = await measure();
  await page.locator(".app-content").evaluate((el) => (el.scrollTop = 350));
  await shot("editor-scrolled");
  geometry.editorScrolled = await measure();
  const entry = page.locator(".canvas-entry").first();
  await entry.getByRole("button", { name: /^Role/ }).click();
  await shot("role-editor");
  await page
    .getByRole("heading", { name: "Master resume", exact: true })
    .click();
  await entry.getByRole("button", { name: /^Information/ }).click();
  await shot("information-editor");
  await page.setViewportSize({ width: 720, height: 520 });
  await page.goto("http://127.0.0.1:1420");
  await page
    .getByRole("heading", { name: "Master resume", exact: true })
    .waitFor();
  await page.locator(".canvas-entry").first().waitFor();
  await page.locator(".canvas-entry").first().scrollIntoViewIfNeeded();
  await shot("minimum-editor");
  geometry.minimumEditor = await measure();
  if (stage !== "before") {
    const assert = (condition, message) => {
      if (!condition) throw new Error(message);
    };
    const fields = geometry.editorScrolled.fields;
    const field = (label) => fields.find((f) => f.label === label);
    assert(
      geometry.editorScrolled.navigation.y ===
        geometry.editorScrolled.scrollport.y,
      "Editor navigation has a top gap",
    );
    assert(
      geometry.minimumEditor.navigation.y ===
        geometry.minimumEditor.scrollport.y,
      "Minimum editor navigation is not sticky",
    );
    assert(field("Role").y > field("Title").y, "Role is not below Title");
    assert(
      field("Skills / details").x > field("Title").x &&
        Math.abs(field("Skills / details").y - field("Title").y) < 2,
      "Skills/details is not right of Title",
    );
    assert(
      field("Location").y > field("Date").y &&
        field("Extra").y > field("Location").y,
      "Metadata is not stacked",
    );
    for (const label of ["Date", "Location", "Extra"])
      assert(
        Math.abs(
          field(label).x +
            field(label).width -
            field("Date").x -
            field("Date").width,
        ) < 1,
        "Metadata is not right aligned",
      );
    assert(
      field("Date").font === field("Role").font &&
        field("Date").color === field("Role").color,
      "Empty date styling is inconsistent",
    );
    const firstEntry = page.locator(".canvas-entry").first();
    await firstEntry.getByRole("button", { name: /^Role/ }).click();
    const role = firstEntry.getByRole("textbox", { name: "Role", exact: true });
    await role.fill("Sample role");
    await role.evaluate((el) => {
      el.setSelectionRange(0, 6);
      el.dispatchEvent(new Event("select", { bubbles: true }));
    });
    await firstEntry
      .getByRole("button", { name: "Turn bold on", exact: true })
      .click();
    assert(
      (await role.inputValue()) === "**Sample** role",
      "Bold selection did not preserve formatting",
    );
    await role.press("Control+i");
    assert(
      (await role.inputValue()).includes("***Sample***"),
      "Italic shortcut did not preserve selected text",
    );
    await firstEntry.getByRole("button", { name: "Link", exact: true }).click();
    assert(
      await firstEntry
        .getByRole("button", { name: "Apply link", exact: true })
        .isDisabled(),
      "Empty link address should not apply",
    );
    await firstEntry
      .getByRole("textbox", { name: "Link address", exact: true })
      .fill("https://example.org");
    await firstEntry
      .getByRole("button", { name: "Apply link", exact: true })
      .click();
    assert(
      (await role.inputValue()).includes("](https://example.org)"),
      "Link application lost selection",
    );
    await shot("minimum-role-editor");
    await firstEntry.getByRole("button", { name: "Done", exact: true }).click();
    assert(
      (await firstEntry
        .getByRole("textbox", { name: "Role", exact: true })
        .count()) === 0,
      "Done did not close editor",
    );
    await page.waitForFunction(() =>
      document.activeElement?.classList.contains("canvas-field__button"),
    );
    assert(
      await firstEntry
        .getByRole("button", { name: /^Role/ })
        .evaluate((el) => el === document.activeElement),
      "Done did not restore keyboard focus",
    );
    await firstEntry.getByRole("button", { name: /^Role/ }).click();
    await firstEntry
      .getByRole("button", { name: "Clear role", exact: true })
      .click();
    assert((await role.inputValue()) === "", "Clear text failed");
    await role.press("Escape");
    assert(
      (await firstEntry
        .getByRole("textbox", { name: "Role", exact: true })
        .count()) === 0,
      "Escape did not close editor",
    );
    await firstEntry.getByRole("button", { name: /^Skills/ }).click();
    await firstEntry
      .getByRole("textbox", { name: "Skills / details", exact: true })
      .fill("TypeScript and accessible interfaces");
    await shot("minimum-details-editor");
    await firstEntry.getByRole("button", { name: "Done", exact: true }).click();
    await firstEntry.getByRole("button", { name: /^Location/ }).click();
    await firstEntry
      .getByRole("textbox", { name: "Location", exact: true })
      .fill("Portland, OR");
    await shot("minimum-location-editor");
    await firstEntry.getByRole("button", { name: "Done", exact: true }).click();
    await firstEntry.getByRole("button", { name: /^Date/ }).click();
    await shot("minimum-date-editor");
    await firstEntry.getByRole("button", { name: "Done", exact: true }).click();
    await firstEntry.getByRole("button", { name: /^Information/ }).click();
    const information = firstEntry.getByRole("textbox", {
      name: "Information",
      exact: true,
    });
    const original = await information.inputValue();
    await firstEntry
      .getByRole("combobox", { name: /^Text layout/ })
      .selectOption("paragraph");
    assert(
      (await information.inputValue()) === original,
      "Paragraph mode lost body text",
    );
    await firstEntry
      .getByRole("combobox", { name: /^Text layout/ })
      .selectOption("bullets");
    assert(
      (await information.inputValue()) === original,
      "Bullet mode lost body text",
    );
    await shot("minimum-information-editor");
    await firstEntry.getByRole("button", { name: "Done", exact: true }).click();
    geometry.interactions = [
      "selected-text bold",
      "italic keyboard shortcut",
      "link validation/application",
      "Done and focus restoration",
      "clear text",
      "Escape closes",
      "details/location/date editing",
      "paragraph/bullet preservation",
    ];
  }

  await page.setViewportSize({ width: 1080, height: 760 });
  await page.goto("http://127.0.0.1:1420/preview/ai.html");
  await page.getByRole("heading", { name: "API keys", exact: true }).waitFor();
  await page.locator(".app-content").evaluate((el) => (el.scrollTop = 350));
  await shot("ai-keys-scrolled");
  geometry.aiKeysScrolled = {
    navigation: await page.locator(".workflow-steps").boundingBox(),
    scrollport: await page.locator(".app-content").boundingBox(),
  };
  if (
    stage !== "before" &&
    geometry.aiKeysScrolled.navigation.y !==
      geometry.aiKeysScrolled.scrollport.y
  )
    throw new Error("AI navigation has a top gap");
  await page.getByRole("button", { name: "Data", exact: true }).click();
  await page.locator(".app-content").evaluate((el) => (el.scrollTop = 350));
  await shot("ai-data-scrolled");
  geometry.aiDataScrolled = {
    navigation: await page.locator(".workflow-steps").boundingBox(),
    scrollport: await page.locator(".app-content").boundingBox(),
  };
  if (
    stage !== "before" &&
    geometry.aiDataScrolled.navigation.y !==
      geometry.aiDataScrolled.scrollport.y
  )
    throw new Error("AI navigation has a top gap");
  writeFileSync(
    resolve(out, `${stage}-geometry.json`),
    JSON.stringify({ geometry, errors }, null, 2),
  );
  console.log(JSON.stringify({ geometry, errors }));
} finally {
  await browser.close();
}
