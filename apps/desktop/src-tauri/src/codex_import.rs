//! Image input for the qualified Codex 0.162.0 protocol, which has no PDF item.
//! Temporary page files live only inside the already-allowed private scratch.
use crate::import_review::ai_pdf::PdfPages;
use serde_json::{Value, json};
use std::{io::Write, path::Path};

pub(crate) struct PreparedInput {
    pub(crate) items: Vec<Value>,
    pages: Option<tempfile::TempDir>,
}
pub(crate) fn prepare(
    scratch: &Path,
    text: &str,
    pages: Option<&PdfPages>,
    cancelled: &dyn Fn() -> bool,
) -> Result<PreparedInput, &'static str> {
    let mut prepared = PreparedInput {
        items: vec![json!({"type":"text","text":text})],
        pages: None,
    };
    if let Some(pages) = pages {
        let directory = tempfile::tempdir_in(scratch).map_err(|_| "AI_PDF_RENDER_FAILED")?;
        for (index, bytes) in pages.pages().iter().enumerate() {
            if cancelled() {
                return Err("AI_CANCELLED");
            }
            let path = directory.path().join(format!("page-{}.png", index + 1));
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&path).map_err(|_| "AI_PDF_RENDER_FAILED")?;
            file.write_all(bytes).map_err(|_| "AI_PDF_RENDER_FAILED")?;
            prepared.items.push(json!({"type":"text","text":format!("Resume page {} of {}", index + 1, pages.pages().len())}));
            prepared
                .items
                .push(json!({"type":"localImage","path":path,"detail":"original"}));
        }
        prepared.pages = Some(directory);
    }
    Ok(prepared)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn ordered_pages_use_private_scratch_and_are_removed_on_every_exit() {
        let scratch = tempfile::tempdir().unwrap();
        let pages = PdfPages::fixture(2);
        let prepared = prepare(scratch.path(), "Extract the resume", Some(&pages), &|| {
            false
        })
        .unwrap();
        assert_eq!(prepared.items.len(), 5);
        let paths: Vec<_> = prepared
            .items
            .iter()
            .filter_map(|item| item["path"].as_str().map(std::path::PathBuf::from))
            .collect();
        for (index, path) in paths.iter().enumerate() {
            assert!(path.starts_with(scratch.path()));
            assert_eq!(std::fs::read(path).unwrap(), pages.pages()[index]);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                assert_eq!(
                    std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                    0o600
                );
            }
        }
        let schema: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/codex-0.162.0-image-input.json"
        ))
        .unwrap();
        assert_eq!(schema["version"], crate::codex_runtime::qualified_version());
        assert!(
            !schema["inputTypes"]
                .as_array()
                .unwrap()
                .contains(&json!("file"))
        );
        for item in prepared
            .items
            .iter()
            .filter(|item| item["type"] == "localImage")
        {
            for field in item.as_object().unwrap().keys() {
                assert!(schema["localImage"]["properties"].get(field).is_some());
            }
            for field in schema["localImage"]["required"].as_array().unwrap() {
                assert!(item.get(field.as_str().unwrap()).is_some());
            }
            assert!(
                schema["imageDetail"]["enum"]
                    .as_array()
                    .unwrap()
                    .contains(&item["detail"])
            );
        }
        assert_eq!(prepared.items[1]["text"], "Resume page 1 of 2");
        assert_eq!(prepared.items[3]["text"], "Resume page 2 of 2");
        drop(prepared);
        assert!(paths.iter().all(|path| !path.exists()));
        assert!(matches!(
            prepare(scratch.path(), "", Some(&pages), &|| true),
            Err("AI_CANCELLED")
        ));
        assert_eq!(std::fs::read_dir(scratch.path()).unwrap().count(), 0);
        assert_eq!(
            prepare(scratch.path(), "text only", None, &|| false)
                .unwrap()
                .items,
            vec![json!({"type":"text","text":"text only"})]
        );
    }

    #[test]
    fn page_input_passes_stdio_with_no_tool_requests_and_cleans_up_after_rejection() {
        for mode in ["pdf_import", "turn_start_rejected"] {
            let mut session = crate::codex_runtime::test_session(mode);
            assert!(session.image_models.contains("gpt-6.1-sol"));
            let pages = PdfPages::fixture_with_links(
                2,
                vec![ort_ai::resume_import::PdfHyperlink {
                    page: 1,
                    rect: [80, 210, 160, 230],
                    url: "https://github.com/example/project".into(),
                }],
            );
            let prepared = prepare(
                session.scratch_path(),
                &ort_ai::resume_import::input_with_pdf_hyperlinks(pages.links()).to_string(),
                Some(&pages),
                &|| false,
            )
            .unwrap();
            let path = std::path::PathBuf::from(prepared.items[2]["path"].as_str().unwrap());
            let end = Instant::now() + Duration::from_secs(3);
            let result = session.with_ephemeral_thread(json!({"model":"gpt-6.1-sol","ephemeral":true}), end, &|| false,
                |session, _, thread| {
                    let turn = session.rpc("turn/start", json!({"threadId":thread,"model":"gpt-6.1-sol","effort":"medium","approvalPolicy":"never","input":prepared.items,"outputSchema":ort_ai::resume_import::output_schema()}), Duration::from_secs(2), &|| false)?;
                    session.run_turn(thread, turn["turn"]["id"].as_str().unwrap(), end, &|| false)
                });
            assert_eq!(result.is_ok(), mode == "pdf_import");
            if let Ok(text) = result {
                let document = ort_ai::resume_import::decode(&text).unwrap();
                document
                    .validate(ort_domain::DocumentLimits::default())
                    .unwrap();
                assert_eq!(
                    document.contact.links[0].url,
                    "https://linkedin.com/in/example-person"
                );
                assert_eq!(
                    document.sections[0].entries[0].bullets[0].text,
                    "Example source bullet, copied verbatim."
                );
                assert_eq!(
                    document.sections[1].entries[0].subheading,
                    "[GitHub](https://github.com/example/project)"
                );
            }
            drop(prepared);
            assert!(!path.exists());
        }
    }
}
