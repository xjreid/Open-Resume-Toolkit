import { describe, expect, it } from "vitest";
import { isHealthCommandResponse } from "../generated/health";

describe("generated health validator", () => {
  it("rejects unrecognized fields", () => {
    expect(
      isHealthCommandResponse({
        ok: true,
        value: {
          status: "ok",
          appVersion: "0.0.0-dev",
          profile: "development",
          storageStatus: "development_gated",
          contractVersion: 2,
          unexpected: "authority",
        },
      }),
    ).toBe(false);
  });
});
