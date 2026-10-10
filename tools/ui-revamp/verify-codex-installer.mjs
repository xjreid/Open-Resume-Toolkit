// Memory-only UI fixtures; never invokes the native installer or an account.
import { createRequire } from "node:module";
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
const browser = await chromium.launch({
  headless: true,
  executablePath:
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
});
try {
  const context = await browser.newContext({
    reducedMotion: "reduce",
    deviceScaleFactor: 2,
  });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const results = [];
  for (const viewport of [
    { width: 1080, height: 760 },
    { width: 720, height: 520 },
  ]) {
    await page.setViewportSize(viewport);
    await page.goto("http://127.0.0.1:1420/preview/ai.html?disabled=1");
    await page.getByRole("button", { name: "Codex", exact: true }).click();
    const start = page.getByRole("button", {
      name: "Start Codex server",
      exact: true,
    });
    await page.waitForFunction(
      () =>
        !document.querySelector(".plan-server-actions button:last-child")
          ?.disabled,
    );
    if (
      await page
        .getByRole("button", { name: "Stop Codex server", exact: true })
        .count()
    )
      throw Error("Readiness must not enable Codex");
    if (
      (await page.getByText("Runtime installation details").count()) ||
      (await page
        .getByRole("button", { name: "Install Codex runtime", exact: true })
        .count())
    )
      throw Error("Verified installations must hide all setup guidance");
    await start.focus();
    await page.screenshot({
      path: `.impeccable/review/readiness-ready-${viewport.width}.png`,
    });
    await page.goto(
      "http://127.0.0.1:1420/preview/ai.html?runtime-missing=1&disabled=1",
    );
    await page.getByRole("button", { name: "Codex", exact: true }).click();
    const button = page.getByRole("button", {
      name: "Install Codex runtime",
      exact: true,
    });
    await button.waitFor();
    if (await start.count())
      throw Error("Unverified runtimes must not enable Codex");
    await button.scrollIntoViewIfNeeded();
    await button.focus();
    await page.screenshot({
      path: `.impeccable/review/installer-missing-${viewport.width}.png`,
    });
    await button.click();
    await page
      .getByRole("progressbar", { name: "Codex runtime download" })
      .waitFor();
    await page.screenshot({
      path: `.impeccable/review/installer-download-${viewport.width}.png`,
    });
    await page
      .getByText("Approve installation in the macOS prompt", { exact: false })
      .waitFor();
    await page.screenshot({
      path: `.impeccable/review/installer-approval-${viewport.width}.png`,
    });
    await page
      .getByRole("button", { name: "Retry runtime installation", exact: true })
      .waitFor();
    await page.screenshot({
      path: `.impeccable/review/installer-error-${viewport.width}.png`,
    });
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    );
    if (overflow) throw Error("Installer horizontal overflow");
    results.push({ viewport, overflow });
  }
  if (errors.length) throw Error(errors.join("\n"));
  console.log(
    JSON.stringify({
      results,
      pageErrors: errors,
      nativeInstallerInvoked: false,
    }),
  );
} finally {
  await browser.close();
}
