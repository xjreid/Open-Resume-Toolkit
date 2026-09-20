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
    nextIdentificationNumber: 3,
    keys: [
      {
        credentialId: "fixture-id",
        identificationNumber: 1,
        provider: "openai",
        preset: "balanced",
        paused: false,
        removed: false,
        cleanupRequired: false,
      },
      {
        credentialId: "second-id",
        identificationNumber: 2,
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
  await clickAccessible("Test key #1");
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
  await clickAccessible("Test key #1");
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

  await clickAccessible("Remove key #1");
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
  await clickAccessible("Test key #1");
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
  await clickAccessible("Test key #1");
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
  await clickAccessible("Edit key #2 spending limit");
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
    "Key #1",
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
  await click("Choose view");
  const picker = document.querySelector('[aria-label="Choose activity view"]')!;
  expect(picker.textContent).toContain("OpenAI");
  expect(picker.textContent).toContain("fixture-model");
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
  await click("Set primary key");
  const selectionResult = await axe.run(document.getElementById("root")!, {
    rules: { "color-contrast": { enabled: false } },
  });
  expect(selectionResult.violations.map((item) => item.id)).toEqual([]);
  await click("Cancel selection");
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
  expect(document.querySelector('[aria-label="Pause key #1"]')).toBeNull();
  expect(
    document.querySelector('[aria-label="Key #1 options"]'),
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
  await clickAccessible("Rename key #1");
  const input = document.querySelector<HTMLInputElement>(
    '[aria-label="Key #1 name"]',
  )!;
  await typeInput(input, "Personal");
  expect(native.invoke).toHaveBeenCalledWith("rename_ai_key", {
    request: { credentialId: "fixture-id", name: "Personal" },
  });
  expect(document.querySelector('[aria-label="Save key name"]')).toBeNull();
  expect(document.querySelector('[aria-label="Cancel rename"]')).toBeNull();
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "Personal · OpenAI",
  );
  await act(async () => {
    input.blur();
  });
  expect(document.querySelector('[aria-label="Key #1 name"]')).toBeNull();
  expect(document.querySelector(".ai-key-name")?.textContent).toBe("Personal");
  await click("Choose view");
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
  await clickAccessible("Rename key #1");
  const input = document.querySelector<HTMLInputElement>(
    '[aria-label="Key #1 name"]',
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
        key.identificationNumber === 1 ? { ...key, name } : key,
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
  await clickAccessible("Rename key #1");
  const input = document.querySelector<HTMLInputElement>(
    '[aria-label="Key #1 name"]',
  )!;
  await typeInput(input, "Unsaved");
  await act(async () => {
    input.blur();
  });
  expect(document.body.textContent).toContain("Name not saved");
  expect(document.querySelector(".ai-key-name")?.textContent).toBe("Key #1");
});

it("shows all key information and controls without expansion", async () => {
  expect(document.querySelectorAll(".ai-key-card-layout")).toHaveLength(2);
  expect(document.body.textContent).toContain("Balanced: fixture-model");
  expect(document.body.textContent).not.toContain("ORT-tested input limit");
  expect(document.body.textContent).not.toContain("Pricing & model details");
  expect(document.querySelector(".ai-key-customization")).toBeNull();
  expect(document.querySelector('[aria-label="Customize key #1"]')).toBeNull();
  expect(
    document.querySelector(".ai-key-card-layout")?.firstElementChild?.className,
  ).toBe("ai-key-card-info");
  expect(
    document.querySelector<HTMLProgressElement>(
      '[aria-label="Key #1 spending cap used"]',
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
    document.querySelector('[aria-label="Edit key #1 spending limit"]'),
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
async function selectCandidate(number: number) {
  const radio = document.querySelector<HTMLInputElement>(
    `[aria-label="Select key #${number} as primary"]`,
  )!;
  expect(radio.disabled).toBe(false);
  await act(async () => {
    radio.click();
  });
}
async function clickAccessible(label: string) {
  if (
    /^(Test|Pause|Unpause|Remove) key #\d+$/.test(label) &&
    !document.querySelector(`button[aria-label="${label}"]`)
  ) {
    await clickAccessible(`Key #${label.match(/#(\d+)/)![1]} options`);
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
  await click("Choose view");
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
it("changes primary only after selection and Confirm", async () => {
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "Key #1",
  );
  returnRegistry({ ...connection.value, primaryCredentialId: "second-id" });
  await click("Set primary key");
  expect(
    document.querySelector<HTMLButtonElement>(
      '[aria-label="Confirm primary key"]',
    )?.disabled,
  ).toBe(false);
  await act(async () => {
    document
      .querySelectorAll(".ai-key-row")[1]
      .dispatchEvent(new dom.window.MouseEvent("click", { bubbles: true }));
  });
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "Key #1",
  );
  expect(
    native.invoke.mock.calls.some(([command]) => command === "change_ai_key"),
  ).toBe(false);
  expect(document.querySelectorAll(".ai-key-row--candidate")).toHaveLength(1);
  await clickAccessible("Confirm primary key");
  expect(native.invoke).toHaveBeenCalledWith("change_ai_key", {
    request: { credentialId: "second-id", action: "select_primary" },
  });
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "Key #2",
  );
  expect(document.querySelectorAll(".ai-key-row--primary")).toHaveLength(1);
  expect(
    document.querySelector('input[name="primary-key-selection"]'),
  ).toBeNull();
});

it("clears the primary when Confirm is clicked with no key selected", async () => {
  returnRegistry({ ...connection.value, primaryCredentialId: null });
  await click("Set primary key");
  expect(
    document.querySelector<HTMLInputElement>(
      '[aria-label="Select key #1 as primary"]',
    )?.checked,
  ).toBe(false);
  await clickAccessible("Confirm primary key");
  expect(native.invoke).toHaveBeenCalledWith("clear_ai_primary");
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "No primary key selected",
  );
});

it("allows a pending primary choice to be deselected before Confirm", async () => {
  returnRegistry({ ...connection.value, primaryCredentialId: null });
  await click("Set primary key");
  await selectCandidate(2);
  expect(document.querySelectorAll(".ai-key-row--candidate")).toHaveLength(1);
  await act(async () => {
    document
      .querySelectorAll(".ai-key-row")[1]
      .dispatchEvent(new dom.window.MouseEvent("click", { bubbles: true }));
  });
  expect(document.querySelectorAll(".ai-key-row--candidate")).toHaveLength(0);
  await clickAccessible("Confirm primary key");
  expect(native.invoke).toHaveBeenCalledWith("clear_ai_primary");
  expect(
    native.invoke.mock.calls.some(([command]) => command === "change_ai_key"),
  ).toBe(false);
});

it("cancels primary selection without changing the active key", async () => {
  await click("Set primary key");
  await selectCandidate(2);
  await click("Cancel selection");
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "Key #1",
  );
  expect(
    native.invoke.mock.calls.some(([command]) => command === "change_ai_key"),
  ).toBe(false);
});

it("invalidates a pending primary choice when that key is paused", async () => {
  await click("Set primary key");
  await selectCandidate(2);
  returnRegistry({
    ...connection.value,
    keys: connection.value.keys.map((key) =>
      key.identificationNumber === 2 ? { ...key, paused: true } : key,
    ),
  });
  await clickAccessible("Pause key #2");
  expect(
    document.querySelector<HTMLInputElement>(
      '[aria-label="Select key #2 as primary"]',
    )?.disabled,
  ).toBe(true);
  expect(
    document.querySelector<HTMLButtonElement>(
      '[aria-label="Confirm primary key"]',
    )?.disabled,
  ).toBe(false);
  expect(document.querySelectorAll(".ai-key-row--candidate")).toHaveLength(0);
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "Key #1",
  );
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
  await clickAccessible("Edit key #1 spending limit");
  const input = document.querySelector<HTMLInputElement>(
    '[aria-label="Key #1 spending limit"]',
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
      '[aria-label="Key #1 spending cap used"]',
    )?.value,
  ).toBe(0);
});

it("pausing the primary leaves no selected key; unpausing does not select one", async () => {
  returnRegistry({
    ...connection.value,
    primaryCredentialId: null,
    keys: connection.value.keys.map((key) =>
      key.credentialId === "fixture-id" ? { ...key, paused: true } : key,
    ),
  });
  await clickAccessible("Pause key #1");
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "No primary key selected",
  );
  await click("Set primary key");
  expect(
    document.querySelector<HTMLInputElement>(
      '[aria-label="Select key #1 as primary"]',
    )?.disabled,
  ).toBe(true);
  await click("Cancel selection");
  expect(document.querySelector('[aria-label="Rename key #1"]')).toBeNull();
  expect(
    document.querySelector<HTMLSelectElement>(
      '[aria-label="Key #1 settings"] select',
    )?.disabled,
  ).toBe(true);
  await clickAccessible("Key #1 options");
  expect(
    document.querySelector<HTMLButtonElement>('[aria-label="Test key #1"]')
      ?.disabled,
  ).toBe(false);
  await act(async () => {
    document.body.dispatchEvent(
      new dom.window.Event("pointerdown", { bubbles: true }),
    );
  });
  returnRegistry({ ...connection.value, primaryCredentialId: null });
  await clickAccessible("Unpause key #1");
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "No primary key selected",
  );
  await click("Set primary key");
  expect(
    document.querySelector<HTMLInputElement>(
      '[aria-label="Select key #1 as primary"]',
    )?.disabled,
  ).toBe(false);
  await click("Cancel selection");
});

it("removing the primary preserves its activity filter and never selects another key", async () => {
  returnRegistry({
    ...connection.value,
    primaryCredentialId: null,
    keys: connection.value.keys.map((key) =>
      key.credentialId === "fixture-id"
        ? { ...key, paused: true, removed: true }
        : key,
    ),
  });
  await clickAccessible("Remove key #1");
  await click("Remove key");
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "No primary key selected",
  );
  expect(document.querySelectorAll(".ai-key-row")).toHaveLength(1);
  await click("Choose view");
  expect(
    document.querySelector('[aria-label="View activity for Key #1"]')
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
  expect(form.querySelector<HTMLInputElement>("input")?.type).toBe("text");
  await clickAccessible("Hide API key");
  expect(form.querySelector<HTMLInputElement>("input")?.type).toBe("password");
  await clickAccessible("Select Gemini");
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
  const previous = native.invoke.getMockImplementation()!;
  const next = {
    ...connection.value,
    nextIdentificationNumber: 4,
    keys: [
      ...connection.value.keys,
      {
        ...connection.value.keys[0],
        credentialId: "third-id",
        identificationNumber: 3,
        provider: "gemini",
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
    request: { provider: "gemini", apiKey: "test-only-key" },
  });
  expect(document.querySelector('[aria-label="Add new API key"]')).toBeNull();
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "Key #1",
  );
  expect(document.querySelectorAll(".ai-key-row")).toHaveLength(3);
  expect(
    document.querySelector(
      '[aria-label="Key #3 · Gemini"] option[value="openai"]',
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
  expect(reopened.querySelector<HTMLInputElement>("input")?.value).toBe("");
  expect(
    reopened.querySelectorAll('.ai-provider-option[aria-pressed="true"]'),
  ).toHaveLength(0);
});

it("testing a non-primary key passes its identity through preview and confirmation without switching primary", async () => {
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
  await clickAccessible("Test key #2");
  expect(native.invoke).toHaveBeenCalledWith("preview_ai_test", {
    credentialId: "second-id",
  });
  await click("Confirm and send test");
  expect(native.invoke).toHaveBeenCalledWith(
    "test_ai_connection",
    expect.objectContaining({ credentialId: "second-id" }),
  );
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "Key #1",
  );
  expect(
    native.invoke.mock.calls.some(([command]) => command === "change_ai_key"),
  ).toBe(false);
});

it("shows cleanup-required keys as unusable after failed vault cleanup", async () => {
  const previous = native.invoke.getMockImplementation()!;
  native.invoke.mockImplementation((command: string, args?: unknown) => {
    if (command === "change_ai_key")
      return Promise.resolve({
        ok: false,
        error: { code: "AI_CREDENTIAL_CLEANUP_REQUIRED" },
      });
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
  await clickAccessible("Remove key #1");
  await click("Remove key");
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "No primary key selected",
  );
  expect(document.body.textContent).toContain("Key cleanup could not finish");
  await clickAccessible("Key #1 options");
  expect(
    document.querySelector<HTMLButtonElement>('[aria-label="Unpause key #1"]')
      ?.disabled,
  ).toBe(true);
  await act(async () => {
    document.body.dispatchEvent(
      new dom.window.Event("pointerdown", { bubbles: true }),
    );
  });
  await click("Set primary key");
  expect(
    document.querySelector<HTMLInputElement>(
      '[aria-label="Select key #1 as primary"]',
    )?.disabled,
  ).toBe(true);
  await click("Cancel selection");
  await clickAccessible("Key #1 options");
  expect(
    document.querySelector<HTMLButtonElement>('[aria-label="Remove key #1"]')
      ?.disabled,
  ).toBe(false);
});

it("dismisses the key menu on outside click and Escape, with keyboard navigation", async () => {
  await clickAccessible("Key #1 options");
  const menu = document.querySelector('[role="menu"]')!;
  expect(menu.textContent).toBe("Test keyPauseRemove");
  expect(document.activeElement?.getAttribute("aria-label")).toBe(
    "Test key #1",
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
    "Pause key #1",
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
    "Key #1 options",
  );
  await clickAccessible("Key #1 options");
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
  await clickAccessible("Pause key #1");
  const card = document.querySelector(".ai-key-row--paused")!;
  expect(card.querySelector('[aria-label="Rename key #1"]')).toBeNull();
  expect(card.querySelector<HTMLSelectElement>("select")?.disabled).toBe(true);
  expect(
    card.querySelector<HTMLButtonElement>(
      '[aria-label="Edit key #1 spending limit"]',
    )?.disabled,
  ).toBe(true);
  await clickAccessible("Test key #1");
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
  await clickAccessible("Pause key #1");
  expect(document.querySelector(".ai-primary")?.textContent).toContain(
    "Primary key unavailable",
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
  await chooseDataView("Key #2");
  expect(native.invoke).toHaveBeenCalledWith(
    "load_ai_monitoring",
    expect.objectContaining({ credentialId: "second-id" }),
  );
  await click("Export JSON…");
  expect(
    document.querySelector('[aria-labelledby="ai-data-action-title"]')
      ?.textContent,
  ).toContain("Step 1 of 2Export activity");
  await clickAccessible("Select Key #1");
  await click("Continue");
  expect(document.body.textContent).toContain("Only months with activity");
  await clickAccessible("Select September 2026");
  await clickAccessible("Select August 2026");
  await click("Export 2 months");
  expect(native.invoke).toHaveBeenCalledWith(
    "export_ai_monitoring",
    expect.objectContaining({
      credentialId: "fixture-id",
      months: [
        expect.objectContaining({ label: "2026-09" }),
        expect.objectContaining({ label: "2026-08" }),
      ],
    }),
  );
  await click("Choose activity…");
  await clickAccessible("Select all keys");
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
      credentialId: null,
      months: [expect.objectContaining({ label: "2026-09" })],
    }),
  );
  await chooseDataView("all keys");
  expect(native.invoke).toHaveBeenCalledWith(
    "load_ai_monitoring",
    expect.objectContaining({ credentialId: null }),
  );
});

it("requires explicit scope choices and can cancel the activity workflow", async () => {
  await click("Choose activity…");
  const continueButton = [...document.querySelectorAll("button")].find(
    (button) => button.textContent === "Continue",
  ) as HTMLButtonElement;
  expect(continueButton.disabled).toBe(true);
  await clickAccessible("Select Key #2");
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
    "Key #1",
  );
  expect(
    native.invoke.mock.calls.some(
      ([command]) => command === "clear_ai_monitoring",
    ),
  ).toBe(false);
});
