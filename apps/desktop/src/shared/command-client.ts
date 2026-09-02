import { invoke } from "@tauri-apps/api/core";
import {
  CONTRACT_VERSION,
  isHealthCommandResponse,
  type HealthCommandResponse,
  type HealthRequest,
} from "@ort/contracts/health";
import {
  isPublishResumeCommandResponse,
  isResumeWorkspaceCommandResponse,
  isVersionedResumeCommandResponse,
  type CommandResponse,
  type PublishResumeCommandResponse,
  type PublishResumeRequest,
  type ResumeDocument,
  type ResumeWorkspace,
  type ResumeWorkspaceCommandResponse,
  type SaveResumeRequest,
  type VersionedResume,
  type VersionedResumeCommandResponse,
} from "@ort/contracts/resume";

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

export async function requestResumeWorkspace(): Promise<ResumeWorkspaceCommandResponse> {
  try {
    const response: unknown = await invoke("load_resume", {
      request: requestEnvelope({}),
    });
    return isResumeWorkspaceCommandResponse(response)
      ? response
      : invalidCommandResponse<ResumeWorkspace>();
  } catch {
    return commandUnavailable<ResumeWorkspace>();
  }
}

export async function saveResume(
  expectedRevision: number | null,
  document: ResumeDocument,
): Promise<VersionedResumeCommandResponse> {
  const request: SaveResumeRequest = requestEnvelope({
    expectedRevision,
    document,
  });
  try {
    const response: unknown = await invoke("save_resume", { request });
    return isVersionedResumeCommandResponse(response)
      ? response
      : invalidCommandResponse<VersionedResume>();
  } catch {
    return commandUnavailable<VersionedResume>();
  }
}

export async function publishResume(
  expectedDraftRevision: number,
): Promise<PublishResumeCommandResponse> {
  const request: PublishResumeRequest = requestEnvelope({
    expectedDraftRevision,
  });
  try {
    const response: unknown = await invoke("publish_resume", { request });
    return isPublishResumeCommandResponse(response)
      ? response
      : invalidCommandResponse();
  } catch {
    return commandUnavailable();
  }
}

function requestEnvelope<T>(payload: T) {
  return {
    contractVersion: CONTRACT_VERSION,
    requestId: crypto.randomUUID(),
    payload,
  };
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

function invalidCommandResponse<T>(): CommandResponse<T> {
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

function commandUnavailable<T>(): CommandResponse<T> {
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
