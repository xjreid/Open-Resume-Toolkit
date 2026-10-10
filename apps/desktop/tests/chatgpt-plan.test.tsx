import { JSDOM } from "jsdom";
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type * as Wire from "@ort/contracts/wire";
import { ChatGptPlanPage } from "../src/shared/ChatGptPlanPage";
import { CodexRuntimeInstaller } from "../src/shared/CodexRuntimeInstaller";
import { planErrorMessage } from "../src/shared/chatgpt-plan-presentation";

const native = vi.hoisted(() => ({
  invoke: vi.fn(),
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (
    name: string,
    handler: (event: { payload: unknown }) => void,
  ) => {
    native.listeners.set(name, handler);
    return () => native.listeners.delete(name);
  },
}));
let dom: JSDOM;
let root: Root;
let status: Wire.PlanStatus;
let installation: Wire.RuntimeInstallStatus;
let readiness: Wire.RuntimeReadiness;
const identifiers = [
  "gpt-5.6-luna",
  "gpt-5.6-terra",
  "gpt-5.6-sol",
  "gpt-6-luna",
  "gpt-6-sol",
  "gpt-6.1-sol",
];

beforeEach(() => {
  dom = new JSDOM("<div id='root'></div>", { url: "http://localhost/" });
  for (const name of [
    "window",
    "document",
    "navigator",
    "HTMLElement",
    "Event",
    "MouseEvent",
    "HTMLInputElement",
  ])
    vi.stubGlobal(
      name,
      name === "window"
        ? dom.window
        : (dom.window as unknown as Record<string, unknown>)[name],
    );
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  status = {
    settings: {
      cleanupRequired: false,
      connectionId: "019a0000-0000-7000-8000-000000000001",
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
    operationActive: false,
    runtimeVersion: "0.162.0",
    errorCode: null,
    models: identifiers.map((id) => ({
      id,
      name: `GPT-${id.slice(4).replace("-", " ")}`,
      supported: id !== "gpt-5.6-terra",
      explanation:
        id === "gpt-5.6-terra" ? "Unavailable in this runtime." : null,
      reasoningEfforts: ["medium", "high"],
    })),
    quota: {
      fetchedAtUnixMs: 1,
      windows: [
        {
          limitId: "codex",
          name: "Codex primary",
          window: "primary",
          remainingPercent: 20,
          windowDurationMinutes: 300,
          resetsAt: 2_000_000_000,
        },
      ],
    },
  };
  native.invoke.mockReset();
  native.listeners.clear();
  readiness = { ready: true, errorCode: null };
  installation = {
    phase: "idle",
    downloadedBytes: 0,
    totalBytes: 98089521,
    errorCode: null,
  };
  native.invoke.mockImplementation(async (command, args) => {
    if (command === "check_codex_runtime")
      return { ok: true, value: { ...readiness } };
    if (command === "load_codex_runtime_install")
      return { ok: true, value: installation };
    if (command === "install_codex_runtime") {
      readiness = { ready: true, errorCode: null };
      installation = { ...installation, phase: "complete" };
      status = { ...status, runtimeVersion: "0.162.0", errorCode: null };
      return { ok: true, value: installation };
    }
    if (
      command === "load_chatgpt_plan" &&
      status.settings.enabled &&
      !status.settings.cleanupRequired &&
      readiness.ready
    )
      status = { ...status, runtimeVersion: "0.162.0" };
    if (command === "save_chatgpt_plan") {
      status = {
        ...status,
        revision: (status.revision ?? 0) + 1,
        settings: {
          ...status.settings,
          enabled: args.request.enabled,
          model: args.request.model,
          reasoning: args.request.reasoning,
          reserveEnabled: args.request.reserveEnabled,
          reservePercent: args.request.reservePercent,
        },
      };
    }
    if (command === "save_chatgpt_plan" && !args.request.enabled)
      status = {
        ...status,
        connected: false,
        accountPlan: null,
        runtimeVersion: null,
        loginPending: false,
        quota: null,
        settings: { ...status.settings, connectionId: null },
      };
    if (command === "connect_chatgpt_plan")
      status = { ...status, loginPending: true };
    if (command === "cancel_chatgpt_login")
      status = { ...status, loginPending: false };
    if (command === "disconnect_chatgpt_plan")
      status = {
        ...status,
        connected: false,
        settings: { ...status.settings, connectionId: null },
        runtimeVersion: null,
        quota: null,
      };
    if (command === "stop_chatgpt_plan") return { ok: true, value: true };
    return { ok: true, value: status };
  });
  root = createRoot(document.getElementById("root")!);
});
afterEach(async () => {
  await act(async () => root.unmount());
  dom.window.close();
  vi.unstubAllGlobals();
});
async function render() {
  function Host() {
    const [value, setValue] = useState(status);
    return (
      <ChatGptPlanPage
        visible
        blocked={false}
        status={value}
        onStatus={setValue}
      />
    );
  }
  await act(async () => root.render(<Host />));
}
async function click(text: string) {
  const button = [...document.querySelectorAll("button")].find(
    (b) => b.textContent?.trim() === text,
  )!;
  expect(button).toBeDefined();
  expect(button.disabled).toBe(false);
  await act(async () => button.click());
}

it("checks installation while disabled and hides all setup for a verified, stopped runtime", async () => {
  status.settings.enabled = false;
  status.connected = false;
  status.settings.connectionId = null;
  status.runtimeVersion = null;
  await render();
  expect(native.invoke).toHaveBeenCalledWith("check_codex_runtime");
  expect(
    document.querySelector<HTMLButtonElement>(
      ".plan-server-actions button:last-child",
    )!.disabled,
  ).toBe(false);
  expect(
    document.querySelector('[aria-label="Codex runtime installation"]'),
  ).toBeNull();
  expect(document.body.textContent).not.toMatch(
    /Runtime installation details|official runtime download|Sign in to ChatGPT|Refresh connection/,
  );
  expect(native.invoke.mock.calls.map(([command]) => command)).not.toEqual(
    expect.arrayContaining([
      "install_codex_runtime",
      "connect_chatgpt_plan",
      "save_chatgpt_plan",
    ]),
  );
});

it("prompts for installation while disabled, without relying on a session error", async () => {
  status.settings.enabled = false;
  status.connected = false;
  status.settings.connectionId = null;
  status.runtimeVersion = null;
  readiness = { ready: false, errorCode: "PLAN_RUNTIME_MISSING" };
  await render();
  expect(document.body.textContent).toContain("Install Codex runtime");
  expect(
    document.querySelector(".plan-server-actions button:last-child"),
  ).toBeNull();
  expect(native.invoke).not.toHaveBeenCalledWith("install_codex_runtime");
});

it.each(["PLAN_RUNTIME_UNTRUSTED", "PLAN_RUNTIME_INCOMPATIBLE"])(
  "offers recovery for %s and still allows disabling",
  async (errorCode) => {
    status.connected = false;
    status.settings.enabled = true;
    readiness = { ready: false, errorCode };
    await render();
    expect(document.body.textContent).toContain("Install Codex runtime");
    const toggle = document.querySelector<HTMLButtonElement>(
      ".plan-server-actions button:last-child",
    )!;
    expect(toggle.disabled).toBe(false);
    await act(async () => toggle.click());
    expect(status.settings.enabled).toBe(false);
    expect(native.invoke).not.toHaveBeenCalledWith("connect_chatgpt_plan");
  },
);

it("waits for verification before allowing enable and never flashes an installer", async () => {
  status.settings.enabled = false;
  status.connected = false;
  status.settings.connectionId = null;
  status.runtimeVersion = null;
  status.connected = false;
  let finish!: (value: unknown) => void;
  const implementation = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command, args) =>
    command === "check_codex_runtime"
      ? new Promise((resolve) => {
          finish = resolve;
        })
      : implementation(command, args),
  );
  await render();
  const toggle = document.querySelector<HTMLButtonElement>(
    ".plan-server-actions button:last-child",
  )!;
  expect(toggle.disabled).toBe(true);
  expect(document.body.textContent).toContain("Checking Codex installation");
  expect(document.body.textContent).not.toContain("Install Codex runtime");
  await act(async () => finish({ ok: true, value: readiness }));
  expect(toggle.disabled).toBe(false);
  expect(document.body.textContent).not.toContain("Install Codex runtime");
});

it("recovers from an unavailable installation check without assuming the runtime is missing", async () => {
  status.settings.enabled = false;
  status.connected = false;
  status.settings.connectionId = null;
  status.runtimeVersion = null;
  status.connected = false;
  const implementation = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command, args) => {
    if (command === "check_codex_runtime") throw Error("bridge unavailable");
    return implementation(command, args);
  });
  await render();
  expect(document.body.textContent).toContain(
    "installation could not be checked",
  );
  expect(document.body.textContent).not.toContain("Install Codex runtime");
  native.invoke.mockImplementation(implementation);
  await click("Check installation");
  expect(
    document.querySelector<HTMLButtonElement>(
      ".plan-server-actions button:last-child",
    )!.disabled,
  ).toBe(false);
  expect(document.body.textContent).not.toContain(
    "installation could not be checked",
  );
});

it("rechecks on focus and ignores a late earlier installation result", async () => {
  status.settings.enabled = false;
  status.connected = false;
  status.settings.connectionId = null;
  status.runtimeVersion = null;
  status.connected = false;
  const checks: ((value: unknown) => void)[] = [];
  const implementation = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command, args) =>
    command === "check_codex_runtime"
      ? new Promise((resolve) => checks.push(resolve))
      : implementation(command, args),
  );
  await render();
  await act(async () => window.dispatchEvent(new Event("focus")));
  expect(checks).toHaveLength(2);
  await act(async () => checks[1]({ ok: true, value: readiness }));
  await act(async () =>
    checks[0]({
      ok: true,
      value: { ready: false, errorCode: "PLAN_RUNTIME_MISSING" },
    }),
  );
  expect(document.body.textContent).not.toContain("Install Codex runtime");
  expect(
    document.querySelector<HTMLButtonElement>(
      ".plan-server-actions button:last-child",
    )!.disabled,
  ).toBe(false);
});

it("shows all six exact models and disables unavailable choices and efforts", async () => {
  await render();
  const models = document.querySelector<HTMLSelectElement>(
    '[aria-label="Codex model"]',
  )!;
  expect([...models.options].slice(1).map((o) => o.value)).toEqual(identifiers);
  expect(
    models.querySelector<HTMLOptionElement>('[value="gpt-5.6-terra"]')!
      .disabled,
  ).toBe(true);
  const reasoning = document.querySelector<HTMLSelectElement>(
    '[aria-label="Codex reasoning"]',
  )!;
  expect([...reasoning.options].map((o) => [o.value, o.disabled])).toEqual([
    ["low", true],
    ["medium", false],
    ["high", false],
    ["xhigh", true],
  ]);
  expect(document.body.textContent).toContain("20% remaining");
  expect(document.body.textContent).toContain(
    "At exactly 20% remaining, dispatch is allowed",
  );
});
it("saves explicit activation with the frozen model, reasoning and reserve revision", async () => {
  status.settings.enabled = false;
  status.connected = false;
  status.settings.connectionId = null;
  status.runtimeVersion = null;
  await render();
  const toggle = document.querySelector<HTMLButtonElement>(
    ".plan-server-actions button:last-child",
  )!;
  expect(document.body.textContent!.includes("Stop Codex server")).toBe(false);
  await act(async () => toggle.click());
  expect(native.invoke).toHaveBeenCalledWith("save_chatgpt_plan", {
    request: {
      expectedRevision: 1,
      enabled: true,
      model: "gpt-6.1-sol",
      reasoning: "medium",
      reserveEnabled: true,
      reservePercent: 20,
    },
  });
  expect(document.body.textContent!.includes("Stop Codex server")).toBe(true);
});
it("locks model and reserve controls during operations while keeping Stop and Sign out usable", async () => {
  status.operationActive = true;
  status.settings.enabled = true;
  await render();
  expect(
    document.querySelector<HTMLSelectElement>('[aria-label="Codex model"]')!
      .disabled,
  ).toBe(true);
  expect(
    document.querySelector<HTMLInputElement>(
      '[aria-label="Usage reserve percentage"]',
    )!.disabled,
  ).toBe(true);
  await click("Stop AI work");
  expect(native.invoke).toHaveBeenCalledWith("stop_chatgpt_plan");
  await click("Sign out");
  expect(native.invoke).toHaveBeenCalledWith("disconnect_chatgpt_plan");
});
it("supports cancellable browser login and explains an unavailable runtime", async () => {
  status.connected = false;
  status.settings.connectionId = null;
  status.settings.enabled = true;
  await render();
  expect(document.body.textContent).toContain("Sign-in stays in memory only");
  expect(document.body.textContent).toContain(
    "without saving it to Keychain or a credential file",
  );
  await click("Sign in to ChatGPT");
  expect(document.body.textContent).toContain("Waiting for browser sign-in");
  await click("Cancel sign-in");
  expect(native.invoke).toHaveBeenCalledWith("cancel_chatgpt_login");
  expect(planErrorMessage("PLAN_RUNTIME_MISSING")).toContain(
    "Install the optional Codex runtime below",
  );
});

it("explains the session lifetime while connected and preserves preferences after reconnecting", async () => {
  status.settings.model = "gpt-6-sol";
  status.settings.reservePercent = 35;
  status.settings.enabled = true;
  await render();
  expect(document.body.textContent).toContain(
    "Your model, reserve settings, and activity history stay saved",
  );
  await click("Sign out");
  expect(status.settings.model).toBe("gpt-6-sol");
  expect(status.settings.reservePercent).toBe(35);
  expect(status.settings.enabled).toBe(true);
  expect(document.body.textContent).toContain("Stop Codex server");
  expect(document.body.textContent).toContain("Sign-in stays in memory only");
});

it("explains an authentication transport failure and allows a fresh sign-in", async () => {
  status.connected = false;
  status.settings.connectionId = null;
  status.errorCode = "PLAN_LOGIN_TRANSPORT_FAILED";
  status.settings.enabled = true;
  await render();
  expect(document.querySelector('[role="alert"]')?.textContent).toContain(
    "could not reach OpenAI’s authentication service",
  );
  expect(document.body.textContent).not.toContain("Sign-in was declined");
  await click("Sign in to ChatGPT");
  expect(native.invoke).toHaveBeenCalledWith("connect_chatgpt_plan");
  expect(
    native.invoke.mock.calls.some(([name]) => String(name).includes("ai_key")),
  ).toBe(false);
});
it("lets an expired connection disable plan usage without invoking a key command", async () => {
  status.connected = false;
  status.settings.enabled = true;
  status.errorCode = "PLAN_AUTH_REQUIRED";
  await render();
  expect(document.querySelector('[role="alert"]')?.textContent).toContain(
    "memory-only ChatGPT session has ended",
  );
  expect(document.querySelector('[role="alert"]')?.textContent).toContain(
    "Disable Codex to use API keys",
  );
  await act(async () =>
    document
      .querySelector<HTMLButtonElement>(
        ".plan-server-actions button:last-child",
      )!
      .click(),
  );
  expect(status.settings.enabled).toBe(false);
  expect(
    native.invoke.mock.calls.some(([name]) => String(name).includes("ai_key")),
  ).toBe(false);
});
it("allows enabling independently of model availability but blocks sign-in until credential cleanup", async () => {
  status.settings.enabled = false;
  status.connected = false;
  status.settings.connectionId = null;
  status.runtimeVersion = null;
  status.settings.model = null;
  status.models.forEach((m) => {
    m.supported = false;
    m.explanation = "This account does not have access.";
  });
  await render();
  expect(
    document.querySelector<HTMLButtonElement>(
      ".plan-server-actions button:last-child",
    )!.disabled,
  ).toBe(false);
  await act(async () => root.unmount());
  root = createRoot(document.getElementById("root")!);
  status.connected = false;
  status.settings.cleanupRequired = true;
  status.errorCode = "PLAN_CREDENTIAL_CLEANUP_REQUIRED";
  await render();
  expect(
    [...document.querySelectorAll("button")].find(
      (b) => b.textContent === "Sign in to ChatGPT",
    ),
  ).toBeUndefined();
  expect(document.body.textContent).toContain("Retry sign-out");
});

it("offers an opt-in verified runtime install without connecting or enabling a plan", async () => {
  status.settings.enabled = false;
  status.connected = false;
  status.settings.connectionId = null;
  status.runtimeVersion = null;
  status.errorCode = "PLAN_RUNTIME_MISSING";
  readiness = { ready: false, errorCode: "PLAN_RUNTIME_MISSING" };
  await render();
  expect(native.invoke).not.toHaveBeenCalledWith("install_codex_runtime");
  expect(document.body.textContent).toContain("administrator approval");
  await click("Install Codex runtime");
  expect(native.invoke).toHaveBeenCalledWith("install_codex_runtime");
  expect(native.invoke).not.toHaveBeenCalledWith("connect_chatgpt_plan");
  expect(status.settings.enabled).toBe(false);
  expect(
    document.querySelector('[aria-label="Codex runtime installation"]'),
  ).toBeNull();
  expect(document.body.textContent).not.toContain(
    "Runtime installation details",
  );
  expect(
    document.querySelector<HTMLButtonElement>(
      ".plan-server-actions button:last-child",
    )!.disabled,
  ).toBe(false);
});

it("refreshes readiness after the install command releases its operation lock", async () => {
  const completed = vi.fn();
  let finish!: (value: unknown) => void;
  let poll!: () => void;
  vi.spyOn(dom.window, "setInterval").mockImplementation((callback) => {
    poll = callback as () => void;
    return 1;
  });
  native.invoke.mockImplementation(async (command) => {
    if (command === "install_codex_runtime")
      return new Promise((resolve) => {
        finish = resolve;
      });
    return { ok: true, value: { ...installation } };
  });
  await act(async () =>
    root.render(
      <CodexRuntimeInstaller
        visible
        blocked={false}
        needed
        onBusy={() => {}}
        onComplete={completed}
      />,
    ),
  );
  await click("Install Codex runtime");
  installation = { ...installation, phase: "complete" };
  await act(async () => poll());
  expect(completed).not.toHaveBeenCalled();
  await act(async () => finish({ ok: true, value: installation }));
  expect(completed).toHaveBeenCalledTimes(1);
});

it("shows recoverable verification failures and does not expose an installer on unsupported platforms", async () => {
  status.connected = false;
  status.runtimeVersion = null;
  status.errorCode = "PLAN_RUNTIME_MISSING";
  readiness = { ready: false, errorCode: "PLAN_RUNTIME_MISSING" };
  installation = {
    ...installation,
    phase: "failed",
    errorCode: "PLAN_INSTALL_VERIFY_FAILED",
  };
  await render();
  expect(document.body.textContent).toContain("failed its size, checksum");
  expect(document.body.textContent).toContain("Retry runtime installation");
  expect(document.querySelector('[role="alert"]')).not.toBeNull();
  await act(async () => root.unmount());
  root = createRoot(document.getElementById("root")!);
  status.errorCode = "PLAN_PLATFORM_UNSUPPORTED";
  readiness = { ready: false, errorCode: "PLAN_PLATFORM_UNSUPPORTED" };
  installation.phase = "idle";
  await render();
  expect(document.body.textContent).not.toContain("Install Codex runtime");
});

it("shows download progress with cancellation and locks installation during AI work", async () => {
  status.connected = false;
  status.runtimeVersion = null;
  status.errorCode = "PLAN_RUNTIME_MISSING";
  readiness = { ready: false, errorCode: "PLAN_RUNTIME_MISSING" };
  installation = {
    ...installation,
    phase: "downloading",
    downloadedBytes: 49000000,
  };
  await render();
  expect(
    document.querySelector('progress[aria-label="Codex runtime download"]'),
  ).not.toBeNull();
  expect(document.body.textContent).toContain("Downloading the official");
  const implementation = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation(async (command, args) =>
    command === "cancel_codex_runtime_install"
      ? { ok: true, value: installation }
      : implementation(command, args),
  );
  await click("Cancel installation");
  expect(native.invoke).toHaveBeenCalledWith("cancel_codex_runtime_install");
  await act(async () => root.unmount());
  root = createRoot(document.getElementById("root")!);
  status.operationActive = true;
  installation.phase = "idle";
  await render();
  expect(
    [...document.querySelectorAll("button")].find(
      (b) => b.textContent === "Install Codex runtime",
    )?.disabled,
  ).toBe(true);
});

it("directs cancellation to macOS while protected installation is pending", async () => {
  status.connected = false;
  status.runtimeVersion = null;
  status.errorCode = "PLAN_RUNTIME_MISSING";
  readiness = { ready: false, errorCode: "PLAN_RUNTIME_MISSING" };
  installation.phase = "awaiting_approval";
  await render();
  expect(document.body.textContent).toContain(
    "Approve installation in the macOS prompt",
  );
  expect(document.body.textContent).not.toContain("Cancel installation");
});

it("keeps controls responsive during a slow quota refresh and ignores its stale response after saving", async () => {
  let finish!: (value: unknown) => void;
  const old = structuredClone(status);
  const implementation = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command, args) =>
    command === "load_chatgpt_plan"
      ? new Promise((resolve) => {
          finish = resolve;
        })
      : implementation(command, args),
  );
  await render();
  const toggle = document.querySelector<HTMLInputElement>(
    ".plan-reserve-toggle input",
  )!;
  expect(toggle.disabled).toBe(false);
  await act(async () => toggle.click());
  expect(toggle.checked).toBe(false);
  await act(async () => finish({ ok: true, value: old }));
  expect(toggle.checked).toBe(false);
  expect(
    native.invoke.mock.calls.filter(([name]) => name === "load_chatgpt_plan"),
  ).toHaveLength(1);
});

it("coalesces polls and blocks background refreshes during a settings mutation", async () => {
  const ticks: (() => void)[] = [];
  vi.spyOn(dom.window, "setInterval").mockImplementation((callback) => {
    ticks.push(callback as () => void);
    return ticks.length;
  });
  await render();
  let finishPoll!: (value: unknown) => void;
  let finishSave!: (value: unknown) => void;
  native.invoke.mockImplementation(
    (command) =>
      new Promise((resolve) => {
        if (command === "load_chatgpt_plan") finishPoll = resolve;
        else if (command === "save_chatgpt_plan") finishSave = resolve;
      }),
  );
  const initialLoads = native.invoke.mock.calls.filter(
    ([name]) => name === "load_chatgpt_plan",
  ).length;
  await act(async () => {
    ticks[0]();
    ticks[0]();
    ticks[0]();
  });
  expect(
    native.invoke.mock.calls.filter(([name]) => name === "load_chatgpt_plan"),
  ).toHaveLength(initialLoads + 1);
  const toggle = document.querySelector<HTMLInputElement>(
    ".plan-reserve-toggle input",
  )!;
  await act(async () => toggle.click());
  await act(async () =>
    finishPoll({ ok: true, value: structuredClone(status) }),
  );
  await act(async () => {
    ticks[0]();
    ticks[0]();
  });
  expect(
    native.invoke.mock.calls.filter(([name]) => name === "load_chatgpt_plan"),
  ).toHaveLength(initialLoads + 1);
  await act(async () =>
    finishSave({
      ok: true,
      value: {
        ...status,
        revision: 2,
        settings: { ...status.settings, reserveEnabled: false },
      },
    }),
  );
  expect(toggle.checked).toBe(false);
});

it("does not publish a late disconnect response after the page unmounts", async () => {
  const publish = vi.fn();
  let finish!: (value: unknown) => void;
  const implementation = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command, args) =>
    command === "disconnect_chatgpt_plan"
      ? new Promise((resolve) => {
          finish = resolve;
        })
      : implementation(command, args),
  );
  await act(async () =>
    root.render(
      <ChatGptPlanPage
        visible
        blocked={false}
        status={status}
        onStatus={publish}
      />,
    ),
  );
  await click("Sign out");
  await act(async () => root.unmount());
  const before = publish.mock.calls.length;
  await act(async () =>
    finish({ ok: true, value: { ...status, connected: false } }),
  );
  expect(publish).toHaveBeenCalledTimes(before);
  root = createRoot(document.getElementById("root")!);
});

it("explains the difference between protocol mismatch and a tool restriction", () => {
  expect(planErrorMessage("PLAN_PROTOCOL_INVALID")).toContain(
    "does not establish",
  );
  expect(planErrorMessage("PLAN_CONTAINMENT_VIOLATION")).toContain(
    "memory-only",
  );
  expect(planErrorMessage("PLAN_CONTAINMENT_VIOLATION")).toContain(
    "sign in again",
  );
});

it("syncs an overlay model and reasoning change into the main app immediately", async () => {
  await render();
  status = {
    ...status,
    revision: 2,
    settings: { ...status.settings, model: "gpt-6-sol", reasoning: "high" },
  };
  await act(async () =>
    native.listeners.get("ort:ai-model-changed")?.({ payload: null }),
  );
  expect(
    document.querySelector<HTMLSelectElement>('[aria-label="Codex model"]')!
      .value,
  ).toBe("gpt-6-sol");
  expect(
    document.querySelector<HTMLSelectElement>('[aria-label="Codex reasoning"]')!
      .value,
  ).toBe("high");
});

it("reloads a shared setting change arriving during a slow usage poll without publishing the stale snapshot", async () => {
  const old = structuredClone(status);
  let finish!: (value: unknown) => void;
  const original = native.invoke.getMockImplementation()!;
  let firstLoad = true;
  native.invoke.mockImplementation((command, args) => {
    if (command === "load_chatgpt_plan" && firstLoad) {
      firstLoad = false;
      return new Promise((resolve) => {
        finish = resolve;
      });
    }
    return original(command, args);
  });
  await render();
  status = {
    ...status,
    revision: 2,
    settings: { ...status.settings, model: "gpt-6-sol", reasoning: "high" },
  };
  native.invoke.mockImplementation(original);
  await act(async () =>
    native.listeners.get("ort:ai-model-changed")?.({ payload: null }),
  );
  await act(async () => finish({ ok: true, value: old }));
  expect(
    native.invoke.mock.calls.filter(([name]) => name === "load_chatgpt_plan"),
  ).toHaveLength(2);
  expect(
    document.querySelector<HTMLSelectElement>('[aria-label="Codex model"]')!
      .value,
  ).toBe("gpt-6-sol");
  expect(
    document.querySelector<HTMLSelectElement>('[aria-label="Codex reasoning"]')!
      .value,
  ).toBe("high");
});

it("shows Start Codex server before sign-in and starts independently of model selection", async () => {
  status.settings.enabled = false;
  status.connected = false;
  status.settings.connectionId = null;
  status.runtimeVersion = null;
  status.connected = false;
  status.settings.connectionId = null;
  status.settings.model = null;
  status.runtimeVersion = null;
  await render();
  const toggle = document.querySelector<HTMLButtonElement>(
    ".plan-server-actions button:last-child",
  )!;
  expect(toggle.disabled).toBe(false);
  expect(document.body.textContent).not.toContain("Sign in to ChatGPT");
  await act(async () => toggle.click());
  expect(status.settings.enabled).toBe(true);
  const connect = [...document.querySelectorAll("button")].find(
    (b) => b.textContent === "Sign in to ChatGPT",
  )!;
  expect(connect.disabled).toBe(false);
  expect(native.invoke).not.toHaveBeenCalledWith("connect_chatgpt_plan");
});

it("starts a previously enabled server on a signed-out app relaunch", async () => {
  status.connected = false;
  status.settings.connectionId = null;
  status.settings.enabled = true;
  status.runtimeVersion = null;
  await render();
  expect(status.settings.enabled).toBe(true);
  await click("Sign in to ChatGPT");
  expect(status.settings.enabled).toBe(true);
});

it("allows stopping the Codex server during an active operation", async () => {
  status.settings.enabled = true;
  status.operationActive = true;
  await render();
  const toggle = document.querySelector<HTMLButtonElement>(
    ".plan-server-actions button:last-child",
  )!;
  expect(toggle.disabled).toBe(false);
  await act(async () => toggle.click());
  expect(status.settings.enabled).toBe(false);
  expect(native.invoke).toHaveBeenCalledWith("save_chatgpt_plan", {
    request: expect.objectContaining({ enabled: false }),
  });
});

it("keeps one connection refresh at the top and refreshes both installation and account usage", async () => {
  await render();
  const refreshes = [...document.querySelectorAll("button")].filter(
    (button) => button.textContent?.trim() === "Refresh connection",
  );
  expect(refreshes).toHaveLength(1);
  expect(refreshes[0].closest(".plan-server-actions")).not.toBeNull();
  expect(document.querySelector(".plan-status-strip")?.textContent).toMatch(
    /Codex serverRunning.*ChatGPT accountSigned in/,
  );
  native.invoke.mockClear();
  await click("Refresh connection");
  expect(native.invoke).toHaveBeenCalledWith("check_codex_runtime");
  expect(native.invoke).toHaveBeenCalledWith("load_chatgpt_plan", {
    request: { refreshUsage: true },
  });
});

it("stops a signed-in server, clears account controls, and hides connection refresh", async () => {
  await render();
  await click("Stop Codex server");
  expect(status.connected).toBe(false);
  expect(status.runtimeVersion).toBeNull();
  expect(status.settings.connectionId).toBeNull();
  expect(document.querySelector(".plan-status-strip")?.textContent).toMatch(
    /Codex serverStopped.*ChatGPT accountSigned out/,
  );
  expect(document.querySelector('[aria-label="Codex model"]')).toBeNull();
  expect(
    document.querySelector('[aria-label="Usage reserve percentage"]'),
  ).toBeNull();
  expect(document.body.textContent).not.toContain("Refresh connection");
  expect(document.body.textContent).toContain("Start Codex server");
});

it("waits for a live server before showing sign-in and leaves recovery controls available on startup failure", async () => {
  status.connected = false;
  status.settings.connectionId = null;
  status.runtimeVersion = null;
  status.errorCode = "PLAN_RUNTIME_UNAVAILABLE";
  const original = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command, args) =>
    command === "load_chatgpt_plan"
      ? Promise.resolve({ ok: true, value: status })
      : original(command, args),
  );
  await render();
  expect(document.body.textContent).toContain("Unavailable");
  expect(
    [...document.querySelectorAll("button")].some(
      (button) => button.textContent === "Sign in to ChatGPT",
    ),
  ).toBe(false);
  expect(document.body.textContent).toContain("Refresh connection");
  await click("Stop Codex server");
  expect(status.settings.enabled).toBe(false);
});
