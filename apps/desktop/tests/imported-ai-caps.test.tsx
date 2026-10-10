// @vitest-environment jsdom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { ImportedAiCaps } from "../src/shared/ImportedAiCaps";
const native = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: native.invoke }));
let root: Root | undefined;
afterEach(async () => {
  await act(async () => root?.unmount());
  document.body.replaceChildren();
  native.invoke.mockReset();
});
it("requires explicit target and confirmation before activating an archived cap", async () => {
  const profile = "019a0000-0000-7000-8000-000000000001";
  const policy = "019a0000-0000-7000-8000-000000000002";
  const oldKey = "019a0000-0000-7000-8000-000000000003";
  const newKey = "019a0000-0000-7000-8000-000000000004";
  native.invoke.mockImplementation(async (name: string) =>
    name === "load_imported_ai_guardrails"
      ? {
          ok: true,
          value: {
            profileId: profile,
            policies: [
              {
                id: policy,
                credentialId: oldKey,
                period: "month",
                currency: "USD",
                timeZone: "UTC",
                limitMicros: 1000000,
                activatedAtUnixMs: 1000,
                periodStartUnixMs: 1000,
                periodEndUnixMs: 2000,
                countedMicros: 200000,
                reservedMicros: 100000,
                unresolvedMicros: 50000,
                revision: 1,
              },
            ],
          },
        }
      : { ok: true, value: true },
  );
  const changed = vi.fn();
  const host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => {
    root!.render(
      <ImportedAiCaps
        keys={[
          {
            credentialId: newKey,
            createdAt: "2026-10-01T00:00:00Z",
            provider: "openai",
            model: "fixture",
            paused: false,
            removed: false,
            cleanupRequired: false,
          },
        ]}
        blocked={false}
        refreshRevision={0}
        onChanged={changed}
      />,
    );
  });
  const choose = host.querySelector("select")!;
  await act(async () => {
    choose.value = policy;
    choose.dispatchEvent(new Event("change", { bubbles: true }));
  });
  const button = host.querySelector("button")!;
  expect(button.disabled).toBe(true);
  const target = host.querySelectorAll("select")[1]!;
  expect(target.value).toBe("");
  await act(async () => {
    target.value = newKey;
    target.dispatchEvent(new Event("change", { bubbles: true }));
  });
  expect(button.disabled).toBe(true);
  expect(native.invoke).not.toHaveBeenCalledWith(
    "bind_imported_ai_guardrail",
    expect.anything(),
  );
  const confirmation = host.querySelector("input")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(
      HTMLInputElement.prototype,
      "value",
    )!.set!.call(confirmation, "RESTORE LIFETIME CAP");
    confirmation.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(button.disabled).toBe(false);
  await act(async () => button.click());
  expect(native.invoke).toHaveBeenCalledWith("bind_imported_ai_guardrail", {
    expectedProfileId: profile,
    importId: policy,
    credentialId: newKey,
    confirmation: "RESTORE LIFETIME CAP",
  });
  expect(changed).toHaveBeenCalledOnce();
});
