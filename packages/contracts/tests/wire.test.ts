import { expect, it } from "vitest";
import fixtures from "../generated/wire-fixtures.json";
import { isDesktopValue } from "../generated/wire-decoder";
import type { DesktopCommands } from "../generated/wire";
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
