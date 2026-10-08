import { expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import {
  invokeDesktop,
  desktopCommand,
  DesktopCommandError,
} from "./desktop-client";
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

it("preserves diagnostic envelopes when a command throws", async () => {
  const envelope = {
    code: "AI_PROVIDER_SERVICE_UNAVAILABLE",
    messageKey: "errors.aiProviderFailed",
    retryable: true,
    details: {
      diagnostic: { code: "AI_PROVIDER_SERVICE_UNAVAILABLE", httpStatus: 503 },
    },
  };
  vi.mocked(invoke).mockResolvedValue({ ok: false, error: envelope });
  const failure = await desktopCommand("load_ai_connection").catch(
    (error) => error,
  );
  expect(failure).toBeInstanceOf(DesktopCommandError);
  expect(failure.message).toBe("AI_PROVIDER_SERVICE_UNAVAILABLE");
  expect(failure.envelope).toEqual(envelope);
});

it("rejects malformed native success values before state can consume them", async () => {
  vi.mocked(invoke).mockResolvedValue({
    ok: true,
    value: { keys: "not an array", primaryCredentialId: null },
  });
  expect(await invokeDesktop("load_ai_connection")).toMatchObject({
    ok: false,
    error: { code: "INVALID_NATIVE_RESPONSE" },
  });
  vi.mocked(invoke).mockResolvedValue({
    ok: true,
    value: { keys: [], primaryCredentialId: null },
  });
  expect(await invokeDesktop("load_ai_connection")).toEqual({
    ok: true,
    value: { keys: [], primaryCredentialId: null },
  });
});

it("rejects retained content inside a tracker metadata response", async () => {
  vi.mocked(invoke).mockResolvedValue({
    ok: true,
    value: [
      {
        id: "id",
        revision: 1,
        hasResume: true,
        hasCoverLetter: false,
        answerCount: 0,
        value: {
          company: "A",
          title: "",
          location: "",
          dateApplied: "",
          status: "applied",
          customStatus: "",
          sourceUrl: "",
          resume: { title: "leaked" },
        },
      },
    ],
  });
  expect(await invokeDesktop("list_tracker_entries")).toMatchObject({
    ok: false,
    error: { code: "INVALID_NATIVE_RESPONSE" },
  });
});
