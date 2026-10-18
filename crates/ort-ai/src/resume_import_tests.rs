use super::*;
use crate::{AnthropicAdapter, GeminiAdapter, OpenAiAdapter};

fn proposal() -> Value {
    json!({"contact":{"fullName":"Sample Person","email":"sample@example.com","phone":"","location":"Boston","links":[{"label":"Website","url":"https://example.com"}]},
        "sections":[{"heading":"Experience","entries":[{"title":"Example Company","role":"Engineer","details":"Platform team","date":"2022–Present","location":"Remote","extra":"Part time","mainInfo":{"format":"bullets","items":["Built a sample service."]}}]},
        {"heading":"Summary","entries":[{"title":"","role":"","details":"","date":"","location":"","extra":"","mainInfo":{"format":"paragraph","items":["First paragraph.","Second paragraph."]}}]}]})
}

#[test]
fn maps_all_seven_slots_and_generates_valid_native_identities() {
    let document = decode(&proposal().to_string()).unwrap();
    document.validate(DocumentLimits::default()).unwrap();
    assert_eq!(document.schema_version, 2);
    assert_eq!(document.contact.links[0].url, "https://example.com");
    let entry = &document.sections[0].entries[0];
    assert_eq!(
        (&*entry.heading, &*entry.subheading, &*entry.location),
        ("Example Company", "Engineer", "Remote")
    );
    assert_eq!(
        entry.field_text(ort_domain::FieldRole::Details, ""),
        "Platform team"
    );
    assert_eq!(
        entry.field_text(ort_domain::FieldRole::Extra, ""),
        "Part time"
    );
    assert_eq!(entry.bullets[0].text, "Built a sample service.");
    assert_eq!(
        entry.dates.as_ref().unwrap()[0].display_text(),
        "2022–Present"
    );
    assert!(entry.date_range.is_empty());
    assert_eq!(
        document.sections[1].entries[0].field_text(ort_domain::FieldRole::Paragraph, ""),
        "First paragraph.\nSecond paragraph."
    );
    assert_ne!(
        document.document_id,
        decode(&proposal().to_string()).unwrap().document_id
    );
}

#[test]
fn normalizes_visible_resume_web_addresses_without_changing_their_paths() {
    for (input, expected) in [
        (
            "linkedin.com/in/sample-person",
            "https://linkedin.com/in/sample-person",
        ),
        (
            "www.example.com/Portfolio?view=CV#Work",
            "https://www.example.com/Portfolio?view=CV#Work",
        ),
        ("  https://example.com/work  ", "https://example.com/work"),
        ("mailto:sample@example.com", "mailto:sample@example.com"),
        ("http://example.com", "http://example.com"),
    ] {
        let mut value = proposal();
        value["contact"]["links"][0]["url"] = json!(input);
        let document = decode(&value.to_string()).unwrap();
        assert_eq!(document.contact.links[0].url, expected);
        document.validate(DocumentLimits::default()).unwrap();
    }
}

#[test]
fn preserves_paragraph_items_that_exceed_one_native_field_together() {
    let mut value = proposal();
    let paragraphs = vec![
        "First paragraph. ".repeat(75),
        "Second paragraph. ".repeat(75),
    ];
    value["sections"][1]["entries"][0]["mainInfo"]["items"] = json!(paragraphs);
    let document = decode(&value.to_string()).unwrap();
    assert_eq!(
        document.sections[1].entries[0].field_text(ort_domain::FieldRole::Paragraph, "\n\n"),
        paragraphs.join("\n\n")
    );
    document.validate(DocumentLimits::default()).unwrap();
}

#[test]
fn accepts_a_single_json_fence_and_absent_optional_text_without_losing_content() {
    let mut value = proposal();
    value["contact"]["phone"] = Value::Null;
    value["sections"][0]["entries"][0]["extra"] = Value::Null;
    value["sections"][0]["entries"][0]
        .as_object_mut()
        .unwrap()
        .remove("location");
    let document = decode(&format!("\u{feff}```json\n{value}\n```\n")).unwrap();
    assert_eq!(document.contact.phone, "");
    let entry = &document.sections[0].entries[0];
    assert_eq!(entry.location, "");
    assert_eq!(entry.field_text(ort_domain::FieldRole::Extra, ""), "");
    assert_eq!(entry.bullets[0].text, "Built a sample service.");
}

#[test]
fn rejects_malformed_missing_extra_empty_and_unsafe_results_without_echoing_content() {
    for text in ["not JSON PRIVATE_CONTENT", "```json\n{}\n```", "{}"] {
        let error = decode(text).unwrap_err();
        assert!(!error.contains("PRIVATE_CONTENT"));
    }
    let mut value = proposal();
    value["sections"][0]["entries"][0]
        .as_object_mut()
        .unwrap()
        .remove("mainInfo");
    assert!(decode(&value.to_string()).is_err());
    let mut value = proposal();
    value["PRIVATE_EXTRA_FIELD"] = json!("PRIVATE_CONTENT");
    assert!(!decode(&value.to_string()).unwrap_err().contains("PRIVATE"));
    let mut value = proposal();
    value["contact"]["links"][0]["url"] = json!("javascript:alert(1)");
    assert!(decode(&value.to_string()).unwrap_err().contains("link"));
    let mut value = proposal();
    value["sections"] = json!([]);
    assert!(
        decode(&value.to_string())
            .unwrap_err()
            .contains("no readable")
    );
    let mut value = proposal();
    value["sections"][0]["entries"][0]["mainInfo"]["items"] = json!(["a".repeat(501)]);
    assert!(decode(&value.to_string()).unwrap_err().contains("limit"));
}

#[test]
fn sends_original_pdf_as_document_content_to_each_provider() {
    let pdf = b"%PDF-1.7\nSYNTHETIC_BINARY_CONTENT\n%%EOF";
    let key = ApiKey::new(b"synthetic-key").unwrap();
    let encoded = STANDARD.encode(pdf);
    let adapters: [&dyn ProviderAdapter; 3] = [&OpenAiAdapter, &AnthropicAdapter, &GeminiAdapter];
    for adapter in adapters {
        let mut request = NormalizedRequest {
            operation: OperationType::ImportMapping,
            model: "test-model".into(),
            system: SYSTEM.into(),
            input: input(),
            max_output_tokens: 4096,
        };
        let http = build_request(adapter, &request, &key, pdf).unwrap();
        let body: Value = serde_json::from_slice(&http.body).unwrap();
        let attachment = match adapter.provider() {
            Provider::OpenAi => {
                assert_eq!(body["text"]["format"]["schema"], output_schema());
                assert_eq!(body["store"], false);
                body["input"][0]["content"][0]["file_data"]
                    .as_str()
                    .unwrap()
                    .strip_prefix("data:application/pdf;base64,")
                    .unwrap()
            }
            Provider::Anthropic => {
                assert_eq!(body["output_config"]["format"]["type"], "json_schema");
                assert_eq!(
                    body["output_config"]["format"]["schema"],
                    provider_output_schema(Provider::Anthropic)
                );
                body["messages"][0]["content"][0]["source"]["data"]
                    .as_str()
                    .unwrap()
            }
            Provider::Gemini => {
                assert_eq!(
                    body["generationConfig"]["responseFormat"]["text"]["schema"],
                    provider_output_schema(Provider::Gemini)
                );
                assert_eq!(
                    body["generationConfig"]["responseFormat"]["text"]["mimeType"],
                    "APPLICATION_JSON"
                );
                assert!(body["generationConfig"].get("responseMimeType").is_none());
                body["contents"][0]["parts"][0]["inlineData"]["data"]
                    .as_str()
                    .unwrap()
            }
        };
        assert_eq!(attachment, encoded);
        let prompt = match adapter.provider() {
            Provider::OpenAi => &body["input"][0]["content"][1]["text"],
            Provider::Anthropic => &body["messages"][0]["content"][1]["text"],
            Provider::Gemini => &body["contents"][0]["parts"][1]["text"],
        };
        let mut expected_prompt = input();
        if adapter.provider() == Provider::Gemini {
            expected_prompt.as_object_mut().unwrap().remove("schema");
        }
        assert_eq!(
            serde_json::from_str::<Value>(prompt.as_str().unwrap()).unwrap(),
            expected_prompt
        );
        assert!(!format!("{http:?}").contains("synthetic-key"));
        request.operation = OperationType::TailorResume;
        assert!(build_request(adapter, &request, &key, pdf).is_err());
    }
}

#[test]
fn rejects_unsafe_ambiguous_and_missing_link_destinations_with_private_diagnostics() {
    for url in [
        "javascript:PRIVATE_CONTENT",
        "data:text/plain,PRIVATE_CONTENT",
        "file:///PRIVATE_CONTENT",
        "ftp://example.com/PRIVATE_CONTENT",
        "",
        "LinkedIn",
        "not a URL",
        "/in/PRIVATE_CONTENT",
        "[LinkedIn](https://example.com/PRIVATE_CONTENT)",
        "https://",
        "https://user:PRIVATE_CONTENT@example.com",
        "mailto:",
    ] {
        let mut value = proposal();
        value["contact"]["links"][0]["url"] = json!(url);
        let error = decode(&value.to_string()).unwrap_err();
        assert!(error.starts_with("contact.links[0].url"), "{url}: {error}");
        assert!(!error.contains("PRIVATE_CONTENT"));
    }
}

#[test]
fn refuses_broken_structure_and_over_limit_content_without_silently_repairing_it() {
    for change in [
        json!({"format":"bullets","items":[{"text":"PRIVATE_CONTENT"}]}),
        json!({"format":"bullet","items":["PRIVATE_CONTENT"]}),
        json!({"format":"bullets"}),
        json!(null),
    ] {
        let mut value = proposal();
        value["sections"][0]["entries"][0]["mainInfo"] = change;
        assert!(
            !decode(&value.to_string())
                .unwrap_err()
                .contains("PRIVATE_CONTENT")
        );
    }
    for (format, maximum) in [("bullets", 500), ("paragraph", 2000)] {
        let mut value = proposal();
        value["sections"][0]["entries"][0]["mainInfo"] =
            json!({"format":format,"items":["é".repeat(maximum)]});
        assert!(decode(&value.to_string()).is_ok());
        value["sections"][0]["entries"][0]["mainInfo"]["items"][0] = json!("é".repeat(maximum + 1));
        let error = decode(&value.to_string()).unwrap_err();
        assert!(error.starts_with("sections[0].entries[0].mainInfo.items[0]"));
        assert!(!error.contains('é'));
    }
    let value = proposal().to_string();
    for text in [
        format!("Here is the resume: {value}"),
        format!("{value}\n{value}"),
        value[..value.len() - 1].into(),
        format!("```json\n{value}"),
    ] {
        assert!(decode(&text).is_err());
    }
}

#[test]
fn schemas_are_strict_and_describe_native_limits_in_each_provider_subset() {
    fn check(value: &Value, provider: Provider) {
        if let Some(object) = value.as_object() {
            if value["type"] == "object" {
                assert_eq!(value["additionalProperties"], false);
                assert_eq!(
                    value["required"].as_array().unwrap().len(),
                    value["properties"].as_object().unwrap().len()
                );
            }
            if value["type"] == "string" && value.get("enum").is_none() {
                assert!(
                    value["description"]
                        .as_str()
                        .is_some_and(|s| s.contains("characters"))
                );
            }
            if provider != Provider::OpenAi {
                assert!(object.get("maxLength").is_none());
                assert!(object.get("minLength").is_none());
            }
            if provider == Provider::Anthropic {
                assert!(object.get("maxItems").is_none());
            }
            for child in object.values() {
                check(child, provider);
            }
        } else if let Some(array) = value.as_array() {
            for child in array {
                check(child, provider);
            }
        }
    }
    for provider in [Provider::OpenAi, Provider::Anthropic, Provider::Gemini] {
        check(&provider_output_schema(provider), provider);
    }
    let schema = output_schema();
    let entry = &schema["properties"]["sections"]["items"]["properties"]["entries"]["items"];
    assert_eq!(
        entry["properties"]["mainInfo"]["anyOf"][0]["properties"]["items"]["items"]["maxLength"],
        500
    );
    assert_eq!(
        entry["properties"]["mainInfo"]["anyOf"][1]["properties"]["items"]["items"]["maxLength"],
        2000
    );
    decode(&input()["formatExampleOnly"].to_string()).unwrap();
    assert!(SYSTEM.contains("linkedin.com/in/person becomes https://linkedin.com/in/person"));
}

#[test]
fn gemini_import_limits_reasoning_to_leave_room_for_resume_json() {
    let request = NormalizedRequest {
        operation: OperationType::ImportMapping,
        model: "gemini-3.5-flash-lite".into(),
        system: SYSTEM.into(),
        input: input(),
        max_output_tokens: 6000,
    };
    let key = ApiKey::new(b"synthetic-key").unwrap();
    let http = build_request(&GeminiAdapter, &request, &key, b"%PDF-1.7\n%%EOF").unwrap();
    let body: Value = serde_json::from_slice(&http.body).unwrap();
    assert_eq!(
        body["generationConfig"]["thinkingConfig"]["thinkingLevel"],
        "LOW"
    );
    assert_eq!(body["generationConfig"]["maxOutputTokens"], 6000);
}

#[test]
fn dates_override_position_without_moving_project_links_or_losing_right_hand_text() {
    let mut value = input()["formatExampleOnly"].clone();
    let entry = &mut value["sections"][1]["entries"][0];
    entry["leftSubheading"] = json!("[GitHub](https://github.com/example/project)\n2024–2025");
    entry["rightTop"] = json!("Boston");
    entry["rightMiddle"] = json!("");
    let document = decode(&value.to_string()).unwrap();
    let project = &document.sections[1].entries[0];
    assert_eq!(
        project.subheading,
        "[GitHub](https://github.com/example/project)"
    );
    assert_eq!(project.heading, "Example Project");
    assert_eq!(
        project.dates.as_ref().unwrap()[0].display_text(),
        "2024–2025"
    );
    assert_eq!(project.location, "Boston");
    assert_eq!(project.field_text(ort_domain::FieldRole::Extra, ""), "");
    assert_eq!(
        project.field_text(ort_domain::FieldRole::Details, ""),
        "Rust"
    );
    document.validate(DocumentLimits::default()).unwrap();
    let properties = &output_schema()["properties"]["sections"]["items"]["properties"]["entries"]["items"]
        ["properties"];
    assert!(properties.get("role").is_none());
    assert!(properties.get("extra").is_none());
    assert!(
        properties["leftSubheading"]["description"]
            .as_str()
            .unwrap()
            .contains("BELOW")
    );
}

#[test]
fn contact_section_links_move_to_the_top_without_moving_project_links() {
    let mut value = input()["formatExampleOnly"].clone();
    value["sections"].as_array_mut().unwrap().insert(0, json!({"heading":"Contact Information","entries":[
        {"leftHeading":"GitHub","leftSubheading":"https://github.com/example","mainInfo":{"format":"paragraph","items":[]}},
        {"mainInfo":{"format":"paragraph","items":["[LinkedIn](https://linkedin.com/in/example-person)", "Boston, MA"]}}
    ]}));
    let document = decode(&value.to_string()).unwrap();
    assert_eq!(
        document
            .sections
            .iter()
            .map(|s| s.heading.as_str())
            .collect::<Vec<_>>(),
        ["Experience", "Projects", "Summary"]
    );
    assert_eq!(document.contact.links.len(), 2);
    assert_eq!(document.contact.links[1].label, "GitHub");
    assert_eq!(document.contact.links[1].url, "https://github.com/example");
    assert_eq!(document.contact.location, "Boston, MA");
    assert_eq!(
        document.sections[1].entries[0].subheading,
        "[GitHub](https://github.com/example/project)"
    );
    document.validate(DocumentLimits::default()).unwrap();
}

#[test]
fn both_pdf_provider_requests_carry_exact_annotation_destinations_and_positions() {
    let links = vec![PdfHyperlink {
        page: 1,
        rect: [80, 210, 160, 230],
        url: "https://github.com/example/project?tab=readme#details".into(),
    }];
    assert!(valid_pdf_hyperlinks(&links, 1));
    for adapter in [
        &OpenAiAdapter as &dyn ProviderAdapter,
        &GeminiAdapter,
        &AnthropicAdapter,
    ] {
        let request = NormalizedRequest {
            operation: OperationType::ImportMapping,
            model: "test-model".into(),
            system: SYSTEM.into(),
            input: input_with_pdf_hyperlinks(&links),
            max_output_tokens: 4096,
        };
        let key = ApiKey::new(b"synthetic-key").unwrap();
        let http = build_request(adapter, &request, &key, b"%PDF-1.7\n%%EOF").unwrap();
        let body: Value = serde_json::from_slice(&http.body).unwrap();
        let text = match adapter.provider() {
            Provider::OpenAi => &body["input"][0]["content"][1]["text"],
            Provider::Gemini => &body["contents"][0]["parts"][1]["text"],
            Provider::Anthropic => &body["messages"][0]["content"][1]["text"],
        };
        let prompt: Value = serde_json::from_str(text.as_str().unwrap()).unwrap();
        assert_eq!(prompt["pdfHyperlinks"]["links"][0]["url"], links[0].url);
        assert_eq!(
            prompt["pdfHyperlinks"]["links"][0]["rect"],
            json!([80, 210, 160, 230])
        );
    }
}

#[test]
fn source_hyperlinks_reject_unsafe_destinations_bad_geometry_and_unbounded_metadata() {
    let valid = PdfHyperlink {
        page: 1,
        rect: [10, 20, 30, 40],
        url: "https://example.com".into(),
    };
    for invalid in [
        PdfHyperlink {
            url: "javascript:alert(1)".into(),
            ..valid.clone()
        },
        PdfHyperlink {
            page: 0,
            ..valid.clone()
        },
        PdfHyperlink {
            page: 3,
            ..valid.clone()
        },
        PdfHyperlink {
            rect: [50, 20, 30, 40],
            ..valid.clone()
        },
        PdfHyperlink {
            rect: [10, 20, 1001, 40],
            ..valid.clone()
        },
    ] {
        assert!(!valid_pdf_hyperlinks(&[invalid], 2));
    }
    assert!(!valid_pdf_hyperlinks(&vec![valid.clone(); 201], 2));
    assert!(!valid_pdf_hyperlinks(
        &vec![
            PdfHyperlink {
                url: format!("https://example.com/{}", "x".repeat(1000)),
                ..valid
            };
            100
        ],
        2
    ));
}

#[test]
fn gemini_pdf_schema_avoids_nested_union_and_array_limit_expansion() {
    fn simple(schema: &Value) {
        if let Some(object) = schema.as_object() {
            for keyword in [
                "anyOf",
                "oneOf",
                "maxItems",
                "minItems",
                "maxLength",
                "minLength",
            ] {
                assert!(
                    object.get(keyword).is_none(),
                    "Gemini import still contains {keyword}"
                );
            }
            for child in object.values() {
                simple(child);
            }
        } else if let Some(array) = schema.as_array() {
            for child in array {
                simple(child);
            }
        }
    }
    let schema = provider_output_schema(Provider::Gemini);
    simple(&schema);
    let body = &schema["properties"]["sections"]["items"]["properties"]["entries"]["items"]["properties"]
        ["mainInfo"];
    assert_eq!(body["type"], "object");
    assert_eq!(
        body["properties"]["format"]["enum"],
        json!(["bullets", "paragraph"])
    );
    assert_eq!(body["properties"]["items"]["items"]["type"], "string");
    let description = body["properties"]["items"]["items"]["description"]
        .as_str()
        .unwrap();
    assert!(description.contains("500") && description.contains("2000"));
    // The native contract still retains and enforces its separate body limits.
    assert!(output_schema()["properties"]["sections"]["maxItems"].is_number());
    let mut value = input()["formatExampleOnly"].clone();
    value["sections"][0]["entries"][0]["mainInfo"]["items"] = json!(["x".repeat(501)]);
    assert!(decode(&value.to_string()).is_err());
}

#[test]
fn import_body_lines_do_not_gain_blank_lines_and_explicit_paragraph_breaks_survive() {
    for heading in ["Skills", "Coursework"] {
        let mut value = proposal();
        value["sections"][1]["heading"] = json!(heading);
        value["sections"][1]["entries"][0]["mainInfo"]["items"] = json!([
            "Programming: Rust, Python",
            "Tools: Git, SQL",
            "First paragraph.\n\nSecond paragraph."
        ]);
        let document = decode(&value.to_string()).unwrap();
        assert_eq!(
            document.sections[1].entries[0].field_text(ort_domain::FieldRole::Paragraph, ""),
            "Programming: Rust, Python\nTools: Git, SQL\nFirst paragraph.\n\nSecond paragraph."
        );
    }
}

#[test]
fn dates_in_every_header_slot_and_standalone_body_lines_use_the_date_box() {
    for slot in [
        "leftHeading",
        "leftSubheading",
        "headingSuffix",
        "rightMiddle",
        "rightBottom",
    ] {
        let mut value = input()["formatExampleOnly"].clone();
        let entry = &mut value["sections"][1]["entries"][0];
        entry["rightTop"] = json!("");
        entry[slot] = json!("Expected May 2027");
        let document = decode(&value.to_string()).unwrap();
        let entry = &document.sections[1].entries[0];
        assert_eq!(
            entry.dates.as_ref().unwrap()[0].display_text(),
            "May. 2027",
            "{slot}"
        );
        assert!(entry.date_range.is_empty());
        assert_eq!(
            entry.field_text(ort_domain::FieldRole::Extra, ""),
            "Expected"
        );
        assert!(
            ![
                entry.heading.as_str(),
                entry.subheading.as_str(),
                entry.location.as_str()
            ]
            .contains(&"Expected May 2027")
        );
        assert!(
            entry
                .fields
                .iter()
                .all(|field| !field.value.contains("Expected May 2027"))
        );
    }
    let mut value = input()["formatExampleOnly"].clone();
    let entry = &mut value["sections"][1]["entries"][0];
    entry["rightTop"] = json!("");
    entry["mainInfo"] =
        json!({"format":"paragraph","items":["May 2024–Present", "Built tools in 2024."]});
    let document = decode(&value.to_string()).unwrap();
    let entry = &document.sections[1].entries[0];
    assert_eq!(
        entry.dates.as_ref().unwrap()[0].display_text(),
        "May. 2024–Present"
    );
    assert_eq!(
        entry.field_text(ort_domain::FieldRole::Paragraph, ""),
        "Built tools in 2024."
    );
}

#[test]
fn date_mapping_preserves_nondate_content_and_displaced_right_hand_order() {
    let mut value = input()["formatExampleOnly"].clone();
    let entry = &mut value["sections"][1]["entries"][0];
    entry["leftHeading"] = json!("Project 2024");
    entry["leftSubheading"] = json!("Engineer | Jan. 2022–Present");
    entry["headingSuffix"] = json!("SQL Server 2022");
    entry["rightTop"] = json!("Boston");
    entry["rightMiddle"] = json!("Remote");
    entry["rightBottom"] = json!("Part time");
    let document = decode(&value.to_string()).unwrap();
    let entry = &document.sections[1].entries[0];
    assert_eq!(entry.heading, "Project 2024");
    assert_eq!(entry.subheading, "Engineer");
    assert_eq!(
        entry.field_text(ort_domain::FieldRole::Details, ""),
        "SQL Server 2022"
    );
    assert_eq!(
        entry.dates.as_ref().unwrap()[0].display_text(),
        "Jan. 2022–Present"
    );
    assert_eq!(entry.location, "Boston");
    assert_eq!(
        entry.field_text(ort_domain::FieldRole::Extra, ""),
        "Remote\nPart time"
    );
}

#[test]
fn date_slot_normalizes_supported_formats_into_selector_values() {
    for (source, display, start_month, end_month) in [
        (
            "January 2020 - September 2024",
            "Jan. 2020–Sep. 2024",
            Some(1),
            Some(9),
        ),
        ("05/2020 to current", "May. 2020–Present", Some(5), None),
        ("2020 - 2024", "2020–2024", None, None),
        ("June 12, 2024", "Jun. 2024", Some(6), None),
        ("2024-06-12", "Jun. 2024", Some(6), None),
        ("Fall 2020–Spring 2024", "2020–2024", None, None),
        ("Present", "Present", None, None),
    ] {
        let mut value = proposal();
        value["sections"][0]["entries"][0]["date"] = json!(source);
        let document = decode(&value.to_string()).unwrap();
        let entry = &document.sections[0].entries[0];
        assert!(entry.date_range.is_empty());
        let dates = entry.dates.as_ref().unwrap();
        assert_eq!(dates.len(), 1);
        let date = &dates[0];
        assert_eq!(date.display_text(), display);
        assert_eq!(
            date.start.as_ref().and_then(|start| start.month),
            start_month
        );
        assert_eq!(
            match &date.end {
                Some(ort_domain::DateEnd::Date { value }) => value.month,
                _ => None,
            },
            end_month
        );
        assert!(date.label.is_empty());
        assert!(date.start.as_ref().is_none_or(|start| !start.expected));
        assert_eq!(entry.location, "Remote");
        document.validate(DocumentLimits::default()).unwrap();
    }
}

#[test]
fn invalid_or_conflicting_dates_are_not_imported_as_free_text_or_hidden_dates() {
    for date in ["13/2024", "June someday 2024", "2020–2024\n2015–2016"] {
        let mut value = proposal();
        value["sections"][0]["entries"][0]["date"] = json!(date);
        let error = decode(&value.to_string()).unwrap_err();
        assert!(error.starts_with("sections[0].entries[0]"));
        assert!(!error.contains(date));
    }
    let mut value = proposal();
    value["sections"][0]["entries"][0]["date"] = json!("2020–2024\n2020 - 2024");
    let document = decode(&value.to_string()).unwrap();
    assert_eq!(
        document.sections[0].entries[0]
            .dates
            .as_ref()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn a_date_only_entry_remains_readable_after_structured_conversion() {
    let value = json!({"contact":{}, "sections":[{"heading":"Certification", "entries":[{
        "rightTop":"May 2024", "mainInfo":{"format":"paragraph", "items":[]}
    }]}]});
    let document = decode(&value.to_string()).unwrap();
    let entry = &document.sections[0].entries[0];
    assert!(entry.date_range.is_empty());
    assert_eq!(entry.dates.as_ref().unwrap()[0].display_text(), "May. 2024");
}
