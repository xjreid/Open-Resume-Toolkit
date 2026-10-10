import { expect, it } from "vitest";
import fixtures from "../generated/wire-fixtures.json";
import { isDesktopValue } from "../generated/wire-decoder";
import type { DesktopCommands } from "../generated/wire";

it("decodes overlay reasoning as a supported level or null", () => {
  const context = {
    aiBusy: false,
    aiLabel: "Using Codex",
    aiReady: true,
    codexConnected: true,
    browserConnected: false,
    connectionSource: "chatgpt_plan",
    model: "gpt-5.6-sol",
    modelOptions: [],
    profileId: "01992187-74f7-7000-8000-000000000001",
    publishedRevision: 1,
    reasoning: "high",
    selectedKeyId: null,
    selectedKeyReady: true,
  };
  expect(isDesktopValue("application_context", context)).toBe(true);
  expect(
    isDesktopValue("application_context", { ...context, reasoning: null }),
  ).toBe(true);
  expect(
    isDesktopValue("application_context", { ...context, reasoning: "unknown" }),
  ).toBe(false);
  expect(
    isDesktopValue("application_context", {
      ...context,
      codexConnected: "true",
    }),
  ).toBe(false);
});
for (const [command, value] of Object.entries(fixtures)) {
  it(`decodes the Rust-produced ${command} response`, () => {
    expect(isDesktopValue(command as keyof DesktopCommands, value)).toBe(true);
  });
}
it("rejects malformed retained content and incomplete billing payloads", () => {
  expect(
    isDesktopValue("get_tracker_entry", {
      ...fixtures.get_tracker_entry,
      value: {
        ...fixtures.get_tracker_entry.value,
        resume: { title: "incomplete" },
      },
    }),
  ).toBe(false);
  expect(isDesktopValue("load_ai_monitoring", { attempts: 1 })).toBe(false);
  expect(
    isDesktopValue("load_application_workspace", {
      ...fixtures.load_application_workspace,
      revision: "1",
    }),
  ).toBe(false);
});

it("rejects malformed UUIDs and inherited property names", () => {
  expect(
    isDesktopValue("load_ai_connection", {
      keys: [],
      primaryCredentialId: "malformed",
    }),
  ).toBe(false);
  expect(
    isDesktopValue("load_ai_connection", {
      keys: [],
      primaryCredentialId: null,
      toString: "unexpected",
    }),
  ).toBe(false);
});

it("rejects unknown installer states, invalid progress and incomplete status", () => {
  const valid = fixtures.load_codex_runtime_install;
  for (const value of [
    { ...valid, phase: "executing_arbitrary_command" },
    { ...valid, downloadedBytes: -1 },
    { ...valid, totalBytes: "98089521" },
    { phase: "complete" },
  ]) {
    expect(isDesktopValue("load_codex_runtime_install", value)).toBe(false);
  }
});

it("requires a complete, typed offline runtime readiness result", () => {
  expect(
    isDesktopValue("check_codex_runtime", { ready: true, errorCode: null }),
  ).toBe(true);
  expect(
    isDesktopValue("check_codex_runtime", { ready: "true", errorCode: null }),
  ).toBe(false);
  expect(isDesktopValue("check_codex_runtime", { ready: true })).toBe(false);
});
