import { planErrorMessage } from "./chatgpt-plan-presentation";
import "./styles/ai-failure-details.css";

function record(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

export function AiFailureDetailsView({
  details,
  open = false,
}: {
  details: Record<string, unknown>;
  open?: boolean;
}) {
  return (
    <details className="ai-failure-details" open={open}>
      <summary>Failure details</summary>
      <AiFailureDetailsContent details={details} />
    </details>
  );
}

export function AiFailureDetailsContent({
  details,
}: {
  details: Record<string, unknown>;
}) {
  const diagnostic = record(details.diagnostic);
  const usage = record(details.usage);
  const text = (value: unknown) => (typeof value === "string" ? value : null);
  const number = (value: unknown) =>
    typeof value === "number" && Number.isFinite(value) ? value : null;
  const call = number(details.callNumber);
  const maximum = number(details.maximumCalls);
  const duration = number(details.durationMs);
  const rows: [string, string | number | null][] = [
    ["Error code", text(diagnostic.code)],
    ["Category", text(details.category)],
    ["Provider", text(details.provider)],
    ["Selected model", text(details.model)],
    ["Serving model", text(details.effectiveModel)],
    ["Operation", text(details.operationType)],
    [
      details.connectionSource === "chatgpt_plan" ? "Pass" : "Call",
      call == null
        ? null
        : `${call}${maximum == null ? "" : ` of ${maximum}`}${maximum === 4 ? ` · ${call === 1 ? "draft" : call === 2 ? "source and editorial review" : "correction"}` : ""}`,
    ],
    [
      "Connection",
      details.connectionSource === "chatgpt_plan" ? "Codex" : null,
    ],
    ["Reasoning", text(details.reasoning)],
    ["Reported internal retries", number(details.reportedRetries)],
    [
      "Monetary cost",
      details.monetaryCostTracking === "not_tracked"
        ? "$0 · not tracked"
        : null,
    ],
    ["HTTP status", number(diagnostic.httpStatus)],
    ["Finish reason", text(diagnostic.finishReason)],
    ["Provider reason", text(diagnostic.providerReason)],
    [
      "Duration",
      duration == null ? null : `${(duration / 1000).toFixed(1)} seconds`,
    ],
    ["Rendered pages", number(diagnostic.pageCount)],
    [
      "Usage confirmed",
      typeof details.usageComplete === "boolean"
        ? details.usageComplete
          ? "Yes"
          : "No · usage may be uncertain"
        : null,
    ],
    ["Input tokens", number(usage.inputTokens)],
    ["Output tokens", number(usage.outputTokens)],
    ["Thinking tokens", number(usage.reasoningTokens)],
    ["Operation ID", text(details.operationId)],
    ["Attempt ID", text(details.attemptId)],
  ];
  const issues = Array.isArray(diagnostic.validationIssues)
    ? diagnostic.validationIssues.filter(
        (item): item is string => typeof item === "string",
      )
    : [];
  return (
    <div className="ai-failure-content">
      <dl>
        {rows
          .filter(([, value]) => value !== null)
          .map(([label, value]) => (
            <div key={label}>
              <dt>{label}</dt>
              <dd>{value}</dd>
            </div>
          ))}
      </dl>
      {issues.length > 0 && (
        <>
          <p>Local validation</p>
          <ul>
            {issues.map((issue, index) => (
              <li key={index}>{issue}</li>
            ))}
          </ul>
        </>
      )}
    </div>
  );
}

export function aiFailureReason(
  code: string | undefined,
  category: string | null,
) {
  if (code?.startsWith("PLAN_")) return planErrorMessage(code);
  const reasons: Record<string, string> = {
    PLAN_AUTH_REQUIRED: "ChatGPT sign-in required",
    PLAN_RESERVE_REJECTED: "Plan reserve reached",
    PLAN_QUOTA_UNAVAILABLE: "Plan quota unavailable",
    PLAN_QUOTA_INVALID: "Invalid plan quota",
    PLAN_MODEL_UNAVAILABLE: "Plan model unavailable",
    PLAN_PROVIDER_REJECTED: "Codex request rejected",
    PLAN_RUNTIME_UNAVAILABLE: "Codex stopped",
    PLAN_REQUEST_TIMEOUT: "Codex pass timed out",
    PLAN_CONTAINMENT_VIOLATION: "Codex tool activity blocked",
    PLAN_PROTOCOL_INVALID: "Incompatible Codex response",
    AI_PROVIDER_SERVICE_UNAVAILABLE:
      "Provider unavailable or overloaded (HTTP 503). Wait before retrying or choose another model.",
    AI_PROVIDER_TEMPORARY:
      "Temporary provider server error. Wait before retrying or choose another model.",
    AI_PROVIDER_UNAVAILABLE:
      "The connection failed or was interrupted. Check your network; usage may be uncertain.",
    AI_PROVIDER_TIMEOUT:
      "The request timed out. Check uncertain usage before retrying; reduce input or choose another model.",
    AI_RATE_LIMITED:
      "Provider rate limit reached (HTTP 429). Check the provider's quota and wait before retrying.",
    AI_AUTHENTICATION_FAILED:
      "The provider rejected authentication or permissions. Check the key in My Keys.",
    AI_PROVIDER_BAD_REQUEST:
      "The provider rejected the request (HTTP 400). Check model support and request configuration.",
    AI_MODEL_UNAVAILABLE:
      "The model is unavailable to this key (HTTP 404). Choose an available model.",
    AI_OUTPUT_LIMIT:
      "Generation reached the output token limit (MAX_TOKENS). Try a more selective instruction or another model.",
    AI_OUTPUT_BLOCKED:
      "Generation was blocked by the provider's safety or content policy. Review the supplied content.",
    AI_OUTPUT_INCOMPLETE:
      "The response ended without a successful completion event. Nothing was accepted.",
    AI_OUTPUT_INVALID:
      "The response could not be read or accepted. Review the local validation details.",
    AI_MATERIAL_INVALID:
      "The resume still did not match the required format after the bounded correction passes.",
    AI_GROUNDING_FAILED:
      "The resume contained invalid or cross-entry source references after correction.",
    AI_PAGE_FIT_FAILED:
      "The resume could not fit exactly one page within four calls using the fixed layout.",
    AI_REVIEW_FAILED:
      "The final source and editorial review still had unresolved issues.",
    AI_USAGE_UNKNOWN:
      "Provider usage could not be confirmed. Another paid call was not started.",
  };
  if (code && reasons[code]) return reasons[code];
  const legacy: Record<string, string> = {
    transient:
      "Temporary provider or connection failure. This older record does not include the exact status code.",
    rate_limit:
      "Provider rate limit reached. Check the provider's quota before retrying.",
    authentication:
      "Authentication or permissions failed. Check the API key in My Keys.",
    invalid_output:
      "The provider output was rejected. This older record does not include the exact validation reason.",
    safety: "Provider content policy blocked the response.",
    timeout: "The request timed out. Usage may be uncertain.",
    provider:
      "The provider request failed. This older record does not include the exact status code.",
  };
  return (
    legacy[category ?? ""] ??
    "The attempt failed. Detailed diagnostics were not recorded for this older attempt."
  );
}

export function aiFailureName(
  code: string | undefined,
  category: string | null,
) {
  const names: Record<string, string> = {
    PLAN_AUTH_REQUIRED: "ChatGPT sign-in required",
    PLAN_RESERVE_REJECTED: "Plan reserve reached",
    PLAN_QUOTA_UNAVAILABLE: "Plan quota unavailable",
    PLAN_QUOTA_INVALID: "Invalid plan quota",
    PLAN_MODEL_UNAVAILABLE: "Plan model unavailable",
    PLAN_PROVIDER_REJECTED: "Codex request rejected",
    PLAN_RUNTIME_UNAVAILABLE: "Codex stopped",
    PLAN_REQUEST_TIMEOUT: "Codex pass timed out",
    PLAN_CONTAINMENT_VIOLATION: "Codex tool activity blocked",
    PLAN_PROTOCOL_INVALID: "Incompatible Codex response",
    AI_PROVIDER_SERVICE_UNAVAILABLE: "Provider unavailable",
    AI_PROVIDER_TEMPORARY: "Provider server error",
    AI_PROVIDER_UNAVAILABLE: "Connection interrupted",
    AI_PROVIDER_TIMEOUT: "Request timed out",
    AI_RATE_LIMITED: "Rate limit reached",
    AI_AUTHENTICATION_FAILED: "Authentication failed",
    AI_PROVIDER_BAD_REQUEST: "Request rejected",
    AI_MODEL_UNAVAILABLE: "Model unavailable",
    AI_MODEL_MISMATCH: "Unexpected serving model",
    AI_OUTPUT_LIMIT: "Output token limit reached",
    AI_OUTPUT_BLOCKED: "Generation blocked",
    AI_OUTPUT_INCOMPLETE: "Incomplete response",
    AI_OUTPUT_INVALID: "Unreadable response",
    AI_MATERIAL_INVALID: "Invalid response format",
    AI_GROUNDING_FAILED: "Invalid source references",
    AI_PAGE_FIT_FAILED: "One-page fit failed",
    AI_REVIEW_FAILED: "Unresolved review issues",
    AI_USAGE_UNKNOWN: "Usage unconfirmed",
  };
  const legacy: Record<string, string> = {
    transient: "Provider or connection failure",
    rate_limit: "Rate limit reached",
    authentication: "Authentication failed",
    invalid_output: "Invalid response",
    safety: "Generation blocked",
    timeout: "Request timed out",
    provider: "Provider request failed",
  };
  return (code && names[code]) || legacy[category ?? ""] || "AI request failed";
}
