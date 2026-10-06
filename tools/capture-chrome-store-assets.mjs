// Render the approved vector identity and photograph the real packaged capture UI.
// Uses a disposable headless Chrome profile and synthetic job text. Does not open ORT.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { createServer } from "node:http";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const source = resolve(root, "packaging/extension/chrome/store-assets");
const output = resolve(root, "artifacts/extension/chrome/store-listing");
mkdirSync(source, { recursive: true });
mkdirSync(output, { recursive: true });
const logoPath = resolve(root, "Aesthetic/Logo/open-folio-app.svg");
const markPath = resolve(root, "Aesthetic/Logo/open-folio-reversed.svg");
const inner = (svg) =>
  svg.replace(/^<svg[^>]*>/, "").replace(/<\/svg>\s*$/, "");
const logo = inner(readFileSync(logoPath, "utf8"));
const mark = inner(readFileSync(markPath, "utf8"));
const svg = (width, height, contents) =>
  `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">${contents}</svg>`;
const vectors = new Map([
  [
    "store-icon-128",
    {
      width: 128,
      height: 128,
      transparent: true,
      svg: svg(
        128,
        128,
        `<svg x="16" y="16" width="96" height="96" viewBox="64 64 896 896">${logo}</svg>`,
      ),
    },
  ],
  [
    "small-promo-440x280",
    {
      width: 440,
      height: 280,
      svg: svg(
        440,
        280,
        `<rect width="440" height="280" fill="#204d52"/><svg x="130" y="62" width="180" height="156" viewBox="0 0 120 104">${mark}</svg>`,
      ),
    },
  ],
  [
    "marquee-promo-1400x560",
    {
      width: 1400,
      height: 560,
      svg: svg(
        1400,
        560,
        `<rect width="1400" height="560" fill="#204d52"/><svg x="520" y="124" width="360" height="312" viewBox="0 0 120 104">${mark}</svg>`,
      ),
    },
  ],
]);
for (const [name, vector] of vectors)
  writeFileSync(join(source, `${name}.svg`), `${vector.svg}\n`);
const css = `@font-face{font-family:Hanken;src:url('/hanken.ttf')}*{box-sizing:border-box}body{margin:0;background:#f2f6f7;color:#1c3135;font:18px/1.5 Hanken,Arial,sans-serif}header{height:92px;padding:0 76px;display:flex;align-items:center;justify-content:space-between;background:white;border-bottom:1px solid #d8e3e5}.brand{font-size:25px;font-weight:650}header nav{display:flex;gap:30px;color:#566c70;font-size:16px}.page{width:1128px;margin:38px auto;display:grid;grid-template-columns:792px 304px;gap:32px}article,aside{background:white;border:1px solid #d8e3e5;border-radius:10px}article{padding:36px 40px}aside{padding:24px;height:max-content;color:#566c70;font-size:16px}.eyebrow{font-size:14px;color:#566c70;margin:0 0 12px}h1{font-size:34px;line-height:1.15;margin:0 0 14px;font-weight:650}h2{font-size:22px;margin:27px 0 10px;font-weight:650}p{margin:0 0 15px}ul{padding-left:24px;margin:0 0 20px}li{margin-bottom:10px}.meta{display:flex;gap:20px;font-size:15px;color:#566c70;margin-bottom:30px}.aside-title{font-size:18px;font-weight:650;color:#1c3135}dl{margin:22px 0}dt{font-size:13px;margin-top:17px}dd{margin:2px 0;color:#1c3135}button{background:#28676c;color:white;font:600 16px Hanken;border:0;border-radius:7px;padding:12px 25px;width:100%;margin-top:10px}.note{font-size:13px;color:#566c70;border-top:1px solid #d8e3e5;padding-top:20px;margin-top:24px}.spacer{height:80px}`;
const html = `<!doctype html><html lang="en"><meta charset="utf-8"><title>Senior Software Engineer · Example Studio</title><link rel="stylesheet" href="/job.css"><header><div class="brand">Example Studio</div><nav><span>Our work</span><span>Our team</span><span>Careers</span></nav></header><div class="page"><article><p class="eyebrow">ENGINEERING · OPEN ROLE</p><h1>Senior Software Engineer</h1><div class="meta"><span>Portland, OR / Remote</span><span>Full time</span></div><section id="capture"><h2>Build thoughtful tools for people</h2><p>We are looking for a software engineer who enjoys turning complex workflows into clear, accessible experiences. Join a small team building practical tools that help people do their best work.</p><h2>What you will do</h2><ul><li>Build and maintain reliable web applications with TypeScript and React.</li><li>Work with designers to create accessible, responsive interfaces.</li><li>Improve performance, write focused tests, and review code with care.</li></ul></section><h2>What you bring</h2><ul><li>Experience shipping applications that people use every day.</li><li>Clear communication and a collaborative approach to problem solving.</li><li>A strong understanding of browser APIs, testing, and web accessibility.</li></ul><h2>How we work</h2><p>We value thoughtful decisions, useful feedback, and time to focus. Our team shares responsibility for quality from the first prototype through ongoing maintenance.</p><h2>Benefits and support</h2><p>Flexible working hours, professional development, and a supportive team. We welcome different backgrounds and perspectives.</p><section id="long-end"><h2>Ready to join us?</h2><p>Tell us about a project you are proud of and what you learned while building it. We look forward to hearing from you.</p></section><p class="note">Sample job listing for demonstrating Open Resume Toolkit. Example Studio and this role are fictional.</p></article><aside><p class="aside-title">Senior Software Engineer</p><dl><dt>TEAM</dt><dd>Engineering</dd><dt>LOCATION</dt><dd>Portland, OR / Remote</dd><dt>EMPLOYMENT</dt><dd>Full time</dd></dl><button>Apply for this role</button><p class="note">This is a sample careers page. No application is submitted.</p></aside></div><div class="spacer"></div></html>`;
const server = createServer((request, response) => {
  if (request.url === "/hanken.ttf") {
    response.setHeader("Content-Type", "font/ttf");
    response.end(
      readFileSync(
        resolve(root, "apps/desktop/src/assets/fonts/HankenGrotesk.ttf"),
      ),
    );
    return;
  }
  if (request.url === "/job.css") {
    response.setHeader("Content-Type", "text/css");
    response.end(css);
    return;
  }
  const vector = vectors.get(request.url?.slice(1).replace(/\.svg$/, ""));
  if (vector) {
    response.setHeader("Content-Type", "image/svg+xml");
    response.end(vector.svg);
    return;
  }
  response.setHeader("Content-Type", "text/html; charset=utf-8");
  response.setHeader(
    "Content-Security-Policy",
    "default-src 'none'; style-src 'self'; font-src 'self'",
  );
  response.end(html);
});
await new Promise((ready, reject) => {
  server.once("error", reject);
  server.listen(0, "127.0.0.1", ready);
});
const origin = `http://127.0.0.1:${server.address().port}`;
const profile = mkdtempSync(join(tmpdir(), "ort-store-assets-"));
const browser = spawn(
  process.env.ORT_BROWSER_EXECUTABLE ||
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
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
browser.stdio[4].setEncoding("utf8");
browser.stdio[4].on("data", (chunk) => {
  buffer += chunk;
  let end;
  while ((end = buffer.indexOf("\0")) >= 0) {
    const message = JSON.parse(buffer.slice(0, end));
    buffer = buffer.slice(end + 1);
    const entry = pending.get(message.id);
    if (!entry) continue;
    clearTimeout(entry.timer);
    pending.delete(message.id);
    if (message.error)
      entry.reject(new Error(`${entry.method}: ${message.error.message}`));
    else entry.resolve(message.result);
  }
});
function send(method, params = {}, sessionId) {
  return new Promise((ready, reject) => {
    const id = ++nextId;
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(`${method} timed out`));
    }, 10000);
    pending.set(id, { method, timer, resolve: ready, reject });
    browser.stdio[3].write(
      `${JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) })}\0`,
    );
  });
}
async function evaluate(session, expression) {
  const result = await send(
    "Runtime.evaluate",
    { expression, awaitPromise: true, returnByValue: true },
    session,
  );
  assert.equal(
    result.exceptionDetails,
    undefined,
    JSON.stringify(result.exceptionDetails),
  );
  return result.result.value;
}
async function waitFor(read) {
  for (let attempt = 0; attempt < 80; attempt++) {
    const result = await read();
    if (result) return result;
    await new Promise((ready) => setTimeout(ready, 100));
  }
  throw new Error("Store capture timed out");
}
async function attach(targetId) {
  const { sessionId } = await send("Target.attachToTarget", {
    targetId,
    flatten: true,
  });
  await send("Runtime.enable", {}, sessionId);
  return sessionId;
}
async function viewport(session, width, height) {
  await send(
    "Emulation.setDeviceMetricsOverride",
    { width, height, deviceScaleFactor: 1, mobile: false },
    session,
  );
}
const assets = [];
async function photograph(session, name, width, height) {
  const { data } = await send(
    "Page.captureScreenshot",
    { format: "png", captureBeyondViewport: false },
    session,
  );
  const bytes = Buffer.from(data, "base64");
  assert.equal(bytes.readUInt32BE(16), width);
  assert.equal(bytes.readUInt32BE(20), height);
  assert.equal(bytes[24], 8);
  if (name !== "store-icon-128")
    assert.equal(
      bytes[25],
      2,
      "Store screenshots/promos must be RGB without alpha",
    );
  writeFileSync(join(output, `${name}.png`), bytes);
  assets.push({
    file: `${name}.png`,
    width,
    height,
    colorType: bytes[25],
    sha256: createHash("sha256").update(bytes).digest("hex"),
  });
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
  const { targetId } = await send("Target.createTarget", {
    url: "about:blank",
  });
  const page = await attach(targetId);
  for (const [name, vector] of vectors) {
    await viewport(page, vector.width, vector.height);
    await send(
      "Emulation.setDefaultBackgroundColorOverride",
      { color: { r: 0, g: 0, b: 0, a: vector.transparent ? 0 : 1 } },
      page,
    );
    await send("Page.navigate", { url: `${origin}/${name}.svg` }, page);
    await waitFor(() =>
      evaluate(
        page,
        `document.querySelector('svg')?.getAttribute('width') === '${vector.width}'`,
      ),
    );
    await photograph(page, name, vector.width, vector.height);
  }
  await send("Emulation.setDefaultBackgroundColorOverride", {}, page);
  await viewport(page, 1280, 800);
  const { id } = await send("Extensions.loadUnpacked", {
    path: resolve(root, "apps/extension/dist/chrome"),
  });
  await send(
    "Page.navigate",
    { url: `${origin}/careers/senior-software-engineer` },
    page,
  );
  await send("Target.activateTarget", { targetId });
  await waitFor(() => evaluate(page, "!!document.querySelector('#capture')"));
  await evaluate(page, "document.fonts.ready.then(()=>true)");
  const workerTarget = await waitFor(async () =>
    (await send("Target.getTargets")).targetInfos.find(
      (target) =>
        target.type === "service_worker" &&
        target.url === `chrome-extension://${id}/service-worker.js`,
    ),
  );
  const worker = await attach(workerTarget.targetId);
  await waitFor(() =>
    evaluate(
      worker,
      "typeof chrome !== 'undefined' && !!chrome.runtime?.connectNative",
    ),
  );
  await evaluate(
    worker,
    `globalThis.fixture={commands:[],captures:[]};chrome.runtime.connectNative=()=>{let reply,disconnect;return {onMessage:{addListener:fn=>reply=fn},onDisconnect:{addListener:fn=>disconnect=fn},disconnect:()=>disconnect?.(),postMessage:message=>queueMicrotask(()=>{let response={ok:true,protocolVersion:1};if(message.kind==='bridge.poll')response.value={ready:true,commands:fixture.commands};if(message.kind==='capture.selection'){fixture.captures.push(message);fixture.commands=[];response.value={requestId:message.requestId};}reply(response);})};};chrome.alarms.create('ort-bridge-reconnect',{delayInMinutes:0.001,periodInMinutes:0.5});`,
  );
  async function arm() {
    await evaluate(
      worker,
      "fixture.commands=[{kind:'capture.start',sessionId:crypto.randomUUID(),target:'job',expiresAt:Date.now()+120000}]",
    );
    await waitFor(() =>
      evaluate(page, "!!document.querySelector('[data-ort-capture]')"),
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
  const box = await evaluate(
    page,
    "(()=>{const r=document.querySelector('#capture').getBoundingClientRect();return {left:r.left-4,top:r.top-4,right:r.right+4,bottom:r.bottom+4}})()",
  );
  await arm();
  await click(box.left, box.top);
  await send(
    "Input.dispatchMouseEvent",
    { type: "mouseMoved", x: box.right, y: box.bottom },
    page,
  );
  await waitFor(() =>
    evaluate(
      page,
      "[...CSS.highlights].some(([name,highlight])=>name.startsWith('ort-capture-')&&[...highlight].some(range=>range.toString().includes('TypeScript')))",
    ),
  );
  await photograph(page, "screenshot-01-select-job", 1280, 800);
  await click(box.right, box.bottom);
  await waitFor(() => evaluate(worker, "fixture.captures.length===1"));
  assert.match(
    await evaluate(worker, "fixture.captures[0].payload.text"),
    /TypeScript and React/,
  );
  await arm();
  await click(box.left, box.top);
  const before = await evaluate(page, "scrollY");
  await send(
    "Input.dispatchMouseEvent",
    { type: "mouseWheel", x: 500, y: 500, deltaX: 0, deltaY: 510 },
    page,
  );
  await waitFor(async () => (await evaluate(page, "scrollY")) > before + 400);
  const end = await evaluate(
    page,
    "document.querySelector('#long-end').getBoundingClientRect().bottom+4",
  );
  await send(
    "Input.dispatchMouseEvent",
    { type: "mouseMoved", x: box.right, y: Math.min(end, 740) },
    page,
  );
  await waitFor(() =>
    evaluate(
      page,
      "[...CSS.highlights].some(([name,highlight])=>name.startsWith('ort-capture-')&&[...highlight].some(range=>range.toString().includes('Ready to join us')))",
    ),
  );
  await photograph(page, "screenshot-02-scroll-job", 1280, 800);
  await click(box.right, Math.min(end, 740));
  await waitFor(() => evaluate(worker, "fixture.captures.length===2"));
  const longText = await evaluate(worker, "fixture.captures[1].payload.text");
  assert.match(longText, /Build thoughtful tools/);
  assert.match(longText, /Ready to join us/);
  writeFileSync(
    join(output, "asset-receipt.json"),
    JSON.stringify(
      {
        browser: version.product,
        approvedIdentity: "Open Folio, October 4, 2026",
        logoSource: "Aesthetic/Logo/open-folio-app.svg",
        logoSha256: createHash("sha256")
          .update(readFileSync(logoPath))
          .digest("hex"),
        markSource: "Aesthetic/Logo/open-folio-reversed.svg",
        screenshots:
          "Unmodified production extension JavaScript in real Chrome, synthetic careers page, mocked local native transport. No installed desktop app launched; no personal data.",
        assets,
      },
      null,
      2,
    ) + "\n",
  );
  console.log(`Prepared ${assets.length} verified store assets in ${output}`);
} finally {
  for (const entry of pending.values()) {
    clearTimeout(entry.timer);
    entry.reject(new Error("Store capture ended"));
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
