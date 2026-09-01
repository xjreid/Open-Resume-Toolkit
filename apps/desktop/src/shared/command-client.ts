import { invoke } from "@tauri-apps/api/core";
import {
  CONTRACT_VERSION,
  isHealthCommandResponse,
  type HealthCommandResponse,
  type HealthRequest,
} from "@ort/contracts/health";

export async function requestHealth(): Promise<HealthCommandResponse> {
  const request: HealthRequest = {
    contractVersion: CONTRACT_VERSION,
    requestId: crypto.randomUUID(),
    payload: {},
  };

  try {
    const response: unknown = await invoke("health", { request });
    if (!isHealthCommandResponse(response)) {
      return invalidResponse();
    }
    return response;
  } catch {
    return {
      ok: false,
      error: {
        code: "COMMAND_UNAVAILABLE",
        messageKey: "errors.commandUnavailable",
        retryable: true,
        details: {},
      },
    };
  }
}

function invalidResponse(): HealthCommandResponse {
  return {
    ok: false,
    error: {
      code: "INVALID_RESPONSE",
      messageKey: "errors.invalidResponse",
      retryable: false,
      details: {},
    },
  };
}
