import { invoke } from "@tauri-apps/api/core";
import { isCommandResponse, type CommandResponse } from "@ort/contracts/resume";

export type AiImportStatus = {
  ready: boolean;
  busy: boolean;
  provider: string | null;
  model: string | null;
  reasoning: string | null;
  issue: string | null;
};

function isStatus(value: unknown): value is AiImportStatus {
  if (typeof value !== "object" || value === null) return false;
  const status = value as Record<string, unknown>;
  return (
    typeof status.ready === "boolean" &&
    typeof status.busy === "boolean" &&
    ["provider", "model", "reasoning", "issue"].every(
      (key) => status[key] === null || typeof status[key] === "string",
    ) &&
    (!status.ready ||
      (typeof status.provider === "string" &&
        typeof status.model === "string" &&
        status.issue === null))
  );
}

export async function aiImportStatus(): Promise<
  CommandResponse<AiImportStatus>
> {
  try {
    const result: unknown = await invoke("ai_import_status");
    if (isCommandResponse(result, isStatus)) return result;
  } catch {
    /* Native transport unavailable; keep import disabled. */
  }
  return {
    ok: false,
    error: {
      code: "AI_IMPORT_STATUS_UNAVAILABLE",
      messageKey: "errors.aiImportFailed",
      retryable: true,
      details: {},
    },
  };
}

export function aiImportError(code: string): string {
  const messages: Record<string, string> = {
    AI_DISABLED:
      "Choose a provider and model in AI & monitoring to import with AI.",
    AI_MODEL_IMAGE_UNSUPPORTED:
      "This model cannot read PDFs; choose one that supports images.",
    AI_PDF_RENDER_FAILED: "This PDF could not be read; try an unencrypted PDF.",
    AI_PDF_LINK_LIMIT:
      "This PDF has too many links; try exporting a simpler PDF.",
    AI_PDF_RENDER_TIMEOUT:
      "This PDF took too long to prepare; try a smaller file.",
    AI_PDF_PAGE_LIMIT:
      "This PDF exceeds the page limit; choose a shorter document.",
    AI_PDF_IMAGES_TOO_LARGE:
      "This PDF is too complex to import; try a simpler PDF.",
    PLAN_AUTH_REQUIRED: "Sign in on the Codex page to import with AI.",
    PLAN_REQUEST_TIMEOUT:
      "AI import timed out; check usage in AI & monitoring before retrying.",
    AI_IMPORT_STATUS_UNAVAILABLE:
      "AI readiness could not be checked; check your connection.",
    AI_OUTPUT_INVALID:
      "Import failed because AI returned an invalid resume; try another model.",
    AI_PDF_REQUIRED:
      "AI import needs a PDF; export your resume as a PDF first.",
    AI_PDF_TOO_LARGE: "This PDF is larger than 10 MB; choose a smaller file.",
    AI_PROVIDER_BAD_REQUEST:
      "Your AI provider could not accept this document; try another file or model.",
    AI_PROVIDER_TIMEOUT:
      "AI import timed out; check usage in AI & monitoring before retrying.",
    AI_CREDENTIAL_MISSING:
      "Your API key is unavailable; reconnect it in AI & monitoring.",
    AI_CONFIGURATION_INVALID:
      "Choose an active provider and model in AI & monitoring.",
    AI_CAP_REJECTED:
      "This import exceeds your spending cap; adjust it in AI & monitoring.",
    AI_BUSY: "AI is busy; try again when the current request finishes.",
    AI_CANCELLED: "AI import was cancelled.",
    IMPORT_INVALID_SOURCE:
      "This document could not be read; choose a valid PDF under 10 MB.",
    IMPORT_REVIEW_OUTCOME_UNKNOWN:
      "The import result could not be confirmed; reload before trying again.",
    IMPORT_REVIEW_UNAVAILABLE:
      "Your imported resume could not be opened; try importing again.",
    LOCAL_DATA_OPERATION_BUSY:
      "Another file operation is in progress; try again when it finishes.",
    REVISION_CONFLICT:
      "Your resume or AI settings changed; reload before importing again.",
    STORAGE_UNAVAILABLE:
      "Local storage is unavailable; reopen your workspace before importing.",
    AI_PROVIDER_UNAVAILABLE:
      "The AI connection failed; check your internet connection.",
    AI_PROVIDER_SERVICE_UNAVAILABLE:
      "Your AI provider is unavailable; try again later.",
    AI_PROVIDER_TEMPORARY:
      "Your AI provider is temporarily unavailable; try again later.",
    AI_RATE_LIMITED:
      "Your AI provider's usage limit was reached; try again later.",
    AI_AUTHENTICATION_FAILED:
      "Your API key was not accepted; reconnect it in AI & monitoring.",
    AI_MODEL_UNAVAILABLE:
      "This AI model is unavailable; choose another in AI & monitoring.",
    AI_OUTPUT_LIMIT: "AI returned an incomplete resume; try another model.",
    AI_OUTPUT_INCOMPLETE:
      "AI returned an incomplete resume; try importing again.",
    AI_OUTPUT_BLOCKED: "Your AI provider declined to read this document.",
    PLAN_RESERVE_REJECTED: "This import would exceed your Codex usage reserve.",
    PLAN_QUOTA_UNAVAILABLE:
      "Codex usage could not be checked; try again later.",
    PLAN_MODEL_UNAVAILABLE:
      "This Codex model is unavailable; choose another model.",
    PLAN_RUNTIME_UNAVAILABLE:
      "Codex is unavailable; reconnect it on the Codex page.",
  };
  return (
    messages[code] ??
    "Import failed; try again or choose another import method."
  );
}

export const importProviderLabel = (provider: string | null) =>
  ({
    openai: "OpenAI",
    anthropic: "Anthropic",
    gemini: "Google Gemini",
    codex: "Codex",
  })[provider ?? ""] ??
  provider ??
  "No provider selected";
