import { invoke } from "@tauri-apps/api/core";
import { CONTRACT_VERSION } from "@ort/contracts/health";
import {
  isCommandResponse,
  isVersionedResumeCommandResponse,
  type CommandResponse,
  type VersionedResume,
} from "@ort/contracts/resume";
import {
  isImportReviewSnapshot,
  type ImportReviewSnapshot,
  type ImportChoices,
  type ImportReviewRequest,
  type ApplyImportReviewRequest,
} from "@ort/contracts/import";

function failure<T>(): CommandResponse<T> {
  return {
    ok: false,
    error: {
      code: "IMPORT_REVIEW_OUTCOME_UNKNOWN",
      messageKey: "errors.importReviewUnavailable",
      retryable: false,
      details: {},
    },
  };
}
export async function readImportReview(
  reviewId: string,
): Promise<CommandResponse<ImportReviewSnapshot>> {
  const request: ImportReviewRequest = {
    contractVersion: CONTRACT_VERSION,
    requestId: crypto.randomUUID(),
    payload: { reviewId },
  };
  try {
    const response: unknown = await invoke("read_import_review", { request });
    return isCommandResponse(response, isImportReviewSnapshot)
      ? response
      : failure();
  } catch {
    return failure();
  }
}
export async function applyImportReview(
  reviewId: string,
  choices: ImportChoices,
): Promise<CommandResponse<VersionedResume>> {
  const request: ApplyImportReviewRequest = {
    contractVersion: CONTRACT_VERSION,
    requestId: crypto.randomUUID(),
    payload: { reviewId, decisionsJson: JSON.stringify(choices) },
  };
  try {
    const response: unknown = await invoke("apply_import_review", { request });
    return isVersionedResumeCommandResponse(response) ? response : failure();
  } catch {
    return failure();
  }
}
export async function cancelImportReview(
  reviewId: string,
): Promise<CommandResponse<boolean>> {
  const request: ImportReviewRequest = {
    contractVersion: CONTRACT_VERSION,
    requestId: crypto.randomUUID(),
    payload: { reviewId },
  };
  try {
    const response: unknown = await invoke("cancel_import_review", { request });
    return isCommandResponse(
      response,
      (value): value is boolean => value === true,
    )
      ? response
      : failure();
  } catch {
    return failure();
  }
}

export async function documentImportAvailable(): Promise<boolean> {
  try {
    return (await invoke("document_import_available")) === true;
  } catch {
    return false;
  }
}
export async function beginDocumentImport(
  expectedRevision: number | null,
): Promise<CommandResponse<ImportReviewSnapshot | null>> {
  const request: import("@ort/contracts/import").BeginImportRequest = {
    contractVersion: CONTRACT_VERSION,
    requestId: crypto.randomUUID(),
    payload: { expectedRevision },
  };
  try {
    const response: unknown = await invoke("begin_document_import", {
      request,
    });
    return isCommandResponse(
      response,
      (value): value is ImportReviewSnapshot | null =>
        value === null || isImportReviewSnapshot(value),
    )
      ? response
      : failure();
  } catch {
    return failure();
  }
}
export async function cancelDocumentImport(): Promise<
  CommandResponse<boolean>
> {
  try {
    const response: unknown = await invoke("cancel_document_import");
    return isCommandResponse(
      response,
      (value): value is boolean => value === true,
    )
      ? response
      : failure();
  } catch {
    return failure();
  }
}
