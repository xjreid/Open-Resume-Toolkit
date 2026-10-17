import { invoke } from "@tauri-apps/api/core";
import { isCommandResponse, type CommandResponse } from "@ort/contracts/resume";
import { aiFailureReason } from "./AiFailureDetailsView";

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
      "Connect Codex, or select an active API key and model in AI & monitoring to import with AI.",
    AI_MODEL_IMAGE_UNSUPPORTED:
      "The selected Codex model does not report image support. Select a model that accepts images to import PDF pages.",
    AI_PDF_RENDER_FAILED:
      "This PDF could not be prepared for AI import. It may be encrypted or damaged. Try exporting a new PDF.",
    AI_PDF_LINK_LIMIT:
      "This PDF has too many hyperlinks or overly long link data. Try a simpler PDF; nothing was sent to your provider.",
    AI_PDF_RENDER_TIMEOUT:
      "Preparing this PDF took too long. Try a smaller or simpler PDF. Nothing was sent to your provider.",
    AI_PDF_PAGE_LIMIT:
      "Codex PDF import supports 1–10 pages; API imports support up to 1,000 pages. Choose a shorter PDF and try again.",
    AI_PDF_IMAGES_TOO_LARGE:
      "The rendered PDF pages exceed the image size limit. Try a PDF with fewer or simpler pages.",
    PLAN_AUTH_REQUIRED:
      "Connect your account on the Codex page to import with Codex.",
    PLAN_REQUEST_TIMEOUT:
      "Codex import timed out after 2 minutes. Check usage on the Codex page before retrying.",
    AI_IMPORT_STATUS_UNAVAILABLE:
      "AI readiness could not be checked. Check the connection and try again.",
    AI_OUTPUT_INVALID:
      "The AI response does not match the resume format. Review the validation details, then try again or select another model.",
    AI_PDF_REQUIRED:
      "AI import accepts PDF files. Export your resume as a PDF and try again.",
    AI_PDF_TOO_LARGE:
      "This PDF is larger than 10 MB. Choose a smaller PDF and try again.",
    AI_PROVIDER_BAD_REQUEST:
      "The provider rejected the import request. Its document input or structured-output settings may be unsupported. Check the failure details before trying another file or model.",
    AI_PROVIDER_TIMEOUT:
      "The AI import timed out after 2 minutes. Check usage in AI & monitoring before retrying, or choose another model.",
    AI_CREDENTIAL_MISSING:
      "The selected API key is unavailable. Reconnect it in AI & monitoring.",
    AI_CONFIGURATION_INVALID:
      "The AI provider configuration is incomplete. Select an active key and model in AI & monitoring.",
    AI_CAP_REJECTED:
      "Your spending cap cannot cover this PDF request. PDF imports reserve the model’s full input allowance because their token count is unknown. Review your cap in AI & monitoring or use local import.",
    AI_BUSY:
      "Another AI request is running. Wait for it to finish, then try again.",
    AI_CANCELLED: "AI import was cancelled. Your saved resume is unchanged.",
    IMPORT_INVALID_SOURCE:
      "This file could not be read as a PDF. Check that it is a regular PDF file no larger than 10 MB and try again.",
    IMPORT_REVIEW_OUTCOME_UNKNOWN:
      "The import result could not be confirmed. Reload the app before importing again.",
    IMPORT_REVIEW_UNAVAILABLE:
      "The extracted resume could not be opened for review. Your saved resume is unchanged. Try importing again.",
    LOCAL_DATA_OPERATION_BUSY:
      "Another file operation is in progress. Try again when it finishes.",
    REVISION_CONFLICT:
      "Your saved resume or AI profile changed. Reload before importing again.",
    STORAGE_UNAVAILABLE:
      "Local storage is unavailable. Reopen your workspace before importing.",
  };
  return messages[code] ?? aiFailureReason(code, null);
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
