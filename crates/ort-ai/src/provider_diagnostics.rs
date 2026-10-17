//! Classify provider request rejections into fixed, locally authored hints.
//! Never retain raw messages, field values, filenames, or response content.
use crate::Provider;
use serde_json::Value;

#[must_use]
pub fn provider_request_issues(provider: Provider, bytes: &[u8]) -> Vec<String> {
    if provider != Provider::Gemini || bytes.len() > 16 * 1024 {
        return vec![];
    }
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return vec![];
    };
    // Google errors may identify the offending field in BadRequest details or
    // only in the message. Use both as classification input; copy neither out.
    let mut evidence = value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if let Some(details) = value.pointer("/error/details").and_then(Value::as_array) {
        for field in details
            .iter()
            .filter_map(|d| d.get("fieldViolations").and_then(Value::as_array))
            .flatten()
            .filter_map(|violation| violation.get("field").and_then(Value::as_str))
        {
            evidence.push(' ');
            evidence.push_str(&field.to_ascii_lowercase());
        }
    }
    let mut issues = vec![];
    if [
        "schema",
        "responseformat",
        "response_format",
        "response format",
        "maxitems",
        "max_items",
    ]
    .iter()
    .any(|word| evidence.contains(word))
    {
        issues.push("Gemini's rejection references the structured-output configuration. Check the response schema and its complexity; this is not evidence that the PDF is damaged.".into());
    }
    if ["thinking", "temperature", "top_p", "topp", "top_k", "topk"]
        .iter()
        .any(|word| evidence.contains(word))
    {
        issues.push("Gemini's rejection references generation settings. Check which parameters the selected model accepts.".into());
    }
    if [
        "inline_data",
        "inlinedata",
        "mime_type",
        "mimetype",
        "pdf",
        "document",
    ]
    .iter()
    .any(|word| evidence.contains(word))
    {
        issues.push("Gemini's rejection references document input. Check the PDF encoding, format, and provider file limits.".into());
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn distinguishes_schema_settings_and_document_rejections_without_echoing_private_text() {
        for (message, category) in [
            (
                "response_schema too many states PRIVATE_RESUME_CONTENT",
                "structured-output",
            ),
            (
                "Unsupported thinking_level PRIVATE_KEY",
                "generation settings",
            ),
            ("Invalid PDF PRIVATE_FILENAME", "document input"),
        ] {
            let bytes =
                json!({"error":{"status":"INVALID_ARGUMENT","message":message}}).to_string();
            let issues = provider_request_issues(Provider::Gemini, bytes.as_bytes());
            assert_eq!(issues.len(), 1);
            assert!(issues[0].contains(category));
            assert!(!issues[0].contains("PRIVATE"));
        }
        let bytes = json!({"error":{"message":"PRIVATE_TEXT", "details":[{"fieldViolations":[{"field":"generation_config.response_format.text.schema.PRIVATE_FIELD","description":"PRIVATE_DESCRIPTION"}]}]}}).to_string();
        assert_eq!(
            provider_request_issues(Provider::Gemini, bytes.as_bytes()).len(),
            1
        );
        assert!(
            !provider_request_issues(Provider::Gemini, bytes.as_bytes())[0].contains("PRIVATE")
        );
        for provider in [Provider::OpenAi, Provider::Anthropic] {
            assert!(provider_request_issues(provider, bytes.as_bytes()).is_empty());
        }
        let oversized = vec![b'x'; 16 * 1024 + 1];
        for bytes in [
            b"invalid JSON".as_slice(),
            b"{\"error\":{\"message\":\"Request contains an invalid argument.\"}}",
            oversized.as_slice(),
        ] {
            assert!(provider_request_issues(Provider::Gemini, bytes).is_empty());
        }
    }
}
