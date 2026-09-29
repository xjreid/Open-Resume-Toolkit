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
fn openai_resume_requests_use_the_strict_resume_draft_schema() {
    let key = ApiKey::new(b"SYNTHETIC_SECRET_VALUE").expect("key");
    for operation in [OperationType::TailorResume, OperationType::RefineResume] {
        let mut normalized = request();
        normalized.operation = operation;
        let built = OpenAiAdapter
            .build_request(&normalized, &key)
            .expect("request");
        let body: Value = serde_json::from_slice(&built.body).expect("json body");
        let format = body.pointer("/text/format").expect("format");
        assert_eq!(format["type"], "json_schema");
        assert_eq!(format["name"], "resume_draft");
        assert_eq!(format["strict"], true);
        assert_eq!(format["schema"], materials::resume_output_schema());
        // Structured outputs follow schema property order. The plan must be
        // generated before the resume even when maps are sorted by serde.
        let wire = std::str::from_utf8(&built.body).unwrap();
        assert!(
            wire.find("\"tailoringPlan\":{").unwrap()
                < wire.find("\"templateSections\":{").unwrap()
        );
    }
}

#[test]
fn gemini_requests_match_the_published_rest_generation_contract() {
    // Independently downloaded API discovery schemas; do not derive the
    // accepted values from the adapter or its expected-payload assertions.
    let discovery: Value = serde_json::from_str(include_str!(
        "fixtures/gemini-v1beta-generation-config.json"
    ))
    .unwrap();
    let schemas = &discovery["schemas"];
    let key = ApiKey::new(b"SYNTHETIC_SECRET_VALUE").unwrap();
    for model in ["gemini-3.6-flash", "gemini-3.5-flash-lite"] {
        for operation in [
            OperationType::TailorResume,
            OperationType::RefineResume,
            OperationType::CredentialTest,
            OperationType::CoverLetter,
            OperationType::AnswerQuestion,
        ] {
            let mut normalized = request();
            normalized.model = model.into();
            normalized.operation = operation;
            let built = GeminiAdapter.build_request(&normalized, &key).unwrap();
            let body: Value = serde_json::from_slice(&built.body).unwrap();
            assert_discovery_contract(
                &body["generationConfig"],
                &schemas["GenerationConfig"],
                schemas,
                &format!("{model}/{operation:?}/generationConfig"),
            );
        }
    }
}

fn assert_discovery_contract(value: &Value, schema: &Value, schemas: &Value, path: &str) {
    if let Some(reference) = schema["$ref"].as_str() {
        let referenced = &schemas[reference];
        assert!(
            !referenced.is_null(),
            "missing discovery schema {reference}"
        );
        assert_discovery_contract(value, referenced, schemas, path);
        return;
    }
    if let Some(allowed) = schema["enum"].as_array() {
        assert!(
            allowed.contains(value),
            "{path}: invalid REST enum {value}; expected {allowed:?}"
        );
    }
    match schema["type"].as_str().unwrap() {
        "any" => (),
        "object" => {
            for (name, field) in value.as_object().expect("REST object") {
                let property = &schema["properties"][name];
                assert!(!property.is_null(), "{path}: unknown REST field {name}");
                assert_discovery_contract(field, property, schemas, &format!("{path}/{name}"));
            }
        }
        "array" => {
            for item in value.as_array().expect("REST array") {
                assert_discovery_contract(item, &schema["items"], schemas, path);
            }
        }
        "string" => assert!(value.is_string(), "{path}: expected string"),
        "integer" => assert!(value.is_i64() || value.is_u64(), "{path}: expected integer"),
        "number" => assert!(value.is_number(), "{path}: expected number"),
        "boolean" => assert!(value.is_boolean(), "{path}: expected boolean"),
        other => panic!("unhandled discovery type {other} at {path}"),
    }
}

#[test]
fn gemini_36_resume_requests_use_body_only_schema_and_low_thinking() {
    let key = ApiKey::new(b"SYNTHETIC_SECRET_VALUE").unwrap();
    let catalog = builtin_catalog("2026-09-29T00:00:00Z", None).unwrap();
    for operation in [OperationType::TailorResume, OperationType::RefineResume] {
        let entry = catalog
            .resolve(Provider::Gemini, Preset::Balanced, operation)
            .unwrap();
        assert_eq!(entry.model, "gemini-3.6-flash");
        let mut normalized = request();
        normalized.operation = operation;
        normalized.model = entry.model.clone();
        normalized.max_output_tokens = entry.max_output_tokens;
        let built = GeminiAdapter.build_request(&normalized, &key).unwrap();
        assert_eq!(
            built.url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.6-flash:streamGenerateContent?alt=sse"
        );
        let body: Value = serde_json::from_slice(&built.body).unwrap();
        let config = &body["generationConfig"];
        assert_eq!(config["maxOutputTokens"], 6000);
        assert_eq!(config["thinkingConfig"]["thinkingLevel"], "LOW");
        assert_eq!(
            config["responseFormat"]["text"]["mimeType"],
            "APPLICATION_JSON"
        );
        assert_eq!(
            config["responseFormat"]["text"]["schema"],
            materials::gemini_resume_output_schema()
        );
        assert!(config.get("responseMimeType").is_none());
        let schema = config["responseFormat"]["text"]["schema"].to_string();
        for field in [
            "title", "role", "details", "date", "location", "extra", "heading",
        ] {
            let properties = &config["responseFormat"]["text"]["schema"]["properties"]["templateSections"]
                ["items"]["properties"];
            assert!(properties.get(field).is_none());
            assert!(
                properties["entries"]["items"]["properties"]
                    .get(field)
                    .is_none()
            );
        }
        assert!(!schema.contains("minLength"));
        assert!(!schema.contains("maxLength"));
        let wire = std::str::from_utf8(&built.body).unwrap();
        assert!(
            wire.find("\"tailoringPlan\":{").unwrap()
                < wire.find("\"templateSections\":{").unwrap()
        );
    }
    let mut other = request();
    other.operation = OperationType::CoverLetter;
    other.model = "gemini-3.6-flash".into();
    let built = GeminiAdapter.build_request(&other, &key).unwrap();
    let body: Value = serde_json::from_slice(&built.body).unwrap();
    assert_eq!(
        body["generationConfig"]["responseMimeType"],
        "application/json"
    );
    assert!(body["generationConfig"].get("responseFormat").is_none());
    assert!(body["generationConfig"].get("thinkingConfig").is_none());
}

#[test]
fn openai_non_resume_requests_keep_json_object_format() {
    let key = ApiKey::new(b"SYNTHETIC_SECRET_VALUE").expect("key");
    let mut normalized = request();
    normalized.operation = OperationType::CoverLetter;
    let built = OpenAiAdapter
        .build_request(&normalized, &key)
        .expect("request");
    let body: Value = serde_json::from_slice(&built.body).expect("json body");
    assert_eq!(
        body.pointer("/text/format/type"),
        Some(&json!("json_object"))
    );
    assert!(body.pointer("/text/format/schema").is_none());
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
    let gemini = br#"data: {"candidates":[{"content":{"parts":[{"text":"ok"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":2,"thoughtsTokenCount":1}}
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
        [
            StreamEvent::Text(_),
            StreamEvent::Usage(_),
            StreamEvent::Finished
        ]
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
fn gemini_credential_test_limits_thinking_and_reads_visible_parts() {
    let key = ApiKey::new(b"SYNTHETIC_SECRET_VALUE").unwrap();
    let mut test = request();
    test.operation = OperationType::CredentialTest;
    test.model = "gemini-3.6-flash".into();
    test.max_output_tokens = 512;
    let built = GeminiAdapter.build_request(&test, &key).unwrap();
    let body: Value = serde_json::from_slice(&built.body).unwrap();
    assert_eq!(body["generationConfig"]["maxOutputTokens"], 512);
    assert_eq!(
        body["generationConfig"]["thinkingConfig"]["thinkingLevel"],
        "LOW"
    );

    let response = br#"data: {"candidates":[{"content":{"parts":[{"thought":true,"text":"hidden reasoning"},{"text":"{\"ok\":true}"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":20,"thoughtsTokenCount":12},"modelVersion":"gemini-3.6-flash"}
"#;
    let events = GeminiAdapter.parse_stream(response).unwrap();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, StreamEvent::Text(content) if content == "{\"ok\":true}"))
    );
    assert!(!events.iter().any(
        |event| matches!(event, StreamEvent::Text(content) if content.contains("hidden reasoning"))
    ));
    assert!(events.iter().any(|event| matches!(event, StreamEvent::Usage(usage) if usage.output_tokens == 0 && usage.reasoning_tokens == 12)));
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
    let catalog = builtin_catalog("2026-09-23T12:00:00Z", None).expect("bundled catalog");
    assert_eq!(catalog.catalog_id, "2026-09-23.1");
    assert_eq!(catalog.entries.len(), 4);
    let economy = catalog
        .resolve(
            Provider::Gemini,
            Preset::Economy,
            OperationType::CredentialTest,
        )
        .expect("Gemini Economy entry");
    assert_eq!(economy.model, "gemini-3.5-flash-lite");
    assert_eq!(
        economy
            .prices
            .iter()
            .find(|price| price.category == PriceCategory::Input)
            .unwrap()
            .micros_per_million,
        300_000
    );
    assert!(
        catalog
            .resolve(
                Provider::OpenAi,
                Preset::Economy,
                OperationType::CredentialTest
            )
            .is_err()
    );
    assert!(
        catalog
            .resolve(
                Provider::Anthropic,
                Preset::Economy,
                OperationType::CredentialTest
            )
            .is_err()
    );
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

#[test]
fn gemini_requires_successful_candidate_completion() {
    for reason in [
        None,
        Some("MAX_TOKENS"),
        Some("SAFETY"),
        Some("RECITATION"),
        Some("OTHER"),
        Some("STOP"),
    ] {
        let mut value = json!({"candidates":[{"content":{"parts":[{"text":"{\"ok\":true}"}]}}],
            "modelVersion":"gemini-3.6-flash", "usageMetadata":{"promptTokenCount":20,"candidatesTokenCount":6}});
        if let Some(reason) = reason {
            value["candidates"][0]["finishReason"] = json!(reason);
        }
        // Even a generic sentinel must not make an incomplete candidate successful.
        let stream = format!("data: {value}\ndata: [DONE]\n");
        let events = GeminiAdapter.parse_stream(stream.as_bytes()).unwrap();
        assert_eq!(
            events.contains(&StreamEvent::ProviderFailure),
            reason != Some("STOP")
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, StreamEvent::Usage(_)))
        );
    }
    for value in [
        json!({"error":{"message":"failed"}}),
        json!({"promptFeedback":{"blockReason":"SAFETY"}}),
    ] {
        let events = GeminiAdapter
            .parse_stream(format!("data: {value}\n").as_bytes())
            .unwrap();
        assert!(events.contains(&StreamEvent::ProviderFailure));
    }
}
