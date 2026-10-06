use super::*;

#[test]
fn material_failures_distinguish_completion_model_and_format() {
    let mut output = SyntheticStreamState::default();
    assert_eq!(
        output.material_error("gemini-3.6-flash"),
        Some("AI_OUTPUT_INCOMPLETE")
    );
    output.finished = true;
    assert_eq!(
        output.material_error("gemini-3.6-flash"),
        Some("AI_MODEL_MISMATCH")
    );
    output.effective_model = Some("gemini-3.6-flash".into());
    assert_eq!(output.material_error("gemini-3.6-flash"), None);
    output.failed = true;
    assert_eq!(
        output.material_error("gemini-3.6-flash"),
        Some("AI_OUTPUT_INVALID")
    );
}

#[test]
fn material_cap_reservations_include_both_provider_schemas() {
    for provider in [Provider::OpenAi, Provider::Gemini] {
        for operation in [OperationType::TailorResume, OperationType::RefineResume] {
            let bytes = material_schema_bytes(provider, operation).unwrap();
            assert!(bytes > 1000);
            let key = ApiKey::new(b"SYNTHETIC_SECRET_VALUE").unwrap();
            let request = NormalizedRequest {
                operation,
                model: "gemini-3.6-flash".into(),
                system: "JSON".into(),
                input: json!({}),
                max_output_tokens: 6000,
            };
            let built = adapter(provider).build_request(&request, &key).unwrap();
            let body: Value = serde_json::from_slice(&built.body).unwrap();
            let schema = match provider {
                Provider::OpenAi => &body["text"]["format"]["schema"],
                Provider::Gemini => &body["generationConfig"]["responseFormat"]["text"]["schema"],
                Provider::Anthropic => unreachable!(),
            };
            assert_eq!(bytes, serde_json::to_vec(schema).unwrap().len());
        }
        assert_eq!(
            material_schema_bytes(provider, OperationType::CoverLetter).unwrap(),
            0
        );
    }
}

fn resume_stream_fixture(provider: Provider, text: &str, split: usize) -> String {
    use std::fmt::Write;
    let (first, last) = text.split_at(split);
    let values = match provider {
        Provider::OpenAi => vec![
            json!({"type":"response.created","response":{"model":"fixture-model"}}),
            json!({"type":"response.output_text.delta","delta":first}),
            json!({"type":"response.output_text.delta","delta":last}),
            json!({"type":"response.completed","response":{"usage":{"input_tokens":20,"output_tokens":100}}}),
        ],
        Provider::Anthropic => vec![
            json!({"type":"message_start","message":{"model":"fixture-model","usage":{"input_tokens":20,"output_tokens":0}}}),
            json!({"type":"content_block_delta","delta":{"text":first}}),
            json!({"type":"content_block_delta","delta":{"text":last}}),
            json!({"type":"message_delta","usage":{"output_tokens":100}}),
            json!({"type":"message_stop"}),
        ],
        Provider::Gemini => vec![
            json!({"modelVersion":"gemini-3.6-flash","candidates":[{"content":{"parts":[{"thought":true,"text":"private reasoning"},{"text":first}]}}]}),
            json!({"modelVersion":"gemini-3.6-flash","candidates":[{"content":{"parts":[{"text":last}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":20,"candidatesTokenCount":100,"thoughtsTokenCount":10}}),
        ],
    };
    values.iter().fold(String::new(), |mut raw, value| {
        write!(raw, "data: {value}\n\n").unwrap();
        raw
    })
}

#[test]
fn all_provider_streams_apply_header_free_resume_consistently() {
    use ort_domain::{Bullet, EntityId, NamedField, ResumeDocument, ResumeEntry, ResumeSection};
    let mut source = ResumeDocument::empty("Master");
    source.contact.full_name = "Alex Rivera".into();
    let section_id = EntityId::new();
    let entry_id = EntityId::new();
    source.sections.push(ResumeSection {
        id: section_id,
        order: 0,
        heading: "Experience".into(),
        entries: vec![ResumeEntry {
            id: entry_id,
            order: 0,
            heading: "North Co".into(),
            subheading: "Engineer".into(),
            date_range: "2021 - 2024".into(),
            dates: None,
            location: "Boston".into(),
            fields: vec![NamedField {
                id: EntityId::new(),
                order: 0,
                label: "Technologies".into(),
                value: "Rust, SQL".into(),
                is_skill: true,
            }],
            bullets: vec![Bullet {
                id: EntityId::new(),
                order: 0,
                text: "Built Rust tools for support teams.".into(),
            }],
            links: vec![],
        }],
    });
    let text = json!({"schemaVersion":5,"tailoringPlan":["Emphasize Rust support tools for the tooling role."],"roleInfo":null,"alerts":[],
        "templateSections":[{"sectionId":section_id,"entries":[{"entryId":entry_id,"sourceEntryIds":[entry_id],"mainInfo":{"format":"bullets","items":["Developed Rust tools for support teams."]}}]}]}).to_string();
    let mut expected = None;
    for provider in [Provider::OpenAi, Provider::Anthropic, Provider::Gemini] {
        let model = if provider == Provider::Gemini {
            "gemini-3.6-flash"
        } else {
            "fixture-model"
        };
        for split in [1, text.len() / 2, text.len() - 1] {
            let raw = resume_stream_fixture(provider, &text, split);
            let output = SyntheticStreamState::from_events(
                adapter(provider).parse_stream(raw.as_bytes()).unwrap(),
            );
            assert_eq!(output.material_error(model), None);
            assert_eq!(output.text, text);
            let material = ort_ai::materials::validate_template_tailoring(
                &source,
                "Rust tooling role",
                &output.text,
                1,
            )
            .unwrap();
            assert_eq!(
                material.resume.sections[0].entries[0].fields,
                source.sections[0].entries[0].fields
            );
            assert_eq!(
                material.resume.sections[0].entries[0].date_range,
                "2021 - 2024"
            );
            let context = ort_ai::materials::resume_context(&material.resume);
            if let Some(expected) = &expected {
                assert_eq!(&context, expected);
            } else {
                expected = Some(context);
            }
        }
    }
}

#[test]
fn provider_failures_preserve_retryable_server_status() {
    assert_eq!(
        provider_failure(reqwest::StatusCode::SERVICE_UNAVAILABLE),
        ("AI_PROVIDER_SERVICE_UNAVAILABLE", true)
    );
    assert_eq!(
        provider_failure(reqwest::StatusCode::INTERNAL_SERVER_ERROR),
        ("AI_PROVIDER_TEMPORARY", true)
    );
    assert_eq!(
        provider_failure(reqwest::StatusCode::BAD_REQUEST),
        ("AI_PROVIDER_BAD_REQUEST", false)
    );
    assert_eq!(
        provider_failure(reqwest::StatusCode::NOT_FOUND),
        ("AI_MODEL_UNAVAILABLE", false)
    );
    assert_eq!(
        provider_failure(reqwest::StatusCode::UNAUTHORIZED),
        ("AI_AUTHENTICATION_FAILED", false)
    );
}

#[test]
fn request_gate_can_cancel_and_reuse_without_cross_request_signal() {
    let gate = AiRequestGate::default();
    let first = gate.begin(Uuid::now_v7()).unwrap();
    assert!(gate.begin(Uuid::now_v7()).is_none());
    assert!(gate.cancel());
    assert!(first.signal.is_cancelled());
    drop(first);
    let second = gate.begin(Uuid::now_v7()).unwrap();
    assert!(!second.signal.is_cancelled());
}

#[test]
fn provider_url_and_secret_headers_are_pinned_and_redacted() {
    let key = ApiKey::new(b"SYNTHETIC-TEST-SECRET").unwrap();
    let adapter = OpenAiAdapter;
    let request = NormalizedRequest {
        operation: OperationType::CredentialTest,
        model: "fixture-model".into(),
        system: "Return JSON".into(),
        input: json!({"fixture":true}),
        max_output_tokens: 10,
    };
    let mut built = adapter.build_request(&request, &key).unwrap();
    assert!(pinned_url(Provider::OpenAi, "fixture-model", &built).is_some());
    let client = http_client_with_timeout(Duration::from_secs(60)).unwrap();
    let outbound = outbound(
        &client,
        pinned_url(Provider::OpenAi, "fixture-model", &built).unwrap(),
        &built,
    )
    .unwrap()
    .build()
    .unwrap();
    assert!(
        outbound
            .headers()
            .get("authorization")
            .unwrap()
            .is_sensitive()
    );
    assert!(!format!("{outbound:?}").contains("SYNTHETIC-TEST-SECRET"));
    built.url = "https://api.openai.com.attacker.invalid/v1/responses".into();
    assert!(pinned_url(Provider::OpenAi, "fixture-model", &built).is_none());
    built.url = "https://api.openai.com/v1/responses?redirect=https://attacker.invalid".into();
    assert!(pinned_url(Provider::OpenAi, "fixture-model", &built).is_none());
}

#[test]
fn bundled_balanced_test_has_a_nonzero_conservative_reservation() {
    let catalog = builtin_catalog("2026-09-23T00:00:00Z", None).unwrap();
    assert_eq!(test_output_limit(Provider::Gemini), 512);
    assert_eq!(test_output_limit(Provider::OpenAi), 64);
    for provider in [Provider::OpenAi, Provider::Anthropic, Provider::Gemini] {
        let entry = catalog
            .resolve(provider, Preset::Balanced, OperationType::CredentialTest)
            .unwrap();
        let reserved = maximum_cost(entry, provider).unwrap();
        assert!(reserved > 0);
        for price in &entry.prices {
            let mut usage = Usage::default();
            match price.category {
                ort_ai::PriceCategory::Input => {
                    usage.input_tokens = u64::from(TEST_INPUT_RESERVATION_BOUND);
                }
                ort_ai::PriceCategory::CachedInput => {
                    usage.cached_input_tokens = u64::from(TEST_INPUT_RESERVATION_BOUND);
                }
                ort_ai::PriceCategory::CacheWrite => {
                    usage.cache_write_tokens = u64::from(TEST_INPUT_RESERVATION_BOUND);
                }
                ort_ai::PriceCategory::Output => {
                    usage.output_tokens = u64::from(test_output_limit(provider));
                }
                ort_ai::PriceCategory::Reasoning => {
                    usage.reasoning_tokens = u64::from(test_output_limit(provider));
                }
            }
            assert!(reserved >= estimate_cost(entry, usage).unwrap());
        }
    }
}

#[test]
fn synthetic_stream_requires_exact_serving_model_json_and_terminal_event() {
    let raw = br#"data: {"type":"response.created","response":{"model":"fixture-model"}}
data: {"type":"response.output_text.delta","delta":"{\"ok\":true}"}
data: {"type":"response.completed","response":{"usage":{"input_tokens":10,"output_tokens":3}}}
"#;
    let parsed = SyntheticStreamState::from_events(OpenAiAdapter.parse_stream(raw).unwrap());
    assert!(parsed.valid("fixture-model"));
    assert_eq!(parsed.usage.unwrap().output_tokens, 3);
    assert!(!parsed.valid("other-model"));
    let incomplete = SyntheticStreamState::from_events(vec![
        StreamEvent::Model("fixture-model".into()),
        StreamEvent::Text("{\"ok\":true}".into()),
    ]);
    assert!(!incomplete.valid("fixture-model"));
    let failed = SyntheticStreamState::from_events(vec![
        StreamEvent::Model("fixture-model".into()),
        StreamEvent::Text("{\"ok\":true}".into()),
        StreamEvent::ProviderFailure,
        StreamEvent::Finished,
    ]);
    assert!(!failed.valid("fixture-model"));

    let gemini = br#"data: {"candidates":[{"content":{"parts":[{"text":"{\"ok\":true}"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":20,"candidatesTokenCount":6,"thoughtsTokenCount":17},"modelVersion":"gemini-3.6-flash"}
"#;
    let parsed = SyntheticStreamState::from_events(GeminiAdapter.parse_stream(gemini).unwrap());
    assert!(parsed.valid("gemini-3.6-flash"));
    assert_eq!(parsed.usage.unwrap().reasoning_tokens, 17);
    let incomplete = std::str::from_utf8(gemini)
        .unwrap()
        .replace(",\"finishReason\":\"STOP\"", "");
    let parsed = SyntheticStreamState::from_events(
        GeminiAdapter.parse_stream(incomplete.as_bytes()).unwrap(),
    );
    assert!(!parsed.valid("gemini-3.6-flash"));
    let blocked = std::str::from_utf8(gemini)
        .unwrap()
        .replace("STOP", "SAFETY");
    let parsed =
        SyntheticStreamState::from_events(GeminiAdapter.parse_stream(blocked.as_bytes()).unwrap());
    assert!(!parsed.valid("gemini-3.6-flash"));
}
