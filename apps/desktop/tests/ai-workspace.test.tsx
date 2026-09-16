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
    mode: "direct_api",
    provider: "openai",
    preset: "balanced",
    credentialId: "fixture-id",
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
  await click("Review synthetic test");
  expect(document.body.textContent).toContain(
    "Conservative maximum reservation: 0.1230 USD",
  );
  expect(document.body.textContent).toContain("at most 512 tokens");
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
      expectedModel: "fixture-model",
      expectedMaximumCostMicros: 123_000,
    }),
  );
  expect(document.body.textContent).toContain("Synthetic request completed");
  expect(document.body.textContent).toContain("10 input, 2 cached input");
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
  await click("Review synthetic test");
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
  await click("Review synthetic test");
  await click("Confirm and send test");
  expect(document.body.textContent).toContain("reservation remains unresolved");
});

it("enables a cap with the current IANA zone and an explicit zero baseline disclosure", async () => {
  await act(async () => {
    await Promise.resolve();
  });
  const input = [...document.querySelectorAll("input")].find(
    (element) => element.getAttribute("inputmode") === "decimal",
  )!;
  await act(async () => {
    const setter = Object.getOwnPropertyDescriptor(
      dom.window.HTMLInputElement.prototype,
      "value",
    )!.set!;
    setter.call(input, "1.25");
    input.dispatchEvent(
      new dom.window.InputEvent("input", {
        bubbles: true,
        inputType: "insertText",
        data: "1.25",
      }),
    );
  });
  expect(input.value).toBe("1.25");
  expect(
    [...document.querySelectorAll("button")]
      .find((element) => element.textContent?.trim() === "Save cap")
      ?.hasAttribute("disabled"),
  ).toBe(false);
  native.invoke.mockImplementation((command: string, args?: unknown) => {
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
    if (command === "save_ai_cap") {
      expect(args).toEqual({
        request: expect.objectContaining({
          period: "month",
          limitMicros: 1_250_000,
          expectedRevision: null,
        }),
      });
      return Promise.resolve({
        ok: true,
        value: {
          credentialId: "fixture-id",
          period: "month",
          currency: "USD",
          timeZone: "UTC",
          limitMicros: 1_250_000,
          activatedAtUnixMs: 1_000,
          periodStartUnixMs: 1_000,
          periodEndUnixMs: 2_000,
          countedMicros: 0,
          reservedMicros: 0,
          unresolvedMicros: 0,
          revision: 1,
        },
      });
    }
    return Promise.reject(new Error(`unexpected ${command}`));
  });
  expect(document.body.textContent).toContain(
    "New caps start at zero when activated",
  );
  await click("Save cap");
  expect(document.body.textContent).toContain("month cap saved");
  expect(document.body.textContent).toContain("0.0000 / 1.2500 USD");
});

it("shows selected-period buckets and content-free breakdowns", async () => {
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
  expect(document.body.textContent).toContain("Token usage over time");
  expect(document.body.textContent).toContain("2026-09-15: 10 tokens");
  expect(document.body.textContent).toContain("fixture-model 1");
  expect(document.body.textContent).toContain("credential_test 1");
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

it("labels the AI and Monitoring controls for accessibility", async () => {
  const result = await axe.run(document.getElementById("root")!, {
    rules: { "color-contrast": { enabled: false } },
  });
  expect(result.violations.map((item) => item.id)).toEqual([]);
});

it("shows signed model details and never represents missing prices as zero", async () => {
  expect(document.body.textContent).toContain("fixture-model");
  expect(document.body.textContent).toContain(
    "ORT-tested input limit: 64,000 tokens",
  );
  expect(document.body.textContent).toContain("Input: 2.0000 USD");
  expect(document.body.textContent).toContain("Cache write: unavailable");
  expect(document.body.textContent).toContain(
    "Official source: official-pricing-fixture",
  );
});

it("defaults the form and catalog details to the saved provider", async () => {
  native.invoke.mockImplementation((command: string) => {
    if (command === "load_ai_connection")
      return Promise.resolve({
        ok: true,
        value: {
          ...connection.value,
          provider: "anthropic",
        },
      });
    if (command === "load_ai_catalog")
      return Promise.resolve({
        ok: true,
        value: {
          ...catalog.value,
          entries: [
            {
              ...catalog.value.entries[0],
              provider: "anthropic",
              model: "claude-fixture",
            },
          ],
        },
      });
    if (command === "load_ai_monitoring")
      return Promise.resolve(emptyMonitoring);
    if (command === "load_ai_caps")
      return Promise.resolve({ ok: true, value: [] });
    if (command === "load_ai_retention")
      return Promise.resolve({
        ok: true,
        value: { policy: "retain_until_cleared", removedOperations: 0 },
      });
    return Promise.reject(new Error(`unexpected ${command}`));
  });
  await act(async () => {
    root.render(<AiWorkspace key="saved-provider" blocked={false} />);
  });
  const providerSelect = [...document.querySelectorAll("select")].find(
    (select) =>
      [...select.options].some((option) => option.value === "anthropic"),
  );
  expect(providerSelect?.value).toBe("anthropic");
  expect(document.body.textContent).toContain("claude-fixture");
});

it("reloads connection state after a credential-cleanup failure", async () => {
  native.invoke.mockImplementation((command: string) => {
    if (command === "remove_ai_credential")
      return Promise.resolve({
        ok: false,
        error: { code: "AI_CREDENTIAL_CLEANUP_REQUIRED" },
      });
    if (command === "load_ai_connection")
      return Promise.resolve({
        ok: true,
        value: {
          mode: "no_ai",
          provider: null,
          preset: null,
          credentialId: null,
        },
      });
    if (command === "load_ai_catalog") return Promise.resolve(catalog);
    if (command === "load_ai_monitoring")
      return Promise.resolve(emptyMonitoring);
    if (command === "load_ai_caps")
      return Promise.resolve({ ok: true, value: [] });
    if (command === "load_ai_retention")
      return Promise.resolve({
        ok: true,
        value: { policy: "retain_until_cleared", removedOperations: 0 },
      });
    return Promise.reject(new Error(`unexpected ${command}`));
  });
  await click("Remove saved credential");
  await click("Remove credential");
  expect(document.body.textContent).toContain(
    "Credential removal could not be verified",
  );
  expect(document.body.textContent).toContain("No AI");
});

it("reloads the recorded baseline after an all-time cap reset", async () => {
  let capLoads = 0;
  let didReset = false;
  const original = {
    credentialId: "fixture-id",
    period: "all_time",
    currency: "USD",
    timeZone: "UTC",
    limitMicros: 1_000_000,
    activatedAtUnixMs: 1_000,
    periodStartUnixMs: 1_000,
    periodEndUnixMs: null,
    countedMicros: 50_000,
    reservedMicros: 0,
    unresolvedMicros: 0,
    revision: 1,
  };
  native.invoke.mockImplementation((command: string) => {
    if (command === "load_ai_catalog") return Promise.resolve(catalog);
    if (command === "load_ai_connection") return Promise.resolve(connection);
    if (command === "load_ai_monitoring")
      return Promise.resolve(emptyMonitoring);
    if (command === "load_ai_caps") {
      capLoads++;
      return Promise.resolve({
        ok: true,
        value: [
          didReset
            ? {
                ...original,
                activatedAtUnixMs: 2_000,
                periodStartUnixMs: 2_000,
                countedMicros: 0,
                revision: 2,
              }
            : original,
        ],
      });
    }
    if (command === "load_ai_retention")
      return Promise.resolve({
        ok: true,
        value: { policy: "retain_until_cleared", removedOperations: 0 },
      });
    if (command === "reset_ai_cap") {
      didReset = true;
      return Promise.resolve({ ok: true, value: true });
    }
    return Promise.reject(new Error(`unexpected ${command}`));
  });
  await act(async () => {
    root.render(<AiWorkspace key="cap-baseline" blocked={false} />);
  });
  const period = [...document.querySelectorAll("select")].find((select) =>
    [...select.options].some((option) => option.value === "all_time"),
  )!;
  await act(async () => {
    period.value = "all_time";
    period.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
  expect(document.body.textContent).toContain("0.0500 / 1.0000 USD");
  await click("Reset all-time baseline");
  await click("Confirm reset");
  expect(native.invoke).toHaveBeenCalledWith("reset_ai_cap", {
    request: { period: "all_time" },
  });
  expect(capLoads).toBeGreaterThanOrEqual(3);
  expect(document.body.textContent).toContain("0.0000 / 1.0000 USD");
  expect(document.body.textContent).toContain("new activation time");
});
