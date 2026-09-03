// Manual synthetic UI QA, not a native/vault test. Requires a local Vite preview
// server plus Playwright and an isolated headless Chromium/Chrome installation.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { join, resolve } from "node:path";
const require = createRequire(import.meta.url);
const { chromium } = require(process.env.ORT_PLAYWRIGHT_MODULE || "playwright");
const base = process.argv[2];
assert.equal(
  new URL(base).hostname,
  "127.0.0.1",
  "loopback synthetic server only",
);
const directory = resolve(process.argv[3]);
const kind = "dense";
const document = JSON.parse(
  readFileSync(join(directory, `${kind}.source.json`), "utf8"),
);
const receipt = JSON.parse(
  readFileSync(join(directory, `${kind}.json`), "utf8"),
);
const pdfBase64 = readFileSync(join(directory, `${kind}.pdf`)).toString(
  "base64",
);
const config = JSON.parse(
  readFileSync(
    new URL("../apps/desktop/src-tauri/tauri.conf.json", import.meta.url),
    "utf8",
  ),
);
const browser = await chromium.launch({
  headless: true,
  executablePath: process.env.ORT_BROWSER_EXECUTABLE,
});
try {
  const context = await browser.newContext({
    viewport: { width: 1280, height: 900 },
    serviceWorkers: "block",
  });
  const page = await context.newPage();
  const errors = [],
    requests = [],
    workers = [],
    workerClosures = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("request", (request) => requests.push(request.url()));
  page.on("worker", (worker) => {
    workers.push(worker.url());
    workerClosures.push(
      new Promise((resolve) => worker.once("close", resolve)),
    );
  });
  await context.route("**/*", async (route) => {
    assert.equal(
      new URL(route.request().url()).origin,
      new URL(base).origin,
      "no external requests",
    );
    const response = await route.fetch();
    await route.fulfill({
      response,
      headers: {
        ...response.headers(),
        "Content-Security-Policy": config.app.security.csp,
      },
    });
  });
  await page.addInitScript(
    ({ document, receipt, pdfBase64 }) => {
      let revision = 1;
      window.pdfQa = { exports: [], releases: [], renderRequests: [] };
      window.__TAURI_INTERNALS__ = {
        metadata: {
          currentWindow: { label: "main" },
          currentWebview: { label: "main" },
        },
        transformCallback: () => 1,
        unregisterCallback: () => {},
        invoke: async (command, args) => {
          const ok = (value) => ({ ok: true, value });
          if (command === "health")
            return ok({
              status: "ok",
              appVersion: "0.0.0-dev",
              profile: "development",
              storageStatus: "ready",
              contractVersion: 2,
            });
          if (command === "load_resume")
            return ok({
              draft: { revision, document },
              latestPublished: { revision: 1, document },
            });
          if (command === "plugin:event|listen") return 1;
          if (command === "plugin:event|unlisten") return null;
          if (command === "close_status") return ok({ pendingAttempt: null });
          if (command === "render_resume_pdf") {
            window.pdfQa.renderRequests.push(args.request.payload);
            return ok({
              renderId: "019a0000-0000-7000-8000-000000000001",
              source: args.request.payload.source,
              revision: args.request.payload.expectedRevision,
              generatedAtUnixMs: Date.now(),
              receipt,
              pdfBase64: window.pdfQa.corrupt
                ? "K" + pdfBase64.slice(1)
                : pdfBase64,
            });
          }
          if (command === "release_resume_pdf") {
            window.pdfQa.releases.push(args.request.payload);
            return ok({ released: true });
          }
          if (command === "export_resume_pdf") {
            window.pdfQa.exports.push(args.request.payload);
            return ok({ status: "cancelled" });
          }
          if (command === "save_resume") {
            revision++;
            return ok({ revision, document: args.request.payload.document });
          }
          throw new Error("Unexpected synthetic command");
        },
      };
    },
    { document, receipt, pdfBase64 },
  );
  await page.goto(base);
  await page
    .getByRole("button", { name: "Preview saved draft", exact: true })
    .click();
  await page
    .getByText("Preview ready. Export uses these exact PDF bytes.", {
      exact: true,
    })
    .waitFor();
  assert.equal(workers.length, 1);
  assert(workers[0].includes("pdf.worker.min-"));
  const panel = page.locator(".pdf-panel");
  await panel.screenshot({ path: join(directory, "preview-browser.png") });
  await page.getByRole("button", { name: "Next page", exact: true }).click();
  await page
    .getByText(`Page 2 of ${receipt.pageCount}`, { exact: true })
    .waitFor();
  await page.getByLabel("PDF zoom").selectOption("2");
  await page.waitForFunction(
    () => document.querySelector("canvas")?.width === 1224,
  );
  await page
    .getByRole("button", { name: "Export this preview (.pdf)", exact: true })
    .click();
  await page
    .getByText("PDF export cancelled. No file was created.", { exact: true })
    .first()
    .waitFor();
  assert.deepEqual(await page.evaluate(() => window.pdfQa.exports), [
    { renderId: "019a0000-0000-7000-8000-000000000001" },
  ]);
  await page
    .getByLabel("Full name", { exact: true })
    .fill("Synthetic changed name");
  assert(
    await page
      .getByRole("button", { name: "Export this preview (.pdf)", exact: true })
      .isDisabled(),
  );
  await page
    .getByText("This preview is out of date.", { exact: false })
    .waitFor();
  await page
    .getByRole("button", { name: "Clear preview", exact: true })
    .click();
  await page.waitForFunction(() => window.pdfQa.releases.length > 0);
  let deadline;
  try {
    await Promise.race([
      Promise.all(workerClosures),
      new Promise((_, reject) => {
        deadline = setTimeout(
          () => reject(new Error("worker did not terminate")),
          10000,
        );
      }),
    ]);
  } finally {
    clearTimeout(deadline);
  }
  assert.equal(await page.locator("canvas").count(), 0);
  assert.equal(
    await page.locator(".pdf-panel iframe, .pdf-panel a").count(),
    0,
  );
  await page.evaluate(() => {
    window.pdfQa.corrupt = true;
  });
  await page
    .getByRole("button", { name: "Preview published snapshot", exact: true })
    .click();
  await page
    .getByText(
      "The PDF could not be displayed. Export is disabled; the accessible content is available below.",
      { exact: true },
    )
    .waitFor();
  assert(
    await page
      .getByRole("button", { name: "Export this preview (.pdf)", exact: true })
      .isDisabled(),
  );
  assert.equal(
    workers.length,
    1,
    "corrupt bytes rejected before creating a worker",
  );
  assert.deepEqual(errors, []);
  assert(requests.every((url) => new URL(url).origin === new URL(base).origin));
  console.log(
    "Production CSP + local worker: preview, page/zoom, ticket-only export cancellation, stale edits, worker termination, corrupt-byte refusal and no external requests passed. Native IPC was mocked; no vault/dialog opened.",
  );
} finally {
  await browser.close();
}
