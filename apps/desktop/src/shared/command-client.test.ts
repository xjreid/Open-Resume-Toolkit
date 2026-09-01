import { describe, expect, it } from "vitest";
import { isHealthCommandResponse } from "@ort/contracts/health";

describe("health response validation", () => {
  it("accepts the bounded M1 health response", () => {
    expect(
      isHealthCommandResponse({
        ok: true,
        value: {
          status: "ok",
          appVersion: "0.0.0-dev",
          profile: "development",
          storageStatus: "development_gated",
          contractVersion: 2,
        },
      }),
    ).toBe(true);
  });

  it("rejects unexpected response fields and states", () => {
    expect(
      isHealthCommandResponse({ ok: true, value: { status: "unknown" } }),
    ).toBe(false);
  });
});
