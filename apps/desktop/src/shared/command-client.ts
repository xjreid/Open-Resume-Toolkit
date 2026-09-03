import { invoke } from "@tauri-apps/api/core";
import {
  isBackupRecoveryStatusCommandResponse,
  isDeleteSafetyCopyCommandResponse,
  isExportBackupCommandResponse,
  isRestoreBackupCommandResponse,
  isRollbackSafetyCopyCommandResponse,
  isValidateBackupCommandResponse,
  type BackupRecoveryStatus,
  type BackupRecoveryStatusCommandResponse,
  type BackupRecoveryStatusRequest,
  type DeleteSafetyCopyCommandResponse,
  type DeleteSafetyCopyRequest,
  type ExportBackupCommandResponse,
  type ExportBackupRequest,
  type RestoreBackupCommandResponse,
  type RestoreBackupRequest,
  type RollbackSafetyCopyCommandResponse,
  type RollbackSafetyCopyRequest,
  type ValidateBackupCommandResponse,
  type ValidateBackupRequest,
} from "@ort/contracts/backup";
import {
  isPdfPreviewCommandResponse,
  isPdfExportCommandResponse,
  isPdfRenderHistoryCommandResponse,
  type PdfPreview,
  type PdfPreviewCommandResponse,
  type PdfExportCommandResponse,
  type PdfRenderHistory,
  type PdfRenderHistoryCommandResponse,
} from "@ort/contracts/pdf";
import {
  isExportTextCommandResponse,
  isExportDocxCommandResponse,
  type ExportFormat,
  type ExportSource,
  type ExportTextCommandResponse,
  type ExportTextRequest,
} from "@ort/contracts/export";
import {
  isCloseCommandResponse,
  type CloseCommandResponse,
  type CloseDecision,
  type ResolveCloseRequest,
} from "@ort/contracts/lifecycle";
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
import {
  isStorageUsageCommandResponse,
  type StorageUsage,
  type StorageUsageCommandResponse,
} from "@ort/contracts/storage";

export async function requestHealth(): Promise<HealthCommandResponse> {
  try {
    const request: HealthRequest = {
      contractVersion: CONTRACT_VERSION,
      requestId: crypto.randomUUID(),
      payload: {},
    };

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

export async function requestStorageUsage(): Promise<StorageUsageCommandResponse> {
  try {
    const response: unknown = await invoke("load_storage_usage", {
      request: requestEnvelope({}),
    });
    return isStorageUsageCommandResponse(response)
      ? response
      : invalidCommandResponse<StorageUsage>();
  } catch {
    return commandUnavailable<StorageUsage>();
  }
}

export async function exportPortableBackup(
  passphrase: string,
): Promise<ExportBackupCommandResponse> {
  try {
    const request: ExportBackupRequest = requestEnvelope({ passphrase });
    const response: unknown = await invoke("export_portable_backup", {
      request,
    });
    return isExportBackupCommandResponse(response)
      ? response
      : invalidCommandResponse();
  } catch {
    return commandUnavailable();
  }
}

export async function validatePortableBackup(
  passphrase: string,
): Promise<ValidateBackupCommandResponse> {
  try {
    const request: ValidateBackupRequest = requestEnvelope({ passphrase });
    const response: unknown = await invoke("validate_portable_backup", {
      request,
    });
    return isValidateBackupCommandResponse(response)
      ? response
      : invalidCommandResponse();
  } catch {
    return commandUnavailable();
  }
}

export async function restorePortableBackup(
  passphrase: string,
  confirmation: string,
): Promise<RestoreBackupCommandResponse> {
  try {
    const request: RestoreBackupRequest = requestEnvelope({
      passphrase,
      confirmation,
    });
    const response: unknown = await invoke("restore_portable_backup", {
      request,
    });
    return isRestoreBackupCommandResponse(response)
      ? response
      : invalidCommandResponse();
  } catch {
    return commandUnavailable();
  }
}

export async function requestBackupRecoveryStatus(): Promise<BackupRecoveryStatusCommandResponse> {
  try {
    const request: BackupRecoveryStatusRequest = requestEnvelope({});
    const response: unknown = await invoke("load_backup_recovery_status", {
      request,
    });
    return isBackupRecoveryStatusCommandResponse(response)
      ? response
      : invalidCommandResponse<BackupRecoveryStatus>();
  } catch {
    return commandUnavailable<BackupRecoveryStatus>();
  }
}

export async function rollbackSafetyCopy(
  confirmation: string,
): Promise<RollbackSafetyCopyCommandResponse> {
  try {
    const request: RollbackSafetyCopyRequest = requestEnvelope({
      confirmation,
    });
    const response: unknown = await invoke("rollback_safety_copy", { request });
    return isRollbackSafetyCopyCommandResponse(response)
      ? response
      : invalidCommandResponse();
  } catch {
    return commandUnavailable();
  }
}

export async function deleteSafetyCopy(
  confirmation: string,
): Promise<DeleteSafetyCopyCommandResponse> {
  try {
    const request: DeleteSafetyCopyRequest = requestEnvelope({ confirmation });
    const response: unknown = await invoke("delete_safety_copy", { request });
    return isDeleteSafetyCopyCommandResponse(response)
      ? response
      : invalidCommandResponse();
  } catch {
    return commandUnavailable();
  }
}

export async function saveResume(
  expectedRevision: number | null,
  document: ResumeDocument,
): Promise<VersionedResumeCommandResponse> {
  try {
    const request: SaveResumeRequest = requestEnvelope({
      expectedRevision,
      document,
    });
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
  try {
    const request: PublishResumeRequest = requestEnvelope({
      expectedDraftRevision,
    });
    const response: unknown = await invoke("publish_resume", { request });
    return isPublishResumeCommandResponse(response)
      ? response
      : invalidCommandResponse();
  } catch {
    return commandUnavailable();
  }
}

export async function requestCloseStatus(): Promise<CloseCommandResponse> {
  try {
    const response: unknown = await invoke("close_status", {
      request: requestEnvelope({}),
    });
    return isCloseCommandResponse(response)
      ? response
      : invalidCommandResponse();
  } catch {
    return commandUnavailable();
  }
}

export async function exportResumeText(
  source: ExportSource,
  expectedRevision: number,
): Promise<ExportTextCommandResponse> {
  return exportResumeDocument(source, expectedRevision, "txt");
}

export async function exportResumeDocument(
  source: ExportSource,
  expectedRevision: number,
  format: ExportFormat,
): Promise<ExportTextCommandResponse> {
  try {
    const request: ExportTextRequest = requestEnvelope({
      source,
      expectedRevision,
    });
    const command =
      format === "docx" ? "export_resume_docx" : "export_resume_text";
    const validate =
      format === "docx"
        ? isExportDocxCommandResponse
        : isExportTextCommandResponse;
    const response: unknown = await invoke(command, { request });
    if (!validate(response)) return invalidCommandResponse();
    if (
      response.ok &&
      response.value.status === "exported" &&
      (response.value.source !== source ||
        response.value.revision !== expectedRevision)
    ) {
      return invalidCommandResponse();
    }
    return response;
  } catch {
    return commandUnavailable();
  }
}

export async function resolveClose(
  attempt: string,
  decision: CloseDecision,
): Promise<CloseCommandResponse> {
  try {
    const request: ResolveCloseRequest = requestEnvelope({ attempt, decision });
    const response: unknown = await invoke("resolve_close", { request });
    return isCloseCommandResponse(response)
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

export async function renderResumePdf(
  source: ExportSource,
  expectedRevision: number,
): Promise<PdfPreviewCommandResponse> {
  try {
    const response: unknown = await invoke("render_resume_pdf", {
      request: requestEnvelope({ source, expectedRevision }),
    });
    if (!isPdfPreviewCommandResponse(response)) return invalidCommandResponse();
    if (
      response.ok &&
      (response.value.source !== source ||
        response.value.revision !== expectedRevision)
    )
      return invalidCommandResponse();
    return response;
  } catch {
    return commandUnavailable();
  }
}

export async function requestPdfRenderHistory(): Promise<PdfRenderHistoryCommandResponse> {
  try {
    const response: unknown = await invoke("load_pdf_render_history", {
      request: requestEnvelope({}),
    });
    return isPdfRenderHistoryCommandResponse(response)
      ? response
      : invalidCommandResponse<PdfRenderHistory>();
  } catch {
    return commandUnavailable<PdfRenderHistory>();
  }
}

export async function exportResumePdf(
  preview: PdfPreview,
): Promise<PdfExportCommandResponse> {
  try {
    const response: unknown = await invoke("export_resume_pdf", {
      request: requestEnvelope({ renderId: preview.renderId }),
    });
    if (!isPdfExportCommandResponse(response)) return invalidCommandResponse();
    if (
      response.ok &&
      response.value.status === "exported" &&
      (response.value.renderId !== preview.renderId ||
        response.value.pdfSha256 !== preview.receipt.pdfSha256 ||
        response.value.byteCount !== preview.receipt.byteCount)
    )
      return invalidCommandResponse();
    return response;
  } catch {
    return commandUnavailable();
  }
}

export async function releaseResumePdf(renderId: string): Promise<void> {
  try {
    await invoke("release_resume_pdf", {
      request: requestEnvelope({ renderId }),
    });
  } catch {
    /* Native cache remains bounded and expires on access. */
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
