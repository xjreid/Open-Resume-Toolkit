import { invoke } from "@tauri-apps/api/core";
import { isDesktopValue, matchesSchema } from "@ort/contracts/wire-decoder";
import type { DesktopCommands, ErrorEnvelope } from "@ort/contracts/wire";

export type DesktopResponse<T> =
  | { ok: true; value: T }
  | { ok: false; error: ErrorEnvelope };

export class DesktopCommandError extends Error {
  constructor(public readonly envelope: ErrorEnvelope) {
    super(envelope.code);
    this.name = "DesktopCommandError";
  }
}
type Arguments<K extends keyof DesktopCommands> =
  {} extends DesktopCommands[K]["args"]
    ? [args?: DesktopCommands[K]["args"]]
    : [args: DesktopCommands[K]["args"]];

export async function invokeDesktop<K extends keyof DesktopCommands>(
  command: K,
  ...[args]: Arguments<K>
): Promise<DesktopResponse<DesktopCommands[K]["value"]>> {
  const value: unknown =
    args === undefined ? await invoke(command) : await invoke(command, args);
  if (typeof value === "object" && value !== null && "ok" in value) {
    if (
      value.ok === true &&
      "value" in value &&
      isDesktopValue(command, value.value)
    ) {
      return { ok: true, value: value.value };
    }
    if (
      value.ok === false &&
      "error" in value &&
      matchesSchema(value.error, { $ref: "#/$defs/ErrorEnvelope" })
    ) {
      return { ok: false, error: value.error as ErrorEnvelope };
    }
  }
  return {
    ok: false,
    error: {
      code: "INVALID_NATIVE_RESPONSE",
      messageKey: "errors.invalidNativeResponse",
      retryable: false,
      details: {},
    },
  };
}

export async function desktopCommand<K extends keyof DesktopCommands>(
  command: K,
  ...args: Arguments<K>
): Promise<DesktopCommands[K]["value"]> {
  const result = await invokeDesktop(command, ...args);
  if (!result.ok) throw new DesktopCommandError(result.error);
  return result.value;
}
