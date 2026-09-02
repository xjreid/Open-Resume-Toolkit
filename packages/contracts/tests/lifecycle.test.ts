import { describe, expect, it } from "vitest";
import { isCloseCommandResponse } from "../generated/lifecycle";

describe("lifecycle response boundary", () => {
  it("accepts only a null or bounded UUIDv7 native attempt", () => {
    for (const pendingAttempt of [
      null,
      "01990000-0000-7000-8000-000000000000",
    ]) {
      expect(
        isCloseCommandResponse({ ok: true, value: { pendingAttempt } }),
      ).toBe(true);
    }
    for (const value of [
      {},
      { pendingAttempt: 1 },
      { pendingAttempt: "invented" },
      { pendingAttempt: null, approveQuit: true },
    ]) {
      expect(isCloseCommandResponse({ ok: true, value })).toBe(false);
    }
  });
});
