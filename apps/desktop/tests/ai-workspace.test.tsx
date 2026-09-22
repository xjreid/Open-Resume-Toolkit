import { JSDOM } from "jsdom";
import axe from "axe-core";
import { act } from "react";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { AiWorkspace } from "../src/shared/AiWorkspace";

const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: native.invoke,
  Channel: class<T> {
    onmessage: (event: T) => void = () => {};
  },
}));

let dom: JSDOM;
let root: Root;
const connection = {
  ok: true,
  value: {
    primaryCredentialId: "fixture-id",
    keys: [
      {
        credentialId: "fixture-id",
        createdAt: "2026-09-01T12:00:00Z",
        provider: "openai",
        preset: "balanced",
        paused: false,
        removed: false,
        cleanupRequired: false,
      },
      {
        credentialId: "second-id",
        createdAt: "2026-09-02T12:00:00Z",
        provider: "anthropic",
        preset: "balanced",
        paused: false,
        removed: false,
        cleanupRequired: false,
      },
    ],
  },
};
const emptyMonitoring = {
  ok: true,
  value: {
    logicalOperations: 0,
    attempts: 0,
    usage: { inputTokens: 0, outputTokens: 0 },
    costByCurrencyMicros: {},
    byProvider: {},
    byStatus: {},
    partial: false,
    unknownCount: 0,
    byModel: {},
    byPreset: {},
    byOperationType: {},
    timeBuckets: [],
    unresolvedReservedMicros: 0,
    estimatedCostMicros: 0,
    currency: null,
  },
};
const preview = {
  ok: true,
  value: {
    credentialId: "fixture-id",
    provider: "openai",
    model: "fixture-model",
    currency: "USD",
    estimatedInputTokens: 512,
    maximumCostMicros: 123_000,
  },
};
const catalog = {
  ok: true,
  value: {
    catalogId: "fixture-catalog",
    expiresAt: "2027-01-01T00:00:00Z",
    entries: [
      {
        provider: "open_ai",
        model: "fixture-model",
        preset: "balanced",
        disabled: false,
        operations: ["credential_test"],
        maxInputTokens: 64000,
        maxOutputTokens: 6000,
        currency: "USD",
        prices: [{ category: "input", microsPerMillion: 2_000_000 }],
        source: "official-pricing-fixture",
        verifiedAt: "2026-09-15T00:00:00Z",
        effectiveFrom: "2026-09-15T00:00:00Z",
        effectiveTo: null,
      },
    ],
  },
};

beforeEach(async () => {
  vi.useFakeTimers({ toFake: ["Date"] });
  vi.setSystemTime(new Date(2026, 8, 17, 12));
  dom = new JSDOM(
    "<!doctype html><html><body><div id='root'></div></body></html>",
    { url: "http://localhost/" },
  );
  vi.stubGlobal("window", dom.window);
  vi.stubGlobal("document", dom.window.document);
  vi.stubGlobal("navigator", dom.window.navigator);
  vi.stubGlobal("Node", dom.window.Node);
  vi.stubGlobal("HTMLElement", dom.window.HTMLElement);
  vi.stubGlobal("HTMLInputElement", dom.window.HTMLInputElement);
  vi.stubGlobal("Event", dom.window.Event);
  vi.stubGlobal("InputEvent", dom.window.InputEvent);
  vi.stubGlobal("MouseEvent", dom.window.MouseEvent);
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  native.invoke.mockReset();
  native.invoke.mockImplementation((command: string) => {
    if (command === "load_ai_catalog") return Promise.resolve(catalog);
    if (command === "load_ai_connection") return Promise.resolve(connection);
    if (command === "load_ai_monitoring")
      return Promise.resolve(emptyMonitoring);
    if (command === "load_ai_key_settings")
      return Promise.resolve({
        ok: true,
        value: { cap: null, lifetimeSpendByCurrencyMicros: { USD: 250_000 } },
      });
    if (command === "load_ai_general_settings")
      return Promise.resolve({
        ok: true,
        value: { cap: null, lifetimeSpendByCurrencyMicros: { USD: 500_000 } },
      });
    if (command === "load_ai_retention")
      return Promise.resolve({
        ok: true,
        value: { policy: "retain_until_cleared", removedOperations: 0 },
      });
    if (command === "preview_ai_test") return Promise.resolve(preview);
    if (command === "test_ai_connection")
      return Promise.resolve({
        ok: true,
        value: {
          confirmed: true,
          effectiveModel: "fixture-model",
          usageComplete: true,
          estimatedCostMicros: 12_000,
          usage: {
            inputTokens: 10,
            cachedInputTokens: 2,
            cacheWriteTokens: 0,
            outputTokens: 3,
            reasoningTokens: 1,
          },
        },
      });
    if (command === "cancel_ai_test")
      return Promise.resolve({ ok: true, value: true });
    return Promise.reject(new Error(`unexpected ${command}`));
  });
  const { createRoot } = await import("react-dom/client");
  root = createRoot(document.getElementById("root")!);
  await act(async () => {
    root.render(<AiWorkspace blocked={false} />);
  });
});

afterEach(async () => {
  await act(async () => root.unmount());
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

async function click(label: string) {
  const button = [...document.querySelectorAll("button")].find(
    (element) => element.textContent?.trim() === label,
  );
  if (!button) throw new Error(`missing button ${label}`);
  await act(async () => {
    button.dispatchEvent(new dom.window.MouseEvent("click", { bubbles: true }));
  });
}

it("requires an estimate review before the synthetic provider request", async () => {
  await clickAccessible("Test OpenAI key");
  expect(document.body.textContent).toContain(
    "Conservative maximum reservation: 0.1230 USD",
  );
  expect(document.body.textContent).toContain("At most 512 tokens");
  expect(document.body.textContent).toContain(
    "retention and privacy practices",
  );
  expect(native.invoke).not.toHaveBeenCalledWith(
    "test_ai_connection",
    expect.anything(),
  );
  await click("Confirm and send test");
  expect(native.invoke).toHaveBeenCalledWith(
    "test_ai_connection",
    expect.objectContaining({
      credentialId: "fixture-id",
      expectedModel: "fixture-model",
      expectedMaximumCostMicros: 123_000,
    }),
  );
  expect(document.body.textContent).toContain("Synthetic request completed");
  expect(document.body.textContent).toContain("10 input, 2 cached input");
});

it("shows test and removal details in click-away popups outside key cards", async () => {
  await clickAccessible("Test OpenAI key");
  const testPopup = document.querySelector(
    '[aria-label="Confirm synthetic provider request"]',
  )!;
  expect(testPopup.closest(".ai-key-row")).toBeNull();
  await act(async () => {
    testPopup.parentElement!.dispatchEvent(
      new dom.window.Event("pointerdown", { bubbles: true }),
    );
  });
  expect(
    document.querySelector('[aria-label="Confirm synthetic provider request"]'),
  ).toBeNull();

  await clickAccessible("Remove OpenAI key");
  const removePopup = document.querySelector(
    '[aria-label="Confirm provider credential removal"]',
  )!;
  expect(removePopup.closest(".ai-key-row")).toBeNull();
  await act(async () => {
    removePopup.parentElement!.dispatchEvent(
      new dom.window.Event("pointerdown", { bubbles: true }),
    );
  });
  expect(
    document.querySelector(
      '[aria-label="Confirm provider credential removal"]',
    ),
  ).toBeNull();
});

it("keeps cancellation available while a request is active", async () => {
  let complete!: (value: unknown) => void;
  native.invoke.mockImplementation((command: string) => {
    if (command === "load_ai_catalog") return Promise.resolve(catalog);
    if (command === "load_ai_connection") return Promise.resolve(connection);
    if (command === "load_ai_monitoring")
      return Promise.resolve(emptyMonitoring);
    if (command === "load_ai_caps")
      return Promise.resolve({ ok: true, value: [] });
    if (command === "load_ai_retention")
      return Promise.resolve({
        ok: true,
        value: { policy: "retain_until_cleared", removedOperations: 0 },
      });
    if (command === "preview_ai_test") return Promise.resolve(preview);
    if (command === "test_ai_connection")
      return new Promise((resolve) => {
        complete = resolve;
      });
    if (command === "cancel_ai_test")
      return Promise.resolve({ ok: true, value: true });
    return Promise.reject(new Error(`unexpected ${command}`));
  });
  await clickAccessible("Test OpenAI key");
  await click("Confirm and send test");
  await click("Cancel active test");
  expect(native.invoke).toHaveBeenCalledWith("cancel_ai_test");
  await act(async () => {
    complete({ ok: false, error: { code: "AI_CANCELLED" } });
  });
  expect(document.body.textContent).toContain("Synthetic request cancelled");
});

it("surfaces a missing-usage failure without treating reserved exposure as zero", async () => {
  native.invoke.mockImplementation((command: string) => {
    if (command === "load_ai_catalog") return Promise.resolve(catalog);
    if (command === "load_ai_connection") return Promise.resolve(connection);
    if (command === "load_ai_monitoring")
      return Promise.resolve(emptyMonitoring);
    if (command === "load_ai_caps")
      return Promise.resolve({ ok: true, value: [] });
    if (command === "load_ai_retention")
      return Promise.resolve({
        ok: true,
        value: { policy: "retain_until_cleared", removedOperations: 0 },
      });
    if (command === "preview_ai_test") return Promise.resolve(preview);
    if (command === "test_ai_connection")
      return Promise.resolve({
        ok: false,
        error: { code: "AI_USAGE_UNKNOWN" },
      });
    return Promise.reject(new Error(`unexpected ${command}`));
  });
  await clickAccessible("Test OpenAI key");
  await click("Confirm and send test");
  expect(document.body.textContent).toContain("reservation remains unresolved");
});

it("saves an inline cap on click-away without changing primary", async () => {
  const previous = native.invoke.getMockImplementation()!;
  let savedCap: Record<string, unknown> | null = null;
  native.invoke.mockImplementation((command: string, args?: any) => {
    if (command === "load_ai_key_settings")
      return Promise.resolve({
        ok: true,
        value: {
          cap: savedCap,
          lifetimeSpendByCurrencyMicros: { USD: 250_000 },
        },
      });
    if (command === "save_ai_cap") {
      savedCap = {
        ...args.request,
        currency: "USD",
        countedMicros: 250_000,
        reservedMicros: 0,
        unresolvedMicros: 0,
        revision: 1,
      };
      return Promise.resolve({ ok: true, value: savedCap });
    }
    return previous(command, args);
  });
  await clickAccessible("Edit Anthropic key spending limit");
  expect(native.invoke).toHaveBeenCalledWith("load_ai_key_settings", {
    credentialId: "second-id",
  });
  const input = document.querySelector<HTMLInputElement>(
    'input[inputmode="decimal"]',
  )!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      dom.window.HTMLInputElement.prototype,
      "value",
    )!.set!.call(input, "1.25");
    input.dispatchEvent(new dom.window.InputEvent("input", { bubbles: true }));
  });
  await act(async () => {
    input.blur();
  });
  expect(native.invoke).toHaveBeenCalledWith("save_ai_cap", {
    request: {
      credentialId: "second-id",
      period: "all_time",
      limitMicros: 1_250_000,
      timeZone: expect.any(String),
      expectedRevision: null,
    },
  });
  expect(document.body.textContent).toContain("$0.25/$1.25");
  expect(document.body.textContent).toContain("20%");
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "OpenAI key",
  );
  expect(document.body.textContent).not.toContain("Calendar period");
});

it("shows selected-period buckets without the redundant activity breakdown", async () => {
  native.invoke.mockImplementation(
    (command: string, args?: Record<string, unknown>) => {
      if (command === "load_ai_catalog") return Promise.resolve(catalog);
      if (command === "load_ai_monitoring") {
        expect(args).toEqual(
          expect.objectContaining({
            bucketSize: "day",
            timeZone: expect.any(String),
          }),
        );
        return Promise.resolve({
          ok: true,
          value: {
            ...emptyMonitoring.value,
            logicalOperations: 1,
            attempts: 1,
            usage: { inputTokens: 8, outputTokens: 2 },
            costByCurrencyMicros: { USD: 10_000 },
            byProvider: { openai: 1 },
            byStatus: { succeeded: 1 },
            byModel: { "fixture-model": 1 },
            byPreset: { "balanced@direct-v1": 1 },
            byOperationType: { credential_test: 1 },
            timeBuckets: [
              {
                label: "2026-09-15",
                attempts: 1,
                usage: { inputTokens: 8, outputTokens: 2 },
                costByCurrencyMicros: { USD: 10_000 },
                partial: false,
                unknownCount: 0,
              },
            ],
          },
        });
      }
      if (command === "load_ai_connection") return Promise.resolve(connection);
      if (command === "load_ai_caps")
        return Promise.resolve({ ok: true, value: [] });
      if (command === "load_ai_retention")
        return Promise.resolve({
          ok: true,
          value: { policy: "retain_until_cleared", removedOperations: 0 },
        });
      return Promise.reject(new Error(`unexpected ${command}`));
    },
  );
  await click("Week");
  await click("Tokens");
  const point = document.querySelector('[aria-label="2026-09-15: 10 tokens"]')!;
  await act(async () => {
    point.dispatchEvent(new dom.window.MouseEvent("click", { bubbles: true }));
  });
  const tooltip = document.querySelector('[role="tooltip"]')!;
  expect(tooltip.textContent).toContain("15 Tue");
  expect(tooltip.textContent).toContain("10Estimated total tokens");
  expect(tooltip.textContent).toContain("1 attempt");
  expect(tooltip.closest(".ai-chart__plot")).not.toBeNull();
  expect(tooltip.querySelector("dl")?.textContent).toContain(
    "Input8Output2Cached input0Cache write0Reasoning0",
  );
  expect(document.querySelector(".ai-chart__readout")).toBeNull();
  expect(document.body.textContent).not.toContain("Activity breakdown");
});

it("places metric and timeframe controls on the chart and settings below it", async () => {
  await click("Data");
  const chart = document.querySelector(".ai-chart")!;
  expect(chart.querySelector('[aria-label="Y axis metric"]')?.textContent).toBe(
    "PriceTokens",
  );
  expect(
    chart.querySelector('[aria-label="Monitoring period"]'),
  ).not.toBeNull();
  expect(document.querySelector(".ai-chart-toolbar")).toBeNull();
  expect(document.querySelector(".ai-usage-summary")).toBeNull();
  const settings = document.querySelector(".ai-data-settings")!;
  expect(
    chart.compareDocumentPosition(settings) & Node.DOCUMENT_POSITION_FOLLOWING,
  ).toBeTruthy();
  expect(chart.closest(".ai-panel")).not.toBe(settings);
  expect(settings.textContent).toContain("Settings");
  expect(settings.textContent).toContain("Export activity");
  expect(settings.textContent).toContain("Clear activity");
  expect(settings.textContent).toContain("Activity retention");
  expect(document.querySelector(".ai-data-heading")?.textContent).toContain(
    "All keysGeneral activity · Every provider and model",
  );
  await clickAccessible("Choose view");
  const picker = document.querySelector('[aria-label="Choose activity view"]')!;
  expect(picker.textContent).toContain("OpenAI");
  expect(picker.textContent).toContain("fixture-model");
  const allKeys = picker.querySelector(
    '[aria-label="View activity for all keys"]',
  )!;
  expect(allKeys.querySelector(".ai-data-key-option-logo")).toBeNull();
  const openAiKey = picker.querySelector(
    '[aria-label="View activity for OpenAI key"]',
  )!;
  expect(
    openAiKey.querySelector(".ai-data-key-option-logo img"),
  ).not.toBeNull();
  expect(
    openAiKey.querySelector(".ai-data-key-option-meta")?.textContent,
  ).toContain("2026");
  expect(picker.querySelector(".ai-data-key-option-icon")).toBeNull();
  const results = await axe.run(picker, {
    rules: { "color-contrast": { enabled: false } },
  });
  expect(results.violations).toEqual([]);
  await act(async () => {
    document.body.dispatchEvent(
      new dom.window.Event("pointerdown", { bubbles: true }),
    );
  });
  expect(
    document.querySelector('[aria-label="Choose activity view"]'),
  ).toBeNull();
});

it("applies the disclosed retain-until-cleared policy separately from caps", async () => {
  native.invoke.mockImplementation((command: string) => {
    if (command === "load_ai_catalog") return Promise.resolve(catalog);
    if (command === "save_ai_retention")
      return Promise.resolve({
        ok: true,
        value: { policy: "retain_until_cleared", removedOperations: 0 },
      });
    if (command === "load_ai_monitoring")
      return Promise.resolve(emptyMonitoring);
    if (command === "load_ai_connection") return Promise.resolve(connection);
    if (command === "load_ai_caps")
      return Promise.resolve({ ok: true, value: [] });
    if (command === "load_ai_retention")
      return Promise.resolve({
        ok: true,
        value: { policy: "retain_until_cleared", removedOperations: 0 },
      });
    return Promise.reject(new Error(`unexpected ${command}`));
  });
  await click("Apply retention");
  expect(native.invoke).toHaveBeenCalledWith("save_ai_retention", {
    policy: "retain_until_cleared",
  });
  expect(document.body.textContent).toContain("Spending caps were not reset");
});

it("confirms retention policies before permanently deleting older activity", async () => {
  const previous = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command: string, args?: unknown) => {
    if (command === "save_ai_retention")
      return Promise.resolve({
        ok: true,
        value: { policy: "30_days", removedOperations: 4 },
      });
    return previous(command, args);
  });
  await choose("Retention policy", "30_days");
  await click("Apply retention");
  expect(
    native.invoke.mock.calls.some(
      ([command]) => command === "save_ai_retention",
    ),
  ).toBe(false);
  const dialog = document.querySelector(
    '[aria-labelledby="ai-retention-confirm-title"]',
  )!;
  expect(dialog.textContent).toContain(
    "Activity older than 30 days will be permanently deleted",
  );
  expect(dialog.textContent).toContain(
    "Lifetime spend and spending-cap progress on My Keys will not change",
  );
  await click("Permanently delete older activity");
  expect(native.invoke).toHaveBeenCalledWith("save_ai_retention", {
    policy: "30_days",
  });
  expect(document.body.textContent).toContain(
    "Retention saved; 4 older operations were cleared",
  );
});

it("labels the AI and Monitoring controls for accessibility", async () => {
  const result = await axe.run(document.getElementById("root")!, {
    rules: { "color-contrast": { enabled: false } },
  });
  expect(result.violations.map((item) => item.id)).toEqual([]);
  await click("Data");
  const dataResult = await axe.run(document.getElementById("root")!, {
    rules: { "color-contrast": { enabled: false } },
  });
  expect(dataResult.violations.map((item) => item.id)).toEqual([]);
});

it("separates General and Data using the resume navigation pattern", async () => {
  const general = document.querySelector<HTMLElement>(
    '[aria-label="My Keys settings"]',
  )!;
  const data = document.querySelector<HTMLElement>(
    '[aria-label="AI usage data"]',
  )!;
  expect(general.hidden).toBe(false);
  expect(data.hidden).toBe(true);
  expect(
    document.querySelector(
      '[aria-label="AI section"] .button--secondary[aria-current]',
    )?.textContent,
  ).toBe("My Keys");
  await click("Data");
  expect(general.hidden).toBe(true);
  expect(data.hidden).toBe(false);
  await click("My Keys");
  expect(general.hidden).toBe(false);
  expect(document.body.textContent).not.toContain("Connect your provider.");
  expect(document.querySelector(".ai-key-status")).toBeNull();
  expect(document.querySelector(".ai-key-primary")).toBeNull();
  expect(document.querySelector('[aria-label="Pause OpenAI key"]')).toBeNull();
  expect(
    document.querySelector('[aria-label="Options for OpenAI key"]'),
  ).not.toBeNull();
});

it("saves names immediately while typing and exits on click-away", async () => {
  const previous = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command: string, args?: any) =>
    command === "rename_ai_key"
      ? Promise.resolve({
          ok: true,
          value: {
            ...connection.value,
            keys: connection.value.keys.map((key) =>
              key.credentialId === args.request.credentialId
                ? { ...key, name: args.request.name }
                : key,
            ),
          },
        })
      : previous(command, args),
  );
  await clickAccessible("Rename OpenAI key");
  const input = document.querySelector<HTMLInputElement>(
    '[aria-label="Name for OpenAI key"]',
  )!;
  await typeInput(input, "Personal");
  expect(native.invoke).toHaveBeenCalledWith("rename_ai_key", {
    request: { credentialId: "fixture-id", name: "Personal" },
  });
  expect(document.querySelector('[aria-label="Save key name"]')).toBeNull();
  expect(document.querySelector('[aria-label="Cancel rename"]')).toBeNull();
  expect(input.value).toBe("Personal");
  await act(async () => {
    input.blur();
  });
  expect(
    document.querySelector('[aria-label="Name for OpenAI key"]'),
  ).toBeNull();
  expect(document.querySelector(".ai-key-name")?.textContent).toBe("Personal");
  await clickAccessible("Choose view");
  expect(
    document.querySelector('[aria-label="View activity for Personal"]')
      ?.textContent,
  ).toContain("Personal");
});

it("serializes rapid name saves without replacing newer draft text", async () => {
  const previous = native.invoke.getMockImplementation()!;
  const requests: Array<{ name: string; resolve: (value: unknown) => void }> =
    [];
  native.invoke.mockImplementation((command: string, args?: any) =>
    command === "rename_ai_key"
      ? new Promise((resolve) =>
          requests.push({ name: args.request.name, resolve }),
        )
      : previous(command, args),
  );
  await clickAccessible("Rename OpenAI key");
  const input = document.querySelector<HTMLInputElement>(
    '[aria-label="Name for OpenAI key"]',
  )!;
  await typeInput(input, "P");
  await typeInput(input, "Pe");
  await typeInput(input, "Personal");
  expect(requests).toHaveLength(1);
  const reply = (name: string) => ({
    ok: true,
    value: {
      ...connection.value,
      keys: connection.value.keys.map((key) =>
        key.credentialId === "fixture-id" ? { ...key, name } : key,
      ),
    },
  });
  await act(async () => {
    requests[0].resolve(reply("P"));
  });
  expect(input.value).toBe("Personal");
  expect(requests).toHaveLength(2);
  expect(requests[1].name).toBe("Personal");
  await act(async () => {
    input.blur();
  });
  await act(async () => {
    requests[1].resolve(reply("Personal"));
  });
  expect(requests).toHaveLength(2);
  expect(document.querySelector(".ai-key-name")?.textContent).toBe("Personal");
});

it("reports failed name saves without pretending the name was saved", async () => {
  const previous = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command: string, args?: unknown) =>
    command === "rename_ai_key"
      ? Promise.resolve({ ok: false, error: { code: "STORAGE_UNAVAILABLE" } })
      : previous(command, args),
  );
  await clickAccessible("Rename OpenAI key");
  const input = document.querySelector<HTMLInputElement>(
    '[aria-label="Name for OpenAI key"]',
  )!;
  await typeInput(input, "Unsaved");
  await act(async () => {
    input.blur();
  });
  expect(document.body.textContent).toContain("Name not saved");
  expect(document.querySelector(".ai-key-name")?.textContent).toBe(
    "OpenAI key",
  );
});

it("shows all key information and controls without expansion", async () => {
  expect(document.querySelectorAll(".ai-key-card-layout")).toHaveLength(2);
  const firstCard = document.querySelector<HTMLElement>(".ai-key-row")!;
  expect(firstCard.querySelector(".ai-key-drag-handle")).toBeNull();
  expect(firstCard.tabIndex).toBe(0);
  expect(firstCard.querySelector(".ai-key-provider-logo img")).not.toBeNull();
  expect(
    firstCard.querySelector(".ai-key-identity-line")?.textContent,
  ).toContain("OpenAI·OpenAI key");
  expect(firstCard.textContent).not.toContain("Sep 1, 2026");
  expect(document.body.textContent).toContain("Balanced: fixture-model");
  expect(document.body.textContent).not.toContain("ORT-tested input limit");
  expect(document.body.textContent).not.toContain("Pricing & model details");
  expect(document.querySelector(".ai-key-customization")).toBeNull();
  expect(
    document.querySelector('[aria-label="Customize OpenAI key"]'),
  ).toBeNull();
  expect(
    document.querySelector(".ai-key-card-layout")?.firstElementChild?.className,
  ).toBe("ai-key-card-info");
  expect(
    document.querySelector<HTMLProgressElement>(
      '[aria-label="OpenAI key spending cap used"]',
    )!.value,
  ).toBe(0);
  expect(document.querySelector(".ai-key-card-budget")?.textContent).toContain(
    "$0.25/Unlimited0%",
  );
});

it("manages one general cap across all keys while keeping lifetime spend", async () => {
  let cap: any = null;
  const previous = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command: string, args?: any) => {
    if (command === "load_ai_general_settings")
      return Promise.resolve({
        ok: true,
        value: { cap, lifetimeSpendByCurrencyMicros: { USD: 2_000_000 } },
      });
    if (command === "save_ai_general_cap") {
      cap = {
        credentialId: "general",
        period: "all_time",
        currency: "USD",
        timeZone: "UTC",
        limitMicros: args.limitMicros,
        countedMicros: 2_000_000,
        reservedMicros: 0,
        unresolvedMicros: 0,
        revision: 1,
      };
      return Promise.resolve({ ok: true, value: cap });
    }
    if (command === "reset_ai_general_cap") {
      cap = { ...cap, countedMicros: 0 };
      return Promise.resolve({ ok: true, value: true });
    }
    if (command === "disable_ai_general_cap") {
      cap = null;
      return Promise.resolve({ ok: true, value: true });
    }
    return previous(command, args);
  });
  await act(async () => {
    root.render(<AiWorkspace key="general-cap" blocked={false} />);
  });
  expect(document.querySelector(".ai-general-spending")?.textContent).toContain(
    "$2.00/Unlimited0%",
  );
  await act(async () => {
    document
      .querySelector<HTMLButtonElement>(
        '[aria-label="Edit general spending limit"]',
      )!
      .click();
  });
  const input = document.querySelector<HTMLInputElement>(
    '[aria-label="General spending limit"]',
  )!;
  await typeInput(input, "5");
  await act(async () => {
    input.blur();
  });
  expect(native.invoke).toHaveBeenCalledWith(
    "save_ai_general_cap",
    expect.objectContaining({ limitMicros: 5_000_000, expectedRevision: null }),
  );
  expect(document.querySelector(".ai-general-spending")?.textContent).toContain(
    "$2.00/$5.00",
  );
  const general = document.querySelector(".ai-general-spending")!;
  await act(async () => {
    general
      .querySelectorAll<HTMLButtonElement>(".ai-key-cap-actions button")[0]
      .click();
  });
  await click("Confirm restart");
  expect(native.invoke).toHaveBeenCalledWith("reset_ai_general_cap");
  expect(general.textContent).toContain("$0.00/$5.00");
  expect(general.textContent).toContain("$2.00Total spent across all keys");
  await act(async () => {
    general
      .querySelectorAll<HTMLButtonElement>(".ai-key-cap-actions button")[1]
      .click();
  });
  expect(native.invoke).toHaveBeenCalledWith("disable_ai_general_cap");
  expect(general.textContent).toContain("$2.00/Unlimited0%");
});

it("fails closed when spending data cannot be loaded", async () => {
  const previous = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command: string, args?: unknown) =>
    command === "load_ai_key_settings"
      ? Promise.resolve({ ok: false, error: { code: "STORAGE_UNAVAILABLE" } })
      : previous(command, args),
  );
  await act(async () => {
    root.render(<AiWorkspace key="failed-settings" blocked={false} />);
  });
  expect(document.body.textContent).toContain("Spending data unavailable");
  expect(
    [...document.querySelectorAll("select")].find((select) =>
      select.closest("label")?.textContent?.includes("Model preset"),
    )?.disabled,
  ).toBe(true);
  expect(
    document.querySelector('[aria-label="Edit OpenAI key spending limit"]'),
  ).toBeNull();
});

async function typeInput(input: HTMLInputElement, value: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      dom.window.HTMLInputElement.prototype,
      "value",
    )!.set!.call(input, value);
    input.dispatchEvent(new dom.window.InputEvent("input", { bubbles: true }));
  });
}
async function clickAccessible(label: string) {
  if (
    /^(Test|Pause|Unpause|Remove) .+$/.test(label) &&
    !document.querySelector(`button[aria-label="${label}"]`)
  ) {
    await clickAccessible(
      `Options for ${label.replace(/^(Test|Pause|Unpause|Remove) /, "")}`,
    );
  }
  const button = document.querySelector<HTMLButtonElement>(
    `button[aria-label="${label}"]`,
  )!;
  expect(button).not.toBeNull();
  expect(button.disabled).toBe(false);
  await act(async () => {
    button.dispatchEvent(new dom.window.MouseEvent("click", { bubbles: true }));
  });
}
async function chooseDataView(label: string) {
  await clickAccessible("Choose view");
  await clickAccessible(`View activity for ${label}`);
}
async function choose(label: string, value: string) {
  const select = [...document.querySelectorAll("select")].find((element) =>
    element.closest("label")?.textContent?.trim().startsWith(label),
  )!;
  await act(async () => {
    select.value = value;
    select.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
}
function returnRegistry(value: typeof connection.value) {
  const previous = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command: string, args?: unknown) => {
    if (command === "load_ai_connection")
      return Promise.resolve({ ok: true, value });
    if (command === "change_ai_key")
      return Promise.resolve({ ok: true, value });
    if (command === "clear_ai_primary")
      return Promise.resolve({ ok: true, value });
    return previous(command, args);
  });
}
it("moves a key into the Active key bucket and returns the previous key to All keys", async () => {
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "OpenAI key",
  );
  returnRegistry({ ...connection.value, primaryCredentialId: "second-id" });
  await act(async () => {
    document
      .querySelector<HTMLElement>('[aria-label="Anthropic key · Anthropic"]')!
      .dispatchEvent(
        new dom.window.KeyboardEvent("keydown", {
          key: "Enter",
          bubbles: true,
        }),
      );
  });
  expect(native.invoke).toHaveBeenCalledWith("change_ai_key", {
    request: { credentialId: "second-id", action: "select_primary" },
  });
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "Anthropic key",
  );
  expect(
    document.querySelector(".ai-key-bucket--available")?.textContent,
  ).toContain("OpenAI key");
  expect(document.querySelectorAll(".ai-key-row--active")).toHaveLength(1);
});

it("moves the active key back to All keys and leaves no active key", async () => {
  returnRegistry({ ...connection.value, primaryCredentialId: null });
  await act(async () => {
    document
      .querySelector<HTMLElement>('[aria-label="OpenAI key · OpenAI"]')!
      .dispatchEvent(
        new dom.window.KeyboardEvent("keydown", {
          key: "Enter",
          bubbles: true,
        }),
      );
  });
  expect(native.invoke).toHaveBeenCalledWith("clear_ai_primary");
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "No active key selected",
  );
  expect(
    document.querySelector(".ai-key-bucket--available")?.textContent,
  ).toContain("OpenAI key");
});

it("uses a vertically bounded drag to replace the active key", async () => {
  returnRegistry({ ...connection.value, primaryCredentialId: "second-id" });
  const buckets = document.querySelector<HTMLElement>(".ai-key-buckets")!;
  const active = document.querySelector<HTMLElement>(".ai-key-bucket--active")!;
  const available = document.querySelector<HTMLElement>(
    ".ai-key-bucket--available",
  )!;
  const card = document.querySelector<HTMLElement>(
    '.ai-key-bucket--available [aria-label="Anthropic key · Anthropic"]',
  )!;
  buckets.getBoundingClientRect = () =>
    ({ left: 20, right: 820, top: 20, bottom: 520 }) as DOMRect;
  active.getBoundingClientRect = () =>
    ({ left: 20, right: 820, top: 20, bottom: 180 }) as DOMRect;
  available.getBoundingClientRect = () =>
    ({ left: 20, right: 820, top: 194, bottom: 520 }) as DOMRect;
  card.getBoundingClientRect = () =>
    ({
      left: 32,
      right: 808,
      top: 240,
      bottom: 340,
      width: 776,
      height: 100,
    }) as DOMRect;
  const pointer = (type: string, clientY: number) => {
    const event = new dom.window.MouseEvent(type, {
      bubbles: true,
      button: 0,
      clientX: 400,
      clientY,
    });
    Object.defineProperty(event, "pointerId", { value: 7 });
    return event;
  };
  await act(async () => {
    card.dispatchEvent(pointer("pointerdown", 260));
    dom.window.dispatchEvent(pointer("pointermove", 90));
  });
  const ghost = document.querySelector<HTMLElement>(".ai-key-drag-ghost")!;
  expect(ghost).not.toBeNull();
  expect(ghost.style.left).toBe("32px");
  expect(document.querySelector(".ai-key-bucket--drop-target")).toBe(active);
  await act(async () => {
    dom.window.dispatchEvent(pointer("pointerup", 90));
  });
  expect(native.invoke).toHaveBeenCalledWith("change_ai_key", {
    request: { credentialId: "second-id", action: "select_primary" },
  });
});

it("keeps All keys sorted by creation date", async () => {
  const previous = native.invoke.getMockImplementation()!;
  const unsorted = {
    ...connection.value,
    primaryCredentialId: null,
    keys: [...connection.value.keys].reverse(),
  };
  native.invoke.mockImplementation((command: string, args?: unknown) =>
    command === "load_ai_connection"
      ? Promise.resolve({ ok: true, value: unsorted })
      : previous(command, args),
  );
  await act(async () => {
    root.render(<AiWorkspace key="sorted-keys" blocked={false} />);
  });
  const ids = [
    ...document.querySelectorAll(".ai-key-bucket--available .ai-key-row"),
  ].map((row) => row.getAttribute("data-key-id"));
  expect(ids).toEqual(["fixture-id", "second-id"]);
});

it("removes the limit back to Unlimited with an empty meter without clearing history", async () => {
  const previous = native.invoke.getMockImplementation()!;
  let cap: Record<string, unknown> | null = {
    credentialId: "second-id",
    period: "all_time",
    currency: "USD",
    timeZone: "UTC",
    limitMicros: 1_000_000,
    countedMicros: 200_000,
    reservedMicros: 0,
    unresolvedMicros: 0,
    revision: 1,
  };
  native.invoke.mockImplementation((command: string, args?: any) => {
    if (
      command === "load_ai_key_settings" &&
      args?.credentialId === "second-id"
    )
      return Promise.resolve({
        ok: true,
        value: { cap, lifetimeSpendByCurrencyMicros: { USD: 250_000 } },
      });
    if (command === "disable_ai_cap") {
      cap = null;
      return Promise.resolve({ ok: true, value: true });
    }
    return previous(command, args);
  });
  await act(async () => {
    root.render(<AiWorkspace key="remove-limit" blocked={false} />);
  });
  const card = document.querySelectorAll(".ai-key-row")[1];
  expect(card.textContent).toContain("$0.20/$1.00");
  await act(async () => {
    [...card.querySelectorAll("button")]
      .find((button) => button.textContent === "Remove limit")!
      .click();
  });
  expect(native.invoke).toHaveBeenCalledWith("disable_ai_cap", {
    request: { credentialId: "second-id", period: "all_time" },
  });
  expect(card.textContent).toContain("$0.25/Unlimited0%");
  expect(card.querySelector<HTMLProgressElement>("progress")?.value).toBe(0);
  expect(card.querySelector(".ai-key-spend")?.textContent).toContain("$0.25");
  expect(
    native.invoke.mock.calls.some(
      ([command]) =>
        command === "clear_ai_monitoring" || command === "reset_ai_cap",
    ),
  ).toBe(false);
});

it("rejects invalid inline limits without writing or changing the meter", async () => {
  await clickAccessible("Edit OpenAI key spending limit");
  const input = document.querySelector<HTMLInputElement>(
    '[aria-label="OpenAI key spending limit"]',
  )!;
  await typeInput(input, "1.0000001");
  await act(async () => {
    input.blur();
  });
  expect(document.body.textContent).toContain(
    "Enter a positive limit with up to six decimal places",
  );
  expect(
    native.invoke.mock.calls.some(([command]) => command === "save_ai_cap"),
  ).toBe(false);
  expect(
    document.querySelector<HTMLProgressElement>(
      '[aria-label="OpenAI key spending cap used"]',
    )?.value,
  ).toBe(0);
});

it("pausing the active key leaves no active key; unpausing keeps it in All keys", async () => {
  returnRegistry({
    ...connection.value,
    primaryCredentialId: null,
    keys: connection.value.keys.map((key) =>
      key.credentialId === "fixture-id" ? { ...key, paused: true } : key,
    ),
  });
  await clickAccessible("Pause OpenAI key");
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "No active key selected",
  );
  expect(
    document.querySelector<HTMLElement>('[aria-label="OpenAI key · OpenAI"]')
      ?.tabIndex,
  ).toBe(-1);
  expect(document.querySelector('[aria-label="Rename OpenAI key"]')).toBeNull();
  expect(
    document.querySelector<HTMLSelectElement>(
      '[aria-label="OpenAI key settings"] select',
    )?.disabled,
  ).toBe(true);
  await clickAccessible("Options for OpenAI key");
  expect(
    document.querySelector<HTMLButtonElement>('[aria-label="Test OpenAI key"]')
      ?.disabled,
  ).toBe(false);
  await act(async () => {
    document.body.dispatchEvent(
      new dom.window.Event("pointerdown", { bubbles: true }),
    );
  });
  returnRegistry({ ...connection.value, primaryCredentialId: null });
  await clickAccessible("Unpause OpenAI key");
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "No active key selected",
  );
  expect(
    document.querySelector<HTMLElement>('[aria-label="OpenAI key · OpenAI"]')
      ?.tabIndex,
  ).toBe(0);
});

it("removing the active key preserves its activity filter and never selects another key", async () => {
  returnRegistry({
    ...connection.value,
    primaryCredentialId: null,
    keys: connection.value.keys.map((key) =>
      key.credentialId === "fixture-id"
        ? { ...key, paused: true, removed: true }
        : key,
    ),
  });
  await clickAccessible("Remove OpenAI key");
  await click("Remove key");
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "No active key selected",
  );
  expect(document.querySelectorAll(".ai-key-row")).toHaveLength(1);
  await clickAccessible("Choose view");
  expect(
    document.querySelector('[aria-label="View activity for Archived key"]')
      ?.textContent,
  ).toContain("OpenAI · Balanced: fixture-model · Removed");
  expect(native.invoke).toHaveBeenCalledWith("change_ai_key", {
    request: { credentialId: "fixture-id", action: "remove" },
  });
});

it("requires both key and provider when adding; the provider is immutable afterward", async () => {
  await clickAccessible("Add key");
  const form = document.querySelector('[aria-label="Add new API key"]')!;
  const password = form.querySelector<HTMLInputElement>(
    'input[type="password"]',
  )!;
  const name = form.querySelector<HTMLInputElement>('input[name="keyName"]')!;
  const providers = form.querySelector(".ai-provider-picker")!;
  expect(
    name.compareDocumentPosition(providers) &
      dom.window.Node.DOCUMENT_POSITION_FOLLOWING,
  ).toBeTruthy();
  expect(name.placeholder).toBe("Enter a name");
  expect(name.getAttribute("aria-label")).toBe("Key name");
  expect(
    form.querySelectorAll('.ai-provider-option[aria-pressed="true"]'),
  ).toHaveLength(0);
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      dom.window.HTMLInputElement.prototype,
      "value",
    )!.set!.call(password, "test-only-key");
    password.dispatchEvent(
      new dom.window.InputEvent("input", { bubbles: true }),
    );
  });
  expect(
    [...form.querySelectorAll("button")].find(
      (button) => button.textContent === "Save key",
    )?.disabled,
  ).toBe(true);
  await clickAccessible("Show API key");
  expect(password.type).toBe("text");
  await clickAccessible("Hide API key");
  expect(password.type).toBe("password");
  await clickAccessible("Select Gemini");
  expect(name.placeholder).toBe("Gemini key");
  expect(
    form
      .querySelector('[aria-label="Select Gemini"]')
      ?.getAttribute("aria-pressed"),
  ).toBe("true");
  expect(
    form
      .querySelector('[aria-label="Select OpenAI"]')
      ?.getAttribute("aria-pressed"),
  ).toBe("false");
  await clickAccessible("Select Gemini");
  expect(
    form
      .querySelector('[aria-label="Select Gemini"]')
      ?.getAttribute("aria-pressed"),
  ).toBe("false");
  expect(
    [...form.querySelectorAll("button")].find(
      (button) => button.textContent === "Save key",
    )?.disabled,
  ).toBe(true);
  await clickAccessible("Select Gemini");
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      dom.window.HTMLInputElement.prototype,
      "value",
    )!.set!.call(name, "Research key");
    name.dispatchEvent(new dom.window.InputEvent("input", { bubbles: true }));
  });
  const previous = native.invoke.getMockImplementation()!;
  const next = {
    ...connection.value,
    keys: [
      ...connection.value.keys,
      {
        ...connection.value.keys[0],
        credentialId: "third-id",
        createdAt: "2026-09-03T12:00:00Z",
        provider: "gemini",
        name: "Research key",
      },
    ],
  };
  native.invoke.mockImplementation((command: string, args?: unknown) =>
    command === "add_ai_key"
      ? Promise.resolve({ ok: true, value: next })
      : previous(command, args),
  );
  await click("Save key");
  expect(native.invoke).toHaveBeenCalledWith("add_ai_key", {
    request: {
      provider: "gemini",
      apiKey: "test-only-key",
      name: "Research key",
    },
  });
  expect(document.querySelector('[aria-label="Add new API key"]')).toBeNull();
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "OpenAI key",
  );
  expect(document.querySelectorAll(".ai-key-row")).toHaveLength(3);
  expect(
    document.querySelector(
      '[aria-label="Research key · Gemini"] option[value="openai"]',
    ),
  ).toBeNull();
  expect(document.body.textContent).not.toContain("test-only-key");
});

it("discards an unfinished add-key popup when clicking away", async () => {
  await clickAccessible("Add key");
  await clickAccessible("Select OpenAI");
  const form = document.querySelector('[aria-label="Add new API key"]')!;
  const input = form.querySelector<HTMLInputElement>("input")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      dom.window.HTMLInputElement.prototype,
      "value",
    )!.set!.call(input, "discard-me");
    input.dispatchEvent(new dom.window.InputEvent("input", { bubbles: true }));
  });
  await act(async () => {
    form.parentElement!.dispatchEvent(
      new dom.window.Event("pointerdown", { bubbles: true }),
    );
  });
  expect(document.querySelector('[aria-label="Add new API key"]')).toBeNull();

  await clickAccessible("Add key");
  const reopened = document.querySelector('[aria-label="Add new API key"]')!;
  expect(
    reopened.querySelector<HTMLInputElement>('input[name="keyName"]')?.value,
  ).toBe("");
  expect(
    reopened.querySelector<HTMLInputElement>('input[type="password"]')?.value,
  ).toBe("");
  expect(
    reopened.querySelectorAll('.ai-provider-option[aria-pressed="true"]'),
  ).toHaveLength(0);
});

it("testing a non-active key preserves the active key", async () => {
  const previous = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command: string, args?: unknown) =>
    command === "preview_ai_test"
      ? Promise.resolve({
          ...preview,
          value: {
            ...preview.value,
            credentialId: "second-id",
            provider: "anthropic",
          },
        })
      : previous(command, args),
  );
  await clickAccessible("Test Anthropic key");
  expect(native.invoke).toHaveBeenCalledWith("preview_ai_test", {
    credentialId: "second-id",
  });
  await click("Confirm and send test");
  expect(native.invoke).toHaveBeenCalledWith(
    "test_ai_connection",
    expect.objectContaining({ credentialId: "second-id" }),
  );
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "OpenAI key",
  );
  expect(
    native.invoke.mock.calls.some(([command]) => command === "change_ai_key"),
  ).toBe(false);
});

it("keeps failed removals visible and requires an explicit retry", async () => {
  const previous = native.invoke.getMockImplementation()!;
  let removalAttempts = 0;
  native.invoke.mockImplementation((command: string, args?: unknown) => {
    if (command === "change_ai_key") {
      removalAttempts += 1;
      return removalAttempts === 1
        ? Promise.resolve({
            ok: false,
            error: { code: "AI_CREDENTIAL_CLEANUP_REQUIRED" },
          })
        : Promise.resolve({
            ok: true,
            value: {
              ...connection.value,
              primaryCredentialId: null,
              keys: connection.value.keys.map((key) =>
                key.credentialId === "fixture-id"
                  ? {
                      ...key,
                      paused: true,
                      removed: true,
                      cleanupRequired: false,
                    }
                  : key,
              ),
            },
          });
    }
    if (command === "load_ai_connection")
      return Promise.resolve({
        ok: true,
        value: {
          ...connection.value,
          primaryCredentialId: null,
          keys: connection.value.keys.map((key) =>
            key.credentialId === "fixture-id"
              ? { ...key, paused: true, cleanupRequired: true }
              : key,
          ),
        },
      });
    return previous(command, args);
  });
  await clickAccessible("Remove OpenAI key");
  await click("Remove key");
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "No active key selected",
  );
  expect(document.body.textContent).toContain("Removal failed");
  expect(
    document.querySelector('[aria-label="OpenAI key · OpenAI"]')?.textContent,
  ).toContain("Removal failed");
  expect(document.querySelectorAll(".ai-key-row")).toHaveLength(2);
  await clickAccessible("Options for OpenAI key");
  await click("Retry removal");
  expect(document.body.textContent).toContain("Retry removing OpenAI key?");
  await click("Retry removal");
  expect(removalAttempts).toBe(2);
  expect(
    document.querySelector('[aria-label="OpenAI key · OpenAI"]'),
  ).toBeNull();
});

it("dismisses the key menu on outside click and Escape, with keyboard navigation", async () => {
  await clickAccessible("Options for OpenAI key");
  const menu = document.querySelector('[role="menu"]')!;
  expect(menu.textContent).toBe("Test keyPauseRemove");
  expect(document.activeElement?.getAttribute("aria-label")).toBe(
    "Test OpenAI key",
  );
  await act(async () => {
    document.activeElement?.dispatchEvent(
      new dom.window.KeyboardEvent("keydown", {
        key: "ArrowDown",
        bubbles: true,
      }),
    );
  });
  expect(document.activeElement?.getAttribute("aria-label")).toBe(
    "Pause OpenAI key",
  );
  const results = await axe.run(menu, {
    rules: { "color-contrast": { enabled: false } },
  });
  expect(results.violations).toEqual([]);
  await act(async () => {
    document.activeElement?.dispatchEvent(
      new dom.window.KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
    );
  });
  expect(document.querySelector('[role="menu"]')).toBeNull();
  expect(document.activeElement?.getAttribute("aria-label")).toBe(
    "Options for OpenAI key",
  );
  await clickAccessible("Options for OpenAI key");
  await act(async () => {
    document.body.dispatchEvent(
      new dom.window.Event("pointerdown", { bubbles: true }),
    );
  });
  expect(document.querySelector('[role="menu"]')).toBeNull();
});

it("locks paused card settings but permits an explicit test without enabling the key", async () => {
  returnRegistry({
    ...connection.value,
    primaryCredentialId: null,
    keys: connection.value.keys.map((key) =>
      key.credentialId === "fixture-id" ? { ...key, paused: true } : key,
    ),
  });
  await clickAccessible("Pause OpenAI key");
  const card = document.querySelector(".ai-key-row--paused")!;
  expect(card.querySelector('[aria-label="Rename OpenAI key"]')).toBeNull();
  expect(card.querySelector<HTMLSelectElement>("select")?.disabled).toBe(true);
  expect(
    card.querySelector<HTMLButtonElement>(
      '[aria-label="Edit OpenAI key spending limit"]',
    )?.disabled,
  ).toBe(true);
  await clickAccessible("Test OpenAI key");
  expect(
    document.querySelector('[aria-label="Confirm synthetic provider request"]'),
  ).not.toBeNull();
  expect(native.invoke).toHaveBeenCalledWith("preview_ai_test", {
    credentialId: "fixture-id",
  });
  expect(
    native.invoke.mock.calls.filter(([command]) => command === "change_ai_key"),
  ).toHaveLength(1);
});

it("blocks key use if a failed mutation cannot refresh backend state", async () => {
  const previous = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command: string, args?: unknown) => {
    if (command === "change_ai_key" || command === "load_ai_connection")
      return Promise.resolve({
        ok: false,
        error: { code: "STORAGE_UNAVAILABLE" },
      });
    return previous(command, args);
  });
  await clickAccessible("Pause OpenAI key");
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "Active key unavailable",
  );
  expect(document.querySelectorAll(".ai-key-row")).toHaveLength(0);
  expect(
    document.querySelector<HTMLButtonElement>('[aria-label="Add key"]')
      ?.disabled,
  ).toBe(true);
});

it("chooses export and clear months independently from the graph", async () => {
  const previous = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command: string, args?: unknown) => {
    if (command === "load_ai_monitoring") {
      const request = args as Record<string, unknown>;
      return Promise.resolve({
        ok: true,
        value: {
          ...emptyMonitoring.value,
          attempts: 1,
          timeBuckets:
            request.fromUnixMs === 0 && request.bucketSize === "month"
              ? [
                  { label: "2026-08", attempts: 3 },
                  { label: "2026-09", attempts: 5 },
                ]
              : [],
        },
      });
    }
    if (command === "clear_ai_monitoring")
      return Promise.resolve({ ok: true, value: 1 });
    if (command === "export_ai_monitoring")
      return Promise.resolve({ ok: true, value: "exported" });
    return previous(command, args);
  });
  await chooseDataView("Anthropic key");
  expect(native.invoke).toHaveBeenCalledWith(
    "load_ai_monitoring",
    expect.objectContaining({ credentialId: "second-id" }),
  );
  await click("Export JSON…");
  expect(
    document.querySelector('[aria-labelledby="ai-data-action-title"]')
      ?.textContent,
  ).toContain("Step 1 of 2Export activity");
  await clickAccessible("Select OpenAI key");
  await clickAccessible("Select Anthropic key");
  await clickAccessible("Deselect Anthropic key");
  await clickAccessible("Select Anthropic key");
  await click("Continue");
  expect(document.body.textContent).toContain(
    "2 keys · Only months with activity",
  );
  expect(document.body.textContent).toContain("Only months with activity");
  await clickAccessible("Select September 2026");
  await clickAccessible("Select August 2026");
  await click("Export 2 months");
  expect(native.invoke).toHaveBeenCalledWith(
    "export_ai_monitoring",
    expect.objectContaining({
      credentialIds: ["fixture-id", "second-id"],
      months: [
        expect.objectContaining({ label: "2026-09" }),
        expect.objectContaining({ label: "2026-08" }),
      ],
    }),
  );
  await click("Choose activity…");
  await clickAccessible("Select all keys");
  await clickAccessible("Select OpenAI key");
  await click("Continue");
  expect(
    document.querySelector('[aria-labelledby="ai-data-action-title"]')
      ?.textContent,
  ).toContain("All keys");
  await clickAccessible("Select September 2026");
  await click("Clear 1 month");
  expect(native.invoke).toHaveBeenCalledWith(
    "clear_ai_monitoring",
    expect.objectContaining({
      credentialIds: null,
      months: [expect.objectContaining({ label: "2026-09" })],
    }),
  );
  await chooseDataView("all keys");
  expect(native.invoke).toHaveBeenCalledWith(
    "load_ai_monitoring",
    expect.objectContaining({ credentialId: null }),
  );
});

it("permanently deletes data only for removed keys after explicit confirmation", async () => {
  const removedKey = {
    ...connection.value.keys[0],
    credentialId: "removed-id",
    createdAt: "2026-08-01T12:00:00Z",
    name: "Old Gemini",
    provider: "gemini" as const,
    paused: true,
    removed: true,
  };
  const withRemoved = {
    ...connection.value,
    keys: [...connection.value.keys, removedKey],
  };
  const withoutRemoved = {
    ...connection.value,
    keys: [...connection.value.keys],
  };
  const previous = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command: string, args?: unknown) => {
    if (command === "load_ai_connection")
      return Promise.resolve({ ok: true, value: withRemoved });
    if (command === "delete_removed_ai_key_data")
      return Promise.resolve({
        ok: true,
        value: { registry: withoutRemoved, clearedOperations: 4 },
      });
    return previous(command, args);
  });
  await act(async () => {
    root.render(<AiWorkspace key="removed-key-data" blocked={false} />);
  });

  await click("Data");
  await click("Choose removed keys…");
  expect(
    document.querySelector('[aria-label="Select removed key Old Gemini"]'),
  ).not.toBeNull();
  expect(
    document.querySelector('[aria-label="Select removed key OpenAI key"]'),
  ).toBeNull();
  await clickAccessible("Select removed key Old Gemini");
  await click("Continue");
  expect(document.body.textContent).toContain("This cannot be undone");
  expect(document.body.textContent).toContain(
    "removed from the All keys data display",
  );
  expect(document.body.textContent).toContain(
    "spending totals on My Keys will not change",
  );
  await click("Permanently delete data");
  expect(native.invoke).toHaveBeenCalledWith("delete_removed_ai_key_data", {
    request: { credentialIds: ["removed-id"] },
  });
  expect(document.body.textContent).toContain(
    "All keys data was updated; My Keys spending totals were not changed",
  );
  await clickAccessible("Choose view");
  expect(
    document.querySelector('[aria-label="View activity for Old Gemini"]'),
  ).toBeNull();
});

it("requires explicit scope choices and can cancel the activity workflow", async () => {
  await click("Choose activity…");
  const continueButton = [...document.querySelectorAll("button")].find(
    (button) => button.textContent === "Continue",
  ) as HTMLButtonElement;
  expect(continueButton.disabled).toBe(true);
  await clickAccessible("Select Anthropic key");
  expect(continueButton.disabled).toBe(false);
  await click("Continue");
  const clearButton = document.querySelector<HTMLButtonElement>(
    ".ai-data-action-footer .button--danger",
  )!;
  expect(clearButton.disabled).toBe(true);
  await click("Back");
  await click("Cancel");
  expect(
    document.querySelector('[aria-labelledby="ai-data-action-title"]'),
  ).toBeNull();
  expect(
    native.invoke.mock.calls.some(
      ([command]) => command === "clear_ai_monitoring",
    ),
  ).toBe(false);
});

it("requests daily buckets for week/month and monthly buckets for year/all time", async () => {
  for (const period of ["Week", "Month", "Year", "All time"]) {
    await click(period);
    expect(native.invoke).toHaveBeenCalledWith(
      "load_ai_monitoring",
      expect.objectContaining({
        bucketSize: period === "Week" || period === "Month" ? "day" : "month",
      }),
    );
  }
});
it("restarts only the expanded key cap while preserving lifetime spend and Data", async () => {
  let loads = 0;
  let didReset = false;
  const previous = native.invoke.getMockImplementation()!;
  const cap = {
    credentialId: "second-id",
    period: "all_time",
    currency: "USD",
    timeZone: "UTC",
    limitMicros: 1_000_000,
    countedMicros: 50_000,
    reservedMicros: 0,
    unresolvedMicros: 0,
    revision: 1,
  };
  native.invoke.mockImplementation((command: string, args?: any) => {
    if (command === "load_ai_key_settings") {
      if (args?.credentialId === "second-id") loads++;
      return Promise.resolve({
        ok: true,
        value: {
          cap: { ...cap, countedMicros: didReset ? 0 : 50_000 },
          lifetimeSpendByCurrencyMicros: { USD: 900_000 },
        },
      });
    }
    if (command === "reset_ai_cap") {
      didReset = true;
      return Promise.resolve({ ok: true, value: true });
    }
    return previous(command, args);
  });
  await act(async () => {
    root.render(<AiWorkspace key="reset-caps" blocked={false} />);
  });
  expect(document.body.textContent).toContain("$0.05/$1.00");
  expect(document.querySelectorAll(".ai-key-spend")[1]?.textContent).toContain(
    "$0.90",
  );
  await act(async () => {
    document
      .querySelectorAll<HTMLButtonElement>(".ai-key-cap-actions button")[2]
      .click();
  });
  expect(didReset).toBe(false);
  await click("Confirm restart");
  expect(native.invoke).toHaveBeenCalledWith("reset_ai_cap", {
    request: { period: "all_time", credentialId: "second-id" },
  });
  expect(loads).toBe(2);
  expect(document.body.textContent).toContain("$0.00/$1.00");
  expect(document.querySelectorAll(".ai-key-spend")[1]?.textContent).toContain(
    "$0.90",
  );
  expect(document.body.textContent).toContain(
    "Lifetime spend and Data are unchanged.",
  );
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "OpenAI key",
  );
  expect(
    native.invoke.mock.calls.some(
      ([command]) => command === "clear_ai_monitoring",
    ),
  ).toBe(false);
});
