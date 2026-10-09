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
async function capture(page, options) {
  await page.mouse.move(0, 0);
  await page.evaluate(async () => {
    await document.fonts.load("600 12px Hanken");
    await document.fonts.ready;
    await new Promise((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(resolve)),
    );
  });
  // Headless Chromium can resolve font readiness before its next text paint.
  await page.waitForTimeout(250);
  if (page.url().includes("overlay.html")) {
    console.log(
      JSON.stringify(
        await page.evaluate(() =>
          [
            ...document.querySelectorAll(
              ".application-provider-label, .application-reasoning label",
            ),
          ].map((element) => {
            const style = getComputedStyle(element);
            const rect = element.getBoundingClientRect();
            return {
              text: element.textContent,
              color: style.color,
              visibility: style.visibility,
              opacity: style.opacity,
              rect: {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
              },
            };
          }),
        ),
      ),
    );
  }
  await page.screenshot({ ...options, animations: "disabled" });
}
const document = JSON.parse(
  readFileSync(
    resolve(root, "Aesthetic/Resume-Designs/representative.source.json"),
    "utf8",
  ),
);
const browser = await chromium.launch({
  headless: true,
  args: ["--disable-gpu"],
  executablePath:
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
});
const context = await browser.newContext({
  viewport: { width: 1080, height: 760 },
  deviceScaleFactor: 2,
  serviceWorkers: "block",
  reducedMotion: "reduce",
});
await context.addInitScript(
  ({ document }) => {
    const ok = (value) => ({ ok: true, value });
    const parameters = new URLSearchParams(location.search);
    let plan = {
      settings: {
        cleanupRequired: false,
        connectionId: "019a0000-0000-7000-8000-000000000050",
        enabled: true,
        model: "gpt-6.1-sol",
        reasoning: "medium",
        reserveEnabled: true,
        reservePercent: 20,
      },
      revision: 1,
      connected: true,
      accountPlan: "plus",
      loginPending: false,
      operationActive: parameters.has("busy"),
      runtimeVersion: "0.162.0",
      errorCode: null,
      models: ["gpt-6.1-sol", "gpt-6-sol", "gpt-6-luna", "gpt-5.6-terra"].map(
        (id) => ({
          id,
          name: id
            .replace("gpt-", "GPT-")
            .replace(
              /-(sol|luna|terra)$/,
              (_, name) => ` ${name[0].toUpperCase()}${name.slice(1)}`,
            ),
          supported: id !== "gpt-5.6-terra",
          explanation: null,
          reasoningEfforts:
            id === "gpt-6-luna"
              ? ["low", "medium"]
              : ["low", "medium", "high", "xhigh"],
        }),
      ),
      quota: parameters.has("unknown-usage")
        ? null
        : {
            fetchedAtUnixMs: Date.now(),
            windows: [
              {
                limitId: "codex",
                name: "Codex primary",
                window: "primary",
                remainingPercent: 64.5,
                windowDurationMinutes: 300,
                resetsAt: null,
              },
              {
                limitId: "codex",
                name: "Codex secondary",
                window: "secondary",
                remainingPercent: 82,
                windowDurationMinutes: 10080,
                resetsAt: null,
              },
            ],
          },
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
    if (parameters.has("disconnected") || parameters.has("disabled")) {
      plan.connected = false;
      plan.accountPlan = null;
      plan.settings.connectionId = null;
      plan.settings.enabled = !parameters.has("disabled");
      plan.quota = null;
    }
    window.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: "main" },
        currentWebview: { label: "main" },
      },
      transformCallback: () => 1,
      unregisterCallback: () => {},
      invoke: async (name, args) => {
        if (name === "load_chatgpt_plan") return ok(plan);
        if (name === "check_codex_runtime")
          return ok({ ready: true, errorCode: null });
        if (name === "load_codex_runtime_install")
          return ok({
            phase: "idle",
            downloadedBytes: 0,
            totalBytes: 98089521,
            errorCode: null,
          });
        if (name === "save_chatgpt_plan") {
          const { expectedRevision, ...settings } = args.request;
          plan = {
            ...plan,
            revision: expectedRevision + 1,
            settings: { ...plan.settings, ...settings },
          };
          return ok(plan);
        }
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
            connectionSource: parameters.has("api-key")
              ? "direct_api"
              : "chatgpt_plan",
            profileId: "019a0000-0000-7000-8000-000000000001",
            publishedRevision: 1,
            aiLabel: "Using Codex · GPT-6.1 Sol",
            aiReady: parameters.has("api-key") || plan.connected,
            aiBusy: parameters.has("busy"),
            selectedKeyReady: parameters.has("api-key") || plan.connected,
            selectedKeyId: "019a0000-0000-7000-8000-000000000050",
            model: parameters.has("api-key")
              ? "gpt-6-sol"
              : plan.settings.model,
            modelOptions: [{ model: "gpt-6-sol" }, { model: "gpt-6.1-sol" }],
            browserConnected: false,
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
              alerts: [],
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
        if (name === "prepare_application_exports")
          return ok({
            revision: 1,
            pdfReady: true,
            docxReady: true,
            pageCount: 1,
          });
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
  page.setDefaultTimeout(7000);
  const results = [];
  for (const viewport of [
    { width: 1080, height: 760 },
    { width: 720, height: 520 },
  ]) {
    await page.setViewportSize(viewport);
    await page.goto("http://127.0.0.1:1420/preview/ai.html?codex=1");
    await page.getByRole("button", { name: "Codex", exact: true }).click();
    await page.getByLabel("Codex model").waitFor();
    await capture(page, { path: `${out}/plan-${viewport.width}.png` });
    const overflow = await page.evaluate(() => ({
      width: document.documentElement.scrollWidth,
      viewport: innerWidth,
    }));
    if (overflow.width > viewport.width)
      throw Error(`Horizontal overflow: ${JSON.stringify(overflow)}`);
    await page
      .getByRole("button", { name: "Sign out", exact: true })
      .scrollIntoViewIfNeeded();
    await capture(page, { path: `${out}/plan-bottom-${viewport.width}.png` });
    await page.getByRole("button", { name: "Sign out", exact: true }).click();
    if (!(await page.getByLabel("Enable Codex").isChecked()))
      throw new Error("Sign out changed the enabled preference");
    await page.getByRole("button", { name: "My Keys", exact: true }).click();
    // Fresh registry is loaded when activation status changes.
    await page
      .getByText("Codex must be disabled to use API keys.", {
        exact: false,
      })
      .waitFor();
    await capture(page, { path: `${out}/plan-keys-${viewport.width}.png` });
    if (await page.locator(".ai-add-key-trigger").isEnabled())
      throw new Error("My Keys must be disabled while Codex is enabled");
    await page.getByRole("button", { name: "Data", exact: true }).click();
    await page
      .getByRole("button", { name: "Choose view", exact: true })
      .click();
    await page
      .getByRole("button", {
        name: "View activity for Codex",
        exact: true,
      })
      .click();
    await page
      .getByText("Codex monetary cost is not tracked.", { exact: false })
      .waitFor();
    await capture(page, { path: `${out}/plan-data-${viewport.width}.png` });
    results.push({ viewport, overflow });
  }
  await page.setViewportSize({ width: 1080, height: 760 });
  await page.goto("http://127.0.0.1:1420/preview/ai.html?disconnected=1");
  await page.getByRole("button", { name: "Codex", exact: true }).click();
  await page
    .getByRole("button", { name: "Connect ChatGPT account", exact: true })
    .waitFor();
  await capture(page, { path: `${out}/plan-disconnected.png` });
  if (!(await page.getByLabel("Enable Codex", { exact: false }).isChecked()))
    throw new Error("Enabled preference disappeared after sign-out");
  await page.getByLabel("Enable Codex", { exact: false }).uncheck();
  if (
    await page
      .getByRole("button", { name: "Connect ChatGPT account", exact: true })
      .count()
  )
    throw new Error("Account controls must be hidden while Codex is disabled");
  await page.getByRole("button", { name: "My Keys", exact: true }).click();
  if (!(await page.locator(".ai-add-key-trigger").isEnabled()))
    throw new Error("API keys did not resume after disabling");
  await capture(page, { path: `${out}/codex-disabled-keys.png` });
  await page.setViewportSize({ width: 360, height: 760 });
  await page.goto("http://127.0.0.1:1420/overlay.html");
  await page.getByLabel("Codex model").waitFor();
  await page.getByText("64.5% remaining", { exact: true }).waitFor();
  await capture(page, { path: `${out}/codex-controls-overlay.png` });
  await page.getByLabel("Codex model").selectOption("gpt-6-luna");
  await page.getByLabel("Codex reasoning").selectOption("medium");
  if ((await page.getByLabel("Codex model").inputValue()) !== "gpt-6-luna")
    throw new Error("Codex model was not saved");
  await page.goto("http://127.0.0.1:1420/overlay.html?unknown-usage=1");
  await page.getByLabel("Codex model").waitFor();
  await capture(page, { path: `${out}/codex-controls-overlay-unknown.png` });
  if (await page.getByLabel("Account-wide remaining usage").count())
    throw new Error("Unknown usage must be omitted");
  await page.goto("http://127.0.0.1:1420/overlay.html?busy=1");
  await page.getByLabel("Codex model").waitFor();
  await capture(page, { path: `${out}/codex-controls-overlay-busy.png` });
  if (await page.getByLabel("Codex model").isEnabled())
    throw new Error("Busy model control must be locked");
  await page.goto("http://127.0.0.1:1420/overlay.html?disconnected=1");
  await page.getByText("Codex", { exact: true }).waitFor();
  await page
    .getByText("AI is disabled until an account is connected.", { exact: true })
    .waitFor();
  if (
    (await page.getByLabel("Codex model").count()) ||
    (await page.getByLabel("Codex reasoning").count())
  )
    throw new Error(
      "Signed-out Codex model and reasoning controls must be hidden",
    );
  if (await page.getByLabel("AI model").count())
    throw new Error("Signed-out Codex cannot expose API models");
  await capture(page, { path: `${out}/codex-controls-overlay-signed-out.png` });
  await page.goto("http://127.0.0.1:1420/overlay.html?api-key=1");
  await page.getByText("API key", { exact: true }).waitFor();
  await capture(page, { path: `${out}/codex-controls-overlay-api.png` });
  console.log(JSON.stringify({ results }));
} finally {
  await browser.close();
}
