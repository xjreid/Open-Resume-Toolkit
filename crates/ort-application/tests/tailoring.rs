use ort_application::tailoring::{TailoringRun, TailoringStep, change_summary};
use ort_domain::{DocumentStyle, ResumeDocument};
use serde_json::{Value, json};
fn fixture(name: &str) -> (ResumeDocument, String, Value) {
    let raw = std::fs::read_to_string(format!(
        "{}/../../fixtures/ai/tailoring-v6/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    let v: Value = serde_json::from_str(&raw).unwrap();
    (
        serde_json::from_value(v["source"].clone()).unwrap(),
        v["job"].as_str().unwrap().into(),
        v["candidate"].clone(),
    )
}
#[test]
fn mandatory_review_even_when_initial_candidate_is_one_page() {
    let (source, job, candidate) = fixture("student-backend");
    let mut run = TailoringRun::new(
        &source,
        &source,
        &job,
        1,
        DocumentStyle::Technical,
        json!({"publishedResume":"frozen"}),
    );
    let TailoringStep::Continue(next) = run.advance(&candidate.to_string()).unwrap() else {
        panic!("draft must be reviewed")
    };
    assert_eq!(next["qualityPhase"], "review");
    assert_eq!(next["publishedResume"], "frozen");
    assert!(matches!(
        run.advance(&candidate.to_string()).unwrap(),
        TailoringStep::Ready(_)
    ));
}
#[test]
fn malformed_draft_is_repaired_in_review_slot() {
    let (source, job, candidate) = fixture("experienced-data");
    let mut run = TailoringRun::new(
        &source,
        &source,
        &job,
        1,
        DocumentStyle::Technical,
        json!({}),
    );
    let TailoringStep::Continue(next) = run.advance("not JSON").unwrap() else {
        panic!("repair")
    };
    assert_eq!(next["qualityPhase"], "review");
    assert!(!next["validationFeedback"].as_array().unwrap().is_empty());
    assert!(matches!(
        run.advance(&candidate.to_string()).unwrap(),
        TailoringStep::Ready(_)
    ));
}

#[test]
fn correction_receives_specific_schema_and_selection_failures() {
    let (source, job, mut candidate) = fixture("student-backend");
    candidate.as_object_mut().unwrap().remove("reviewIssues");
    let mut run = TailoringRun::new(
        &source,
        &source,
        &job,
        1,
        DocumentStyle::Technical,
        json!({}),
    );
    let TailoringStep::Continue(next) = run.advance(&candidate.to_string()).unwrap() else {
        panic!("review repairs the missing field");
    };
    assert!(
        next["validationFeedback"]
            .to_string()
            .contains("Missing required schema v6 field: reviewIssues")
    );
    assert!(run.validation_feedback()[0].contains("reviewIssues"));
    assert_eq!(run.page_count(), None);
}
#[test]
fn unresolved_factual_review_issues_block_acceptance_and_stop_after_four() {
    let (source, job, mut candidate) = fixture("unsupported-metric");
    candidate["templateSections"][0]["entries"][0]["mainInfo"]["items"][0]["text"] =
        json!("Improved reporting speed by 90%.");
    candidate["reviewIssues"] = json!(["The published source does not support the 90% metric."]);
    let mut run = TailoringRun::new(
        &source,
        &source,
        &job,
        1,
        DocumentStyle::Technical,
        json!({}),
    );
    for _ in 0..3 {
        assert!(matches!(
            run.advance(&candidate.to_string()).unwrap(),
            TailoringStep::Continue(_)
        ));
    }
    assert!(matches!(
        run.advance(&candidate.to_string()),
        Err("AI_REVIEW_FAILED")
    ));
    assert!(matches!(
        run.advance(&candidate.to_string()),
        Err("AI_TAILORING_FAILED")
    ));
}
#[test]
fn overflowing_candidate_can_be_corrected_without_layout_changes() {
    let (source, job, mut candidate) = fixture("oversized-master");
    let style = DocumentStyle::Technical;
    let mut run = TailoringRun::new(
        &source,
        &source,
        &job,
        1,
        style,
        json!({"style":"technical"}),
    );
    let TailoringStep::Continue(next) = run.advance(&candidate.to_string()).unwrap() else {
        panic!("overflow")
    };
    assert_eq!(next["style"], "technical");
    assert!(!next["validationFeedback"].as_array().unwrap().is_empty());
    candidate["templateSections"][0]["entries"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    assert!(matches!(
        run.advance(&candidate.to_string()).unwrap(),
        TailoringStep::Ready(_)
    ));
}
#[test]
fn page_fit_feedback_reports_measured_count_and_content_length() {
    let (source, job, mut candidate) = fixture("oversized-master");
    candidate["templateSections"][0]["entries"]
        .as_array_mut()
        .unwrap()
        .truncate(8);
    let mut run = TailoringRun::new(
        &source,
        &source,
        &job,
        1,
        DocumentStyle::Technical,
        json!({}),
    );
    let TailoringStep::Continue(next) = run.advance(&candidate.to_string()).unwrap() else {
        panic!("initial candidate requires review");
    };
    let pages = run.page_count().expect("bounded candidate must render");
    assert!(pages > 1);
    assert!(
        next["validationFeedback"]
            .to_string()
            .contains(&format!("pageCount={pages}"))
    );
    assert!(
        next["validationFeedback"]
            .to_string()
            .contains("description characters")
    );
}
#[test]
fn renderer_layout_limit_requests_reduction_without_acceptance() {
    let (mut source, job, candidate) = fixture("oversized-master");
    for entry in &mut source.sections[0].entries {
        entry
            .heading
            .push_str(&" Additional source context".repeat(8));
    }
    source
        .validate(ort_domain::DocumentLimits::default())
        .unwrap();
    let mut run = TailoringRun::new(
        &source,
        &source,
        &job,
        1,
        DocumentStyle::Technical,
        json!({}),
    );
    let TailoringStep::Continue(next) = run.advance(&candidate.to_string()).unwrap() else {
        panic!("layout limit requires correction");
    };
    assert_eq!(run.page_count(), None);
    assert!(
        next["validationFeedback"]
            .to_string()
            .contains("layout limit exceeded")
    );
}

#[test]
fn repeated_overflow_exhausts_cap_and_preserves_baseline() {
    let (source, job, candidate) = fixture("oversized-master");
    let before = source.clone();
    let mut run = TailoringRun::new(
        &source,
        &source,
        &job,
        1,
        DocumentStyle::Technical,
        json!({}),
    );
    for _ in 0..3 {
        assert!(matches!(
            run.advance(&candidate.to_string()).unwrap(),
            TailoringStep::Continue(_)
        ));
    }
    assert!(matches!(
        run.advance(&candidate.to_string()),
        Err("AI_PAGE_FIT_FAILED")
    ));
    assert_eq!(source, before);
}
#[test]
fn fixture_corpus_contracts_and_actual_summary() {
    for name in [
        "student-backend",
        "experienced-data",
        "sparse-career-change",
        "oversized-master",
        "misleading-job",
        "unsupported-metric",
        "ambiguous-skills",
    ] {
        let (source, job, candidate) = fixture(name);
        let result = ort_ai::materials::validate_template_tailoring(
            &source,
            &job,
            &candidate.to_string(),
            1,
        )
        .unwrap();
        assert!(change_summary(&source, &result.resume).is_empty(), "{name}");
    }
    let (source, job, mut candidate) = fixture("oversized-master");
    candidate["templateSections"][0]["entries"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    let result =
        ort_ai::materials::validate_template_tailoring(&source, &job, &candidate.to_string(), 1)
            .unwrap();
    assert!(change_summary(&source, &result.resume)[0].contains("removed 37"));
}

#[test]
fn summary_does_not_report_reordering_from_removal_alone() {
    let (source, job, mut candidate) = fixture("oversized-master");
    candidate["templateSections"][0]["entries"]
        .as_array_mut()
        .unwrap()
        .remove(0);
    let result =
        ort_ai::materials::validate_template_tailoring(&source, &job, &candidate.to_string(), 1)
            .unwrap();
    let summary = change_summary(&source, &result.resume);
    assert!(summary[0].contains("removed 1"));
    assert!(summary[0].contains("Example 1"));
    assert!(summary[0].contains("reordered 0"));
}

#[test]
fn summary_ignores_field_slot_changes_when_exact_list_is_unchanged() {
    let (source, _, _) = fixture("student-backend");
    let mut after = source.clone();
    after.sections[0].entries[0].fields[0].order = 1;
    assert!(change_summary(&source, &after).is_empty());
}
