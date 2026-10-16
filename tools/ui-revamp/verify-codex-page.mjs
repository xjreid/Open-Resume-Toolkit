// Main-window Codex verification with synthetic, memory-only native responses.
// Never launches Codex, installs a runtime, or connects a real account.
import { createRequire } from "node:module";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";
import { homedir } from "node:os";
const require = createRequire(import.meta.url);
const { chromium } = require(
  process.env.ORT_PLAYWRIGHT_MODULE ??
    resolve(
      homedir(),
      ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright",
    ),
);
const axePath = require.resolve(
  "../../apps/desktop/node_modules/axe-core/axe.min.js",
);
const out = resolve(".impeccable/review/codex-page");
mkdirSync(out, { recursive: true });
const browser = await chromium.launch({
  headless: true,
  executablePath:
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
});
const context = await browser.newContext({
  reducedMotion: "reduce",
  deviceScaleFactor: 1,
  timezoneId: "America/New_York",
});
const page = await context.newPage();
page.setDefaultTimeout(7000);
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));
const results = [];
const sectionWidths = new Map();
async function open(scenario) {
  const query =
    scenario === "missing"
      ? "runtime-missing=1&disabled=1"
      : scenario === "stopped"
        ? "disabled=1"
        : "codex=1";
  await page.goto(`http://127.0.0.1:1420/preview/ai.html?${query}`);
  await page.getByRole("button", { name: "ChatGPT", exact: true }).waitFor();
  await page.evaluate(async (scenario) => {
    const native = window.__TAURI_INTERNALS__;
    const original = native.invoke;
    let plan = structuredClone((await original("load_chatgpt_plan")).value);
    if (["signed-out", "pending", "unavailable"].includes(scenario)) {
      plan.connected = false;
      plan.accountPlan = null;
      plan.settings.connectionId = null;
      plan.quota = null;
      plan.loginPending = scenario === "pending";
    }
    if (scenario === "unavailable") {
      plan.runtimeVersion = null;
      plan.errorCode = "PLAN_RUNTIME_UNAVAILABLE";
    }
    if (scenario === "unknown-usage") plan.quota = null;
    if (scenario === "busy") plan.operationActive = true;
    window.codexPreviewCalls = [];
    native.invoke = async (name, args) => {
      window.codexPreviewCalls.push(name);
      if (name === "load_chatgpt_plan") {
        if (plan.settings.enabled && scenario !== "unavailable")
          plan.runtimeVersion = "0.162.0";
      } else if (name === "save_chatgpt_plan") {
        const { expectedRevision, ...settings } = args.request;
        plan.settings = { ...plan.settings, ...settings };
        plan.revision = expectedRevision + 1;
        if (!settings.enabled) {
          plan.connected = false;
          plan.loginPending = false;
          plan.runtimeVersion = null;
          plan.quota = null;
          plan.accountPlan = null;
          plan.settings.connectionId = null;
        }
      } else if (name === "disconnect_chatgpt_plan") {
        plan.connected = false;
        plan.loginPending = false;
        plan.runtimeVersion = null;
        plan.quota = null;
        plan.accountPlan = null;
        plan.settings.connectionId = null;
      } else if (name === "connect_chatgpt_plan") {
        plan.loginPending = true;
      } else if (name === "cancel_chatgpt_login") {
        plan.loginPending = false;
      } else return original(name, args);
      return { ok: true, value: structuredClone(plan) };
    };
  }, scenario);
  await page.getByRole("button", { name: "ChatGPT", exact: true }).click();
  await page.waitForFunction(
    () =>
      !document.querySelector(".plan-server-actions button:last-child")
        ?.disabled || document.querySelector(".plan-setup:not([hidden])"),
  );
  await page
    .getByText("Refreshing…", { exact: true })
    .waitFor({ state: "hidden" });
}
async function capture(name, bottom = false) {
  await page.evaluate(async (bottom) => {
    await document.fonts.ready;
    window.scrollTo(0, 0);
    const content = document.querySelector(".app-content");
    content.scrollTop = bottom === true ? content.scrollHeight : 0;
    if (typeof bottom === "string") {
      const target = document.querySelector(bottom);
      content.scrollTop =
        target.getBoundingClientRect().top -
        content.getBoundingClientRect().top -
        parseFloat(getComputedStyle(content).scrollPaddingBlockStart);
    }
    await new Promise((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(resolve)),
    );
  }, bottom);
  await page.screenshot({ path: `${out}/${name}.png`, animations: "disabled" });
  const widths = await page
    .locator(".chatgpt-plan-page section:visible")
    .evaluateAll((sections) =>
      sections.map((section) => section.getBoundingClientRect().width),
    );
  const viewportWidth = page.viewportSize().width;
  const expectedWidth = sectionWidths.get(viewportWidth) ?? widths[0];
  sectionWidths.set(viewportWidth, expectedWidth);
  if (widths.some((width) => Math.abs(width - expectedWidth) > 1))
    throw Error(
      `Section widths changed in ${name}: ${widths}, expected ${expectedWidth}`,
    );
  const overflow = await page.evaluate(() => ({
    viewport: innerWidth,
    document: document.documentElement.scrollWidth,
    content: document.querySelector(".app-content").clientWidth,
    scroll: document.querySelector(".app-content").scrollWidth,
  }));
  if (
    overflow.document > overflow.viewport ||
    overflow.scroll > overflow.content + 1
  )
    throw Error(`Horizontal overflow: ${JSON.stringify(overflow)}`);
  await page.addScriptTag({ path: axePath });
  const violations = await page.evaluate(async () =>
    (
      await axe.run(document.querySelector(".chatgpt-plan-page"), {
        runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21aa"] },
      })
    ).violations.map(({ id, nodes }) => ({
      id,
      targets: nodes.map((node) => node.target),
    })),
  );
  if (violations.length)
    throw Error(`Accessibility: ${JSON.stringify(violations)}`);
  results.push({
    name,
    widths,
    overflow,
    accessibilityViolations: violations.length,
  });
}
try {
  for (const viewport of [
    { width: 1080, height: 760 },
    { width: 720, height: 520 },
  ]) {
    await page.setViewportSize(viewport);
    for (const scenario of [
      "connected",
      "stopped",
      "missing",
      "signed-out",
      "pending",
      "unknown-usage",
      "unavailable",
      "busy",
    ]) {
      await open(scenario);
      await capture(`${scenario}-${viewport.width}`);
      if (["connected", "unknown-usage", "missing"].includes(scenario))
        await capture(`${scenario}-bottom-${viewport.width}`, true);
      if (scenario === "connected") {
        await capture(
          `connected-account-${viewport.width}`,
          ".plan-usage-settings",
        );
        await capture(`connected-reserve-${viewport.width}`, ".plan-reserve");
        const header = await page.locator(".plan-server").boundingBox();
        const tabs = await page
          .locator('[aria-label="AI section"]')
          .boundingBox();
        if (
          !header ||
          !tabs ||
          Math.abs(header.y - (tabs.y + tabs.height + 14)) > 1
        )
          throw Error(
            "Runtime header did not stay below the AI tabs on scroll",
          );
      }
      const refresh = page.getByRole("button", {
        name: "Refresh connection",
        exact: true,
      });
      if (
        (await refresh.count()) !==
        (["stopped", "missing"].includes(scenario) ? 0 : 1)
      )
        throw Error("Refresh must appear once, only while enabled");
      if (scenario === "stopped") {
        if (
          await page
            .getByRole("button", { name: "Log in with OpenAI", exact: true })
            .count()
        )
          throw Error("Sign-in before server startup");
        await page.getByRole("button", { name: "Launch", exact: true }).click();
        await page
          .getByRole("button", { name: "Log in with OpenAI", exact: true })
          .waitFor();
        await page.getByRole("button", { name: "Quit", exact: true }).waitFor();
        await page
          .getByRole("button", { name: "Log in with OpenAI", exact: true })
          .click();
        await page
          .getByRole("button", { name: "Cancel sign-in", exact: true })
          .click();
        await page.getByRole("button", { name: "Quit", exact: true }).click();
        await page
          .getByRole("button", { name: "Launch", exact: true })
          .waitFor();
        if (await refresh.count())
          throw Error("Refresh must disappear on stop");
      }
      if (scenario === "connected") {
        const connectionDetails = page.locator(".plan-connection-details");
        if (await connectionDetails.evaluate((details) => details.open))
          throw Error("Connection details should start collapsed");
        const summary = connectionDetails.locator("summary");
        await summary.focus();
        await page.keyboard.press("Enter");
        if (!(await connectionDetails.evaluate((details) => details.open)))
          throw Error("Connection details cannot be opened by keyboard");
        await page
          .getByText("Sign-in stays in memory only", { exact: false })
          .waitFor();
        await page.keyboard.press("Enter");
        if (await connectionDetails.evaluate((details) => details.open))
          throw Error("Connection details cannot be closed by keyboard");
        await page
          .getByRole("switch", { name: "Keep a usage reserve" })
          .uncheck();
        const disabledReserveWidth = await page
          .locator(".plan-usage-settings")
          .evaluate((section) => section.getBoundingClientRect().width);
        if (
          Math.abs(disabledReserveWidth - sectionWidths.get(viewport.width)) > 1
        )
          throw Error("Disabling reserve changed section width");
        if (await page.getByLabel("Usage reserve percentage").isEnabled())
          throw Error("Reserve is editable while disabled");
        await page
          .getByRole("switch", { name: "Keep a usage reserve" })
          .check();
        await page.waitForFunction(
          () =>
            !document.querySelector('[aria-label="Usage reserve percentage"]')
              .disabled &&
            !document.querySelector('[aria-label="Codex model"]').disabled,
        );
        await page.getByLabel("Codex model").selectOption("gpt-6-sol");
        await page.waitForFunction(
          () =>
            document.querySelector('[aria-label="Codex model"]').value ===
            "gpt-6-sol",
        );
        await page.getByLabel("Usage reserve percentage").fill("35");
        await page
          .getByRole("button", { name: "Save reserve", exact: true })
          .click();
        await page
          .getByText("ORT pauses new AI work below 35% remaining.", {
            exact: false,
          })
          .waitFor();
        await page
          .getByRole("button", { name: "Sign out", exact: true })
          .click();
        await page
          .getByRole("button", { name: "Log in with OpenAI", exact: true })
          .waitFor();
        if (
          !(await page
            .getByRole("button", { name: "Quit", exact: true })
            .count())
        )
          throw Error("Sign-out disabled server");
        if (await page.getByLabel("Codex model").count())
          throw Error("Signed-out account controls visible");
      }
      if (scenario === "busy") {
        if (await page.getByLabel("Codex model").isEnabled())
          throw Error("Busy model is editable");
        if (
          !(await page
            .getByRole("button", { name: "Quit", exact: true })
            .isEnabled())
        )
          throw Error("Stop unavailable during AI work");
      }
    }
  }
  if (errors.length) throw Error(errors.join("\n"));
  console.log(
    JSON.stringify({
      results,
      pageErrors: errors,
      liveRuntimeOrAccountUsed: false,
    }),
  );
} finally {
  await browser.close();
}
