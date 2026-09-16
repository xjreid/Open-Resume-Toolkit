use super::*;
use ed25519_dalek::{Signer, SigningKey};

fn entry() -> CatalogEntry {
    CatalogEntry {
        provider: Provider::OpenAi,
        model: "synthetic-model".into(),
        preset: Preset::Balanced,
        operations: vec![OperationType::TailorResume],
        max_input_tokens: 64_000,
        max_output_tokens: 5_000,
        currency: "USD".into(),
        prices: vec![
            Price {
                category: PriceCategory::Input,
                micros_per_million: 2_000_000,
            },
            Price {
                category: PriceCategory::Output,
                micros_per_million: 12_000_000,
            },
        ],
        source: "https://example.invalid/pricing".into(),
        verified_at: "2026-09-01T00:00:00Z".into(),
        effective_from: "2026-09-01T00:00:00Z".into(),
        effective_to: None,
        disabled: false,
    }
}
fn request() -> NormalizedRequest {
    NormalizedRequest {
        operation: OperationType::TailorResume,
        model: "synthetic-model".into(),
        system: "Return JSON".into(),
        input: json!({"safe":true}),
        max_output_tokens: 20,
    }
}

#[test]
fn credentials_and_transport_debug_are_redacted() {
    let key = ApiKey::new(b"SYNTHETIC_SECRET_VALUE").expect("key");
    let req = OpenAiAdapter
        .build_request(&request(), &key)
        .expect("request");
    assert_eq!(format!("{key:?}"), "ApiKey([REDACTED])");
    assert!(!format!("{req:?}").contains("SYNTHETIC_SECRET_VALUE"));
    assert_eq!(req.url, "https://api.openai.com/v1/responses");
}

#[test]
fn provider_fixtures_normalize_text_and_usage() {
    let openai=br#"data: {"type":"response.output_text.delta","delta":"ok"}
data: {"type":"response.completed","response":{"usage":{"input_tokens":10,"output_tokens":2,"input_tokens_details":{"cached_tokens":3}}}}
data: [DONE]
"#;
    let anthropic = br#"data: {"type":"message_start","message":{"usage":{"input_tokens":10,"cache_read_input_tokens":3,"output_tokens":0}}}
data: {"type":"content_block_delta","delta":{"text":"ok"}}
data: {"type":"message_delta","usage":{"output_tokens":2}}
data: {"type":"message_stop"}
"#;
    let gemini = br#"data: {"candidates":[{"content":{"parts":[{"text":"ok"}]}}],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":2,"thoughtsTokenCount":1}}
"#;
    for result in [
        OpenAiAdapter.parse_stream(openai),
        AnthropicAdapter.parse_stream(anthropic),
        GeminiAdapter.parse_stream(gemini),
    ] {
        let events = result.expect("fixture");
        assert!(
            events
                .iter()
                .any(|e| matches!(e,StreamEvent::Text(v) if v=="ok"))
        );
        assert!(events.iter().any(|e| matches!(e, StreamEvent::Usage(_))));
    }
    let anthropic_usage = AnthropicAdapter
        .parse_stream(anthropic)
        .unwrap()
        .into_iter()
        .find_map(|event| {
            if let StreamEvent::Usage(usage) = event {
                Some(usage)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(anthropic_usage.input_tokens, 10);
    assert_eq!(anthropic_usage.cached_input_tokens, 3);
    assert_eq!(anthropic_usage.output_tokens, 2);
    let gemini_events = GeminiAdapter.parse_stream(gemini).unwrap();
    assert!(matches!(
        gemini_events.as_slice(),
        [StreamEvent::Text(_), StreamEvent::Usage(_)]
    ));
    let openai_usage = OpenAiAdapter
        .parse_stream(openai)
        .unwrap()
        .into_iter()
        .find_map(|event| {
            if let StreamEvent::Usage(usage) = event {
                Some(usage)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(openai_usage.input_tokens, 7);
    assert_eq!(openai_usage.cached_input_tokens, 3);
    let gemini_cached = br#"data: {"usageMetadata":{"promptTokenCount":10,"cachedContentTokenCount":3,"candidatesTokenCount":2}}"#;
    let gemini_usage = GeminiAdapter
        .parse_stream(gemini_cached)
        .unwrap()
        .into_iter()
        .find_map(|event| {
            if let StreamEvent::Usage(usage) = event {
                Some(usage)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(gemini_usage.input_tokens, 7);
    assert_eq!(gemini_usage.cached_input_tokens, 3);
}

#[test]
fn openai_reasoning_breakdown_is_not_billed_twice() {
    let raw = br#"data: {"type":"response.completed","response":{"usage":{"input_tokens":10,"output_tokens":10,"output_tokens_details":{"reasoning_tokens":5}}}}"#;
    let usage = OpenAiAdapter
        .parse_stream(raw)
        .unwrap()
        .into_iter()
        .find_map(|event| {
            if let StreamEvent::Usage(value) = event {
                Some(value)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(usage.output_tokens, 10);
    assert_eq!(usage.reasoning_tokens, 5);
    assert_eq!(estimate_cost(&entry(), usage).unwrap(), 140);
    assert_eq!(
        estimate_cost(
            &entry(),
            Usage {
                output_tokens: 1,
                reasoning_tokens: 2,
                ..Usage::default()
            }
        ),
        Err(AiError::InvalidResponse),
    );
}

#[test]
fn signed_catalog_rejects_rollback() {
    let catalog = Catalog {
        format_version: 1,
        catalog_id: "2026-09-13.1".into(),
        issued_at: "2026-09-13T00:00:00Z".into(),
        expires_at: "2027-01-01T00:00:00Z".into(),
        minimum_app_version: "0.0.0-dev".into(),
        entries: vec![entry()],
    };
    let bytes = serde_json::to_vec(&catalog).expect("json");
    let signing = SigningKey::from_bytes(&[7; 32]);
    let sig = base64::engine::general_purpose::STANDARD.encode(signing.sign(&bytes).to_bytes());
    let verified = Catalog::verify(
        &bytes,
        &sig,
        &signing.verifying_key().to_bytes(),
        "2026-09-13T12:00:00Z",
        "0.0.0-dev",
        None,
    )
    .expect("catalog");
    assert_eq!(
        verified
            .resolve(
                Provider::OpenAi,
                Preset::Balanced,
                OperationType::TailorResume
            )
            .expect("entry")
            .model,
        "synthetic-model"
    );
    assert_eq!(
        Catalog::verify(
            &bytes,
            &sig,
            &signing.verifying_key().to_bytes(),
            "2026-09-13T12:00:00Z",
            "0.0.0-dev",
            Some("2026-09-13.1")
        ),
        Err(AiError::InvalidCatalog)
    );
}

#[test]
fn signed_catalog_rejects_missing_duplicate_or_zero_pricing_components() {
    let signing = SigningKey::from_bytes(&[7; 32]);
    for prices in [
        Vec::new(),
        vec![
            Price {
                category: PriceCategory::Input,
                micros_per_million: 1,
            },
            Price {
                category: PriceCategory::Input,
                micros_per_million: 2,
            },
        ],
        vec![Price {
            category: PriceCategory::Input,
            micros_per_million: 0,
        }],
    ] {
        let mut invalid_entry = entry();
        invalid_entry.prices = prices;
        let catalog = Catalog {
            format_version: 1,
            catalog_id: "2026-09-13.1".into(),
            issued_at: "2026-09-13T00:00:00Z".into(),
            expires_at: "2027-01-01T00:00:00Z".into(),
            minimum_app_version: "0.0.0-dev".into(),
            entries: vec![invalid_entry],
        };
        let bytes = serde_json::to_vec(&catalog).expect("json");
        let signature =
            base64::engine::general_purpose::STANDARD.encode(signing.sign(&bytes).to_bytes());
        assert_eq!(
            Catalog::verify(
                &bytes,
                &signature,
                &signing.verifying_key().to_bytes(),
                "2026-09-13T12:00:00Z",
                "0.0.0-dev",
                None,
            ),
            Err(AiError::InvalidCatalog)
        );
    }
}

#[test]
fn bundled_catalog_has_a_valid_independent_signature() {
    let catalog = builtin_catalog("2026-09-15T12:00:00Z", None).expect("bundled catalog");
    assert_eq!(catalog.catalog_id, "2026-09-15.1");
    assert_eq!(catalog.entries.len(), 3);
}

#[test]
fn cost_math_rounds_up_and_missing_dimensions_fail_closed() {
    assert_eq!(
        estimate_cost(
            &entry(),
            Usage {
                input_tokens: 10,
                output_tokens: 2,
                ..Usage::default()
            }
        ),
        Ok(44)
    );
    let mut gemini_entry = entry();
    gemini_entry.provider = Provider::Gemini;
    assert_eq!(
        estimate_cost(
            &gemini_entry,
            Usage {
                reasoning_tokens: 1,
                ..Usage::default()
            }
        ),
        Err(AiError::CostUnavailable)
    );
}

#[test]
fn caps_are_atomic_and_unknown_outcomes_remain_counted() {
    let ledger = AccountingLedger::new();
    let credential = Uuid::now_v7();
    ledger
        .set_caps(
            credential,
            vec![Cap {
                period: CapPeriod::Month,
                limit_micros: 100,
                starts_at_unix: 0,
                ends_at_unix: Some(1_000),
            }],
        )
        .expect("caps");
    let operation = ledger
        .begin_operation(OperationType::TailorResume, 10)
        .expect("operation");
    let attempt = ledger
        .reserve(
            operation,
            Provider::OpenAi,
            credential,
            "model",
            "catalog",
            "USD",
            60,
            10,
            None,
        )
        .expect("reserve");
    assert_eq!(
        ledger.reserve(
            operation,
            Provider::OpenAi,
            credential,
            "model",
            "catalog",
            "USD",
            41,
            10,
            None
        ),
        Err(AiError::CapExceeded)
    );
    ledger
        .transition(attempt, AttemptStatus::Dispatching)
        .expect("dispatch");
    ledger
        .settle(
            attempt,
            AttemptStatus::OutcomeUnknown,
            None,
            None,
            None,
            false,
            Some(ErrorCategory::Timeout),
            20,
        )
        .expect("unknown");
    assert_eq!(
        ledger.reserve(
            operation,
            Provider::OpenAi,
            credential,
            "model",
            "catalog",
            "USD",
            41,
            21,
            None
        ),
        Err(AiError::CapExceeded)
    );
}

#[test]
fn monitoring_export_is_aggregate_and_activity_clear_preserves_cap_count() {
    let ledger = AccountingLedger::new();
    let credential = Uuid::now_v7();
    let operation = ledger
        .begin_operation(OperationType::TailorResume, 10)
        .expect("operation");
    let attempt = ledger
        .reserve(
            operation,
            Provider::Anthropic,
            credential,
            "model",
            "catalog",
            "USD",
            100,
            10,
            None,
        )
        .expect("reserve");
    ledger
        .transition(attempt, AttemptStatus::Dispatching)
        .expect("dispatch");
    ledger
        .settle(
            attempt,
            AttemptStatus::Succeeded,
            Some("model"),
            Some(Usage {
                input_tokens: 5,
                output_tokens: 2,
                ..Usage::default()
            }),
            Some(7),
            true,
            None,
            12,
        )
        .expect("settle");
    ledger.finish_operation(operation, 12).expect("finish");
    let summary = ledger.summary(0, 20).expect("summary");
    assert_eq!(
        (
            summary.logical_operations,
            summary.attempts,
            summary.estimated_micros,
            summary.partial
        ),
        (1, 1, 7, false)
    );
    let csv = ledger.export_csv(0, 20).expect("csv");
    assert!(!csv.contains("model"));
    assert!(!csv.contains("provider"));
    assert_eq!(ledger.clear_activity(0, 20), Ok(1));
    ledger
        .set_caps(
            credential,
            vec![Cap {
                period: CapPeriod::AllTime,
                limit_micros: 7,
                starts_at_unix: 0,
                ends_at_unix: None,
            }],
        )
        .expect("caps");
    let next = ledger
        .begin_operation(OperationType::AnswerQuestion, 21)
        .expect("operation");
    assert_eq!(
        ledger.reserve(
            next,
            Provider::Anthropic,
            credential,
            "model",
            "catalog",
            "USD",
            1,
            21,
            None
        ),
        Err(AiError::CapExceeded)
    );
}

#[test]
fn cancellation_and_crash_recovery_are_explicit() {
    let ledger = AccountingLedger::new();
    let operation = ledger
        .begin_operation(OperationType::ImportMapping, 10)
        .expect("operation");
    let attempt = ledger
        .reserve(
            operation,
            Provider::Gemini,
            Uuid::now_v7(),
            "model",
            "catalog",
            "USD",
            20,
            10,
            None,
        )
        .expect("reserve");
    ledger
        .transition(attempt, AttemptStatus::Dispatching)
        .expect("dispatch");
    ledger.cancel(operation).expect("cancel");
    assert_eq!(ledger.recover_interrupted(20), Ok(1));
    assert!(ledger.summary(0, 30).expect("summary").partial);
}
