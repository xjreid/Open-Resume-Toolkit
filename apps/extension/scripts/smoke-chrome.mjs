// Real Chrome QA in a disposable profile. Default uses a mocked final transport;
// dev-bridge mode uses the actual native host and desktop encrypted intake.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import {
  existsSync,
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";

const development = process.argv[2] === "dev-bridge";
const qaRoot = process.env.ORT_DEV_QA_ROOT;
if (development && (!qaRoot || !process.env.ORT_DEV_QA_HOST))
  throw new Error(
    "Run dev Chrome QA through the ignored Rust integration test.",
  );
const root = resolve(import.meta.dirname, "../../..");
const executable =
  process.env.ORT_BROWSER_EXECUTABLE ||
  (process.platform === "darwin"
    ? "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
    : "google-chrome");
const fixtureCss = `body{font:18px Arial;margin:24px}p{white-space:pre-wrap;width:max-content;max-width:95vw}
  #hidden{display:none}.transparent{opacity:0}#clipped{height:0;overflow:hidden}
  #pane{height:240px;width:350px;overflow:auto;border:1px solid black}#pane p{margin:12px}
  .spacer-700{height:700px}.spacer-650{height:650px}.spacer-500{height:500px}.spacer-80{height:80px}
  .long-job{position:relative}.outside-scrolled{position:absolute;left:500px;top:700px}`;
const server = createServer((request, response) => {
  if (request.url === "/fixture.css") {
    response.setHeader("Content-Type", "text/css");
    response.end(fixtureCss);
    return;
  }
  response.setHeader("Content-Type", "text/html; charset=utf-8");
  response.setHeader(
    "Content-Security-Policy",
    "default-src 'none'; style-src 'self'; script-src 'nonce-ort-fixture'",
  );
  response.end(`<!doctype html><html lang="en"><title>Synthetic job</title><link rel="stylesheet" href="/fixture.css"><body><h1>Job page</h1><p id="selection">Synthetic<span> job description</span><br>Résumé experience required.</p>
  <span id="hidden">Hidden secret must not be captured</span><span class="transparent">Invisible secret</span><div id="clipped">Clipped secret</div>
  <p id="question">Why this team?</p><p id="partial">One two three</p><p>Unselected private-looking text must not be captured.</p><input type="password" value="synthetic-password">
  <div class="spacer-700"></div><section class="long-job">
  <p id="long-start">Beginning of long job</p><div class="spacer-650"></div>
  <p id="long-middle">Middle of long job</p><input type="password" value="scrolled-password"><span hidden>Hidden scrolled secret</span>
  <p class="outside-scrolled">Outside scrolled secret</p><div class="spacer-650"></div>
  <p id="long-end">End of long job</p></section><div class="spacer-80"></div>
  <div id="pane"><p id="pane-start">Beginning of panel job</p><div class="spacer-500"></div>
  <p>Middle of panel job</p><div class="spacer-500"></div><p id="pane-end">End of panel job</p></div>
  <div class="spacer-700"></div><script nonce="ort-fixture">globalThis.pageClicks=0;document.addEventListener('click',()=>pageClicks++);</script></body></html>`);
});
await new Promise((ready, reject) => {
  server.once("error", reject);
  server.listen(0, "127.0.0.1", ready);
});
const profile = mkdtempSync(join(tmpdir(), "ort-chrome-qa-"));
if (development) {
  mkdirSync(join(profile, "NativeMessagingHosts"));
  const { key } = JSON.parse(
    (await import("node:fs")).readFileSync(
      resolve(root, "apps/extension/manifest/chrome-dev-key.json"),
      "utf8",
    ),
  );
  const digest = (await import("node:crypto"))
    .createHash("sha256")
    .update(Buffer.from(key, "base64"))
    .digest()
    .subarray(0, 16);
  const id = [...digest]
    .map((byte) => String.fromCharCode(97 + (byte >> 4), 97 + (byte & 15)))
    .join("");
  writeFileSync(
    join(profile, "NativeMessagingHosts/com.openresumetoolkit.dev.json"),
    JSON.stringify({
      name: "com.openresumetoolkit.dev",
      description: "Isolated development QA",
      path: process.env.ORT_DEV_QA_HOST,
      type: "stdio",
      allowed_origins: [`chrome-extension://${id}/`],
    }),
  );
}
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = spawn(
  executable,
  [
    "--headless=new",
    "--remote-debugging-pipe",
    "--enable-unsafe-extension-debugging",
    `--user-data-dir=${profile}`,
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-background-networking",
    "--disable-component-update",
    "--disable-sync",
    "--disable-default-apps",
    "--disable-features=MediaRouter,OptimizationHints",
    "about:blank",
  ],
  { stdio: ["ignore", "ignore", "ignore", "pipe", "pipe"] },
);
let nextId = 0,
  buffer = "";
const pending = new Map();
const exceptions = [];
browser.stdio[4].setEncoding("utf8");
browser.stdio[4].on("data", (chunk) => {
  buffer += chunk;
  let end;
  while ((end = buffer.indexOf("\0")) >= 0) {
    const message = JSON.parse(buffer.slice(0, end));
    buffer = buffer.slice(end + 1);
    if (message.method === "Runtime.exceptionThrown")
      exceptions.push(message.params.exceptionDetails.text);
    const entry = pending.get(message.id);
    if (entry) {
      clearTimeout(entry.timer);
      pending.delete(message.id);
      if (message.error)
        entry.reject(new Error(`${entry.method}: ${message.error.message}`));
      else entry.resolve(message.result);
    }
  }
});
function send(method, params = {}, sessionId) {
  return new Promise((resolvePromise, reject) => {
    const id = ++nextId;
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`${method} timed out`));
    }, 10_000);
    pending.set(id, { method, timer, resolve: resolvePromise, reject });
    browser.stdio[3].write(
      `${JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) })}\0`,
    );
  });
}
async function evaluate(sessionId, expression) {
  const response = await send(
    "Runtime.evaluate",
    { expression, awaitPromise: true, returnByValue: true, userGesture: true },
    sessionId,
  );
  assert.equal(
    response.exceptionDetails,
    undefined,
    JSON.stringify(response.exceptionDetails),
  );
  return response.result.value;
}
async function waitFor(read, description) {
  for (let attempt = 0; attempt < 50; attempt++) {
    const value = await read();
    if (value) return value;
    await new Promise((ready) => setTimeout(ready, 100));
  }
  throw new Error(`Waiting for ${description} timed out`);
}
async function attach(targetId) {
  const { sessionId } = await send("Target.attachToTarget", {
    targetId,
    flatten: true,
  });
  await send("Runtime.enable", {}, sessionId);
  return sessionId;
}
let stopped = false;
const onExit = new Promise((ready) =>
  browser.once("exit", () => {
    stopped = true;
    ready();
  }),
);
try {
  const version = await send("Browser.getVersion");
  const { id } = await send("Extensions.loadUnpacked", {
    path: resolve(
      root,
      development
        ? "apps/extension/dist/chrome-dev-bridge"
        : "apps/extension/dist/chrome",
    ),
  });
  const { targetId } = await send("Target.createTarget", {
    url: `${origin}/job?jobId=42&token=secret&utm_source=qa#section`,
  });
  const page = await attach(targetId);
  await send("Target.activateTarget", { targetId });
  await waitFor(
    () => evaluate(page, '!!document.querySelector("#selection")'),
    "fixture",
  );
  await evaluate(
    page,
    "(()=>{const range=document.createRange();range.selectNodeContents(document.querySelector('h1'));CSS.highlights.set('page-owned',new Highlight(range));})()",
  );
  const target = await waitFor(
    async () =>
      (await send("Target.getTargets")).targetInfos.find(
        (target) =>
          target.type === "service_worker" &&
          target.url === `chrome-extension://${id}/service-worker.js`,
      ),
    "worker",
  );
  const worker = await attach(target.targetId);
  await waitFor(
    () =>
      evaluate(
        worker,
        "typeof chrome !== 'undefined' && !!chrome.runtime?.connectNative",
      ),
    "worker APIs",
  );
  if (!development) {
    await evaluate(
      worker,
      `globalThis.qa={commands:[],captures:[],events:[]};chrome.runtime.connectNative=()=>{let reply,disconnect;return {onMessage:{addListener:fn=>reply=fn},onDisconnect:{addListener:fn=>disconnect=fn},disconnect:()=>disconnect?.(),postMessage:message=>queueMicrotask(()=>{let response={ok:true,protocolVersion:1};if(message.kind==='bridge.poll')response.value={ready:true,commands:qa.commands};if(message.kind==='capture.selection'){qa.captures.push(message);qa.commands=[];response.value={requestId:message.requestId};}if(message.kind==='capture.event'){qa.events.push(message);if(message.phase==='failed'||message.phase==='cancelled')qa.commands=[];}reply(response);})};};`,
    );
  }
  const mark = (name, text = "") => {
    if (development) writeFileSync(join(qaRoot, name), text);
  };
  const flag = async (name) => {
    if (development) await waitFor(() => existsSync(join(qaRoot, name)), name);
  };
  async function armed(name, target = "job") {
    if (development) await flag(`${name}-armed`);
    else
      await evaluate(
        worker,
        `qa.commands=[{kind:'capture.start',sessionId:crypto.randomUUID(),target:${JSON.stringify(target)},expiresAt:Date.now()+60000}];`,
      );
    await waitFor(
      () => evaluate(page, '!!document.querySelector("[data-ort-capture]")'),
      `${name} capture mode`,
    );
  }
  async function bounds(selector, startOffset, endOffset) {
    return evaluate(
      page,
      `(()=>{const element=document.querySelector(${JSON.stringify(selector)});const range=document.createRange();${startOffset === undefined ? "range.selectNodeContents(element);" : `range.setStart(element.firstChild,${startOffset});range.setEnd(element.firstChild,${endOffset});`}const rect=range.getBoundingClientRect();return {left:rect.left-1,top:rect.top-2,right:rect.right+1,bottom:rect.bottom+2};})()`,
    );
  }
  async function click(x, y) {
    await send(
      "Input.dispatchMouseEvent",
      { type: "mousePressed", x, y, button: "left", clickCount: 1 },
      page,
    );
    await send(
      "Input.dispatchMouseEvent",
      { type: "mouseReleased", x, y, button: "left", clickCount: 1 },
      page,
    );
  }
  async function wheel(x, y, deltaY) {
    await send(
      "Input.dispatchMouseEvent",
      { type: "mouseWheel", x, y, deltaX: 0, deltaY },
      page,
    );
  }
  async function scrollTo(selector) {
    await evaluate(
      page,
      `window.scrollTo({top:document.querySelector(${JSON.stringify(selector)}).getBoundingClientRect().top+scrollY-60,behavior:'instant'})`,
    );
    await evaluate(
      page,
      "new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))",
    );
  }
  async function corners(rect) {
    await click(rect.left, rect.top);
    await send(
      "Input.dispatchMouseEvent",
      { type: "mouseMoved", x: rect.right, y: rect.bottom },
      page,
    );
  }
  async function highlighted() {
    return evaluate(
      page,
      "[...CSS.highlights].filter(([name])=>name.startsWith('ort-capture-')).flatMap(([,highlight])=>[...highlight].map(range=>range.toString())).join('')",
    );
  }
  async function preview(expected) {
    await waitFor(
      async () => (await highlighted()) === expected,
      `live highlighted text: ${expected}`,
    );
  }
  async function complete(rect, expected) {
    await corners(rect);
    if (expected !== undefined) await preview(expected);
    await click(rect.right, rect.bottom);
    await waitFor(
      () => evaluate(page, '!document.querySelector("[data-ort-capture]")'),
      "box cleanup",
    );
    assert.equal(
      await evaluate(page, "pageClicks"),
      0,
      "Capture clicks must not activate page controls",
    );
    assert.equal(
      await evaluate(
        page,
        "[...CSS.highlights.keys()].some(name=>name.startsWith('ort-capture-'))",
      ),
      false,
    );
    assert.equal(
      await evaluate(page, "CSS.highlights.has('page-owned')"),
      true,
    );
  }
  async function captured(count) {
    if (!development)
      await waitFor(
        async () => (await evaluate(worker, "qa.captures.length")) === count,
        "native capture acknowledgement",
      );
  }
  const output = resolve(root, "artifacts/extension/chrome");
  mkdirSync(output, { recursive: true });
  mark("browser-ready", origin);
  await armed("cancel");
  const job = await bounds("#selection");
  await corners(job);
  await preview("Synthetic job descriptionRésumé experience required.");
  await evaluate(
    page,
    "new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))",
  );
  const image = await send("Page.captureScreenshot", {}, page);
  writeFileSync(
    join(
      output,
      development ? "capture-box-chrome-dev.png" : "capture-box-chrome.png",
    ),
    Buffer.from(image.data, "base64"),
  );
  await wheel(job.left + 20, job.top + 20, 300);
  await waitFor(
    () => evaluate(page, "scrollY > 100"),
    "page scroll after first corner",
  );
  assert.equal(
    await evaluate(page, '!!document.querySelector("[data-ort-capture]")'),
    true,
  );
  mark("cancel-selecting");
  if (!development)
    await evaluate(
      worker,
      "qa.commands=[{kind:'capture.cancel',sessionId:qa.commands[0].sessionId}];",
    );
  await waitFor(
    () => evaluate(page, '!document.querySelector("[data-ort-capture]")'),
    "overlay cancellation removed box",
  );
  assert.equal(
    development ? true : (await evaluate(worker, "qa.captures.length")) === 0,
    true,
  );
  mark("cancelled");
  assert.equal(
    await evaluate(
      page,
      "[...CSS.highlights.keys()].some(name=>name.startsWith('ort-capture-'))",
    ),
    false,
  );
  assert.equal(await evaluate(page, "CSS.highlights.has('page-owned')"), true);
  await evaluate(page, "window.scrollTo({top:0,behavior:'instant'})");
  await armed("job");
  await complete(job, "Synthetic job descriptionRésumé experience required.");
  await captured(1);
  mark("job-ready", origin);
  await flag("job-reviewed");
  await armed("question", "question");
  await complete(await bounds("#question"), "Why this team?");
  await captured(2);
  mark("question-ready");
  await flag("question-reviewed");
  await armed("partial", "question");
  const wholeWordLine = await bounds("#partial");
  const partial = await bounds("#partial", 4, 7);
  await click(partial.left, partial.top);
  await send(
    "Input.dispatchMouseEvent",
    { type: "mouseMoved", x: wholeWordLine.right, y: partial.bottom },
    page,
  );
  await preview("two three");
  await send(
    "Input.dispatchMouseEvent",
    { type: "mouseMoved", x: partial.right, y: partial.bottom },
    page,
  );
  await preview("two");
  await click(partial.right, partial.bottom);
  await captured(3);
  mark("partial-ready");
  await flag("partial-reviewed");
  await scrollTo("#long-start");
  await armed("scrolled");
  const longStart = await bounds("#long-start");
  await click(longStart.left, longStart.top);
  const delta = await evaluate(
    page,
    "document.querySelector('#long-end').getBoundingClientRect().bottom-200",
  );
  const previousScroll = await evaluate(page, "scrollY");
  await wheel(longStart.left + 20, longStart.top + 20, delta);
  await waitFor(
    async () => (await evaluate(page, "scrollY")) > previousScroll + 1000,
    "long job scroll",
  );
  assert.equal(
    await evaluate(page, '!!document.querySelector("[data-ort-capture]")'),
    true,
  );
  assert.equal(
    await evaluate(
      page,
      "document.querySelector('#long-start').getBoundingClientRect().bottom < 0",
    ),
    true,
  );
  const longEnd = await bounds("#long-end");
  await send(
    "Input.dispatchMouseEvent",
    { type: "mouseMoved", x: longStart.left + 350, y: longEnd.bottom },
    page,
  );
  await preview("End of long job");
  await evaluate(
    page,
    "new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))",
  );
  const scrolledImage = await send("Page.captureScreenshot", {}, page);
  writeFileSync(
    join(
      output,
      development
        ? "capture-scrolled-chrome-dev.png"
        : "capture-scrolled-chrome.png",
    ),
    Buffer.from(scrolledImage.data, "base64"),
  );
  await click(longStart.left + 350, longEnd.bottom);
  await captured(4);
  mark("scrolled-ready");
  await flag("scrolled-reviewed");
  await scrollTo("#pane");
  await armed("pane", "question");
  const paneStart = await bounds("#pane-start");
  await click(paneStart.left, paneStart.top);
  const pane = await evaluate(
    page,
    "(()=>{const pane=document.querySelector('#pane'),r=pane.getBoundingClientRect();return {left:r.left,top:r.top,max:pane.scrollHeight-pane.clientHeight};})()",
  );
  await wheel(pane.left + 100, pane.top + 100, pane.max);
  await waitFor(
    () => evaluate(page, "document.querySelector('#pane').scrollTop > 800"),
    "job panel scroll",
  );
  assert.equal(
    await evaluate(page, '!!document.querySelector("[data-ort-capture]")'),
    true,
  );
  const paneEnd = await bounds("#pane-end");
  await send(
    "Input.dispatchMouseEvent",
    { type: "mouseMoved", x: pane.left + 320, y: paneEnd.bottom },
    page,
  );
  await preview("End of panel job");
  await evaluate(
    page,
    "new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)))",
  );
  const paneImage = await send("Page.captureScreenshot", {}, page);
  writeFileSync(
    join(
      output,
      development ? "capture-panel-chrome-dev.png" : "capture-panel-chrome.png",
    ),
    Buffer.from(paneImage.data, "base64"),
  );
  await click(pane.left + 320, paneEnd.bottom);
  await captured(5);
  mark("pane-ready");
  await flag("pane-reviewed");
  assert.equal(await evaluate(page, "pageClicks"), 0);
  assert.equal(
    await evaluate(
      page,
      "[...CSS.highlights.keys()].some(name=>name.startsWith('ort-capture-'))",
    ),
    false,
  );
  assert.equal(await evaluate(page, "CSS.highlights.has('page-owned')"), true);
  await evaluate(page, "window.scrollTo({top:0,behavior:'instant'})");
  await armed("empty", "question");
  await complete({ left: 500, top: 450, right: 550, bottom: 500 });
  if (!development)
    await waitFor(
      async () =>
        await evaluate(
          worker,
          "qa.events.some(event=>event.code==='EMPTY_SELECTION')",
        ),
      "empty-box failure",
    );
  mark("empty-picked");
  await armed("reload", "question");
  await send(
    "Page.navigate",
    { url: `${origin}/job?jobId=42&token=secret&utm_source=qa#section` },
    page,
  );
  await waitFor(
    () =>
      evaluate(
        page,
        '!!document.querySelector("#selection") && !document.querySelector("[data-ort-capture]")',
      ),
    "reload cleanup",
  );
  if (!development)
    await waitFor(
      async () =>
        await evaluate(
          worker,
          "qa.events.some(event=>event.code==='PAGE_CHANGED')",
        ),
      "navigation failure",
    );
  mark("page-reloaded");
  await flag("disconnected");
  if (!development) {
    const captures = await evaluate(worker, "qa.captures");
    assert.equal(
      captures[0].payload.text,
      "Synthetic job description\nRésumé experience required.",
    );
    assert.equal(captures[0].payload.url, `${origin}/job?jobId=42`);
    assert.equal(captures[1].payload.text, "Why this team?");
    assert.equal(captures[1].payload.target, "question");
    assert.equal(captures[2].payload.text, "two");
    assert.equal(
      captures[3].payload.text,
      "Beginning of long job\nMiddle of long job\nEnd of long job",
    );
    assert.equal(
      captures[4].payload.text,
      "Beginning of panel job\nMiddle of panel job\nEnd of panel job",
    );
    assert.equal(captures[4].payload.target, "question");
    assert.equal(
      JSON.stringify(captures).includes("synthetic-password"),
      false,
    );
    assert.equal(JSON.stringify(captures).includes("secret"), false);
  }
  assert.equal(await evaluate(worker, "typeof chrome.storage"), "undefined");
  assert.deepEqual(exceptions, []);
  writeFileSync(
    join(output, development ? "chrome-dev-qa.json" : "chrome-qa.json"),
    JSON.stringify(
      {
        browser: version.product,
        extensionId: id,
        scope:
          "Overlay-authorized two-click capture, live exact text highlights including expansion/shrinkage and cleanup without removing page-owned highlights, scroll-anchored rectangle, cancellation after scrolling, whole-page and panel scroll capture including off-screen start/middle text, job/question/partial text, URL cleanup, empty region, same-URL reload, empty extension storage",
        nativeTransport: development
          ? "Real native port, authenticated development socket and encrypted desktop review"
          : "Mocked native responses",
        passed: true,
      },
      null,
      2,
    ) + "\n",
  );
  console.log(
    `Chrome rectangle capture passed (${version.product}); native transport ${development ? "real" : "mocked"}.`,
  );
} finally {
  for (const entry of pending.values()) {
    clearTimeout(entry.timer);
    entry.reject(new Error("QA ended"));
  }
  pending.clear();
  browser.kill("SIGTERM");
  if (!stopped)
    await Promise.race([
      onExit,
      new Promise((ready) => setTimeout(ready, 3000)),
    ]);
  if (!stopped) {
    browser.kill("SIGKILL");
    await onExit;
  }
  server.close();
  rmSync(profile, { recursive: true, force: true });
}
