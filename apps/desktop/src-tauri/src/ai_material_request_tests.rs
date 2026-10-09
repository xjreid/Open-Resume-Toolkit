use super::*;
use ort_ai::{Provider, plan::ReasoningEffort};

fn completed(call: u8, reasoning: Option<ReasoningEffort>) -> CompletedPass {
    CompletedPass {
        attempt_id: Uuid::from_u128(u128::from(call)),
        text: call.to_string(),
        started_at_unix_ms: now_unix_ms().unwrap() - 5_000,
        provider: Provider::OpenAi,
        model: "frozen-model".into(),
        reasoning,
        reported_retries: 3,
    }
}

#[tokio::test]
async fn both_provider_results_share_four_stages_and_attempt_linkage() {
    for reasoning in [None, Some(ReasoningEffort::High)] {
        let mut calls = vec![];
        let result = run_stages(
            9,
            json!({"stage":1}),
            &|| false,
            |input, call, previous| {
                assert_eq!(input["stage"], call);
                assert_eq!(
                    previous,
                    (call > 1).then(|| Uuid::from_u128(u128::from(call - 1)))
                );
                calls.push(call);
                async move { Ok(completed(call, reasoning)) }
            },
            |pass, call| {
                assert_eq!(pass.text, call.to_string());
                Ok(if call == 4 {
                    MaterialDecision::Complete("best final revision")
                } else {
                    MaterialDecision::Continue(json!({"stage":call+1}))
                })
            },
        )
        .await
        .unwrap();
        assert_eq!(calls, [1, 2, 3, 4]);
        assert_eq!(result, "best final revision");
    }
}

#[tokio::test]
async fn local_validation_failure_retains_original_duration_and_codex_retries() {
    let failure = run_stages(
        4,
        json!({}),
        &|| false,
        |_, call, _| async move { Ok(completed(call, Some(ReasoningEffort::High))) },
        |pass, call| -> Result<MaterialDecision<()>, MaterialFailure> {
            let details = execution::failure_details("AI_OUTPUT_INVALID");
            Err(pass.failure(
                "AI_OUTPUT_INVALID",
                &details,
                Uuid::nil(),
                OperationType::TailorResume,
                call,
                4,
            ))
        },
    )
    .await
    .err()
    .unwrap();
    assert_eq!(failure.details["reportedRetries"], 3);
    assert!(failure.details["durationMs"].as_i64().unwrap() >= 5_000);
    assert_eq!(failure.details["connectionSource"], "chatgpt_plan");
    assert_eq!(failure.details["model"], "frozen-model");
    assert_eq!(failure.attempt_id, Some(Uuid::from_u128(1)));
}

#[tokio::test]
async fn cancellation_before_and_after_dispatch_prevents_advancement() {
    use std::sync::atomic::{AtomicBool, Ordering};
    for before in [true, false] {
        let cancelled = AtomicBool::new(before);
        let mut dispatches = 0;
        let failure = run_stages(
            4,
            json!({}),
            &|| cancelled.load(Ordering::Acquire),
            |_, call, _| {
                dispatches += 1;
                cancelled.store(true, Ordering::Release);
                async move { Ok(completed(call, None)) }
            },
            |_, _| -> Result<MaterialDecision<()>, MaterialFailure> {
                panic!("cancelled pass advanced")
            },
        )
        .await
        .err()
        .unwrap();
        assert_eq!(failure.code, "AI_CANCELLED");
        assert_eq!(dispatches, usize::from(!before));
    }
}
