//! Bounded editorial policy, independent of providers, storage and native UI.
use ort_ai::materials::{self, TailoredMaterial};
use ort_domain::{DocumentStyle, ResumeDocument};
use serde_json::{Value, json};

pub const MAX_TAILORING_CALLS: u8 = 4;
pub enum TailoringStep {
    Continue(Value),
    Ready(Box<TailoredMaterial>),
}
pub struct TailoringRun<'a> {
    source: &'a ResumeDocument,
    baseline: &'a ResumeDocument,
    job: &'a str,
    published_revision: i64,
    style: DocumentStyle,
    input: Value,
    calls: u8,
    page_count: Option<usize>,
    validation_feedback: Vec<String>,
}
impl<'a> TailoringRun<'a> {
    #[must_use]
    pub fn new(
        source: &'a ResumeDocument,
        baseline: &'a ResumeDocument,
        job: &'a str,
        published_revision: i64,
        style: DocumentStyle,
        input: Value,
    ) -> Self {
        Self {
            source,
            baseline,
            job,
            published_revision,
            style,
            input,
            calls: 0,
            page_count: None,
            validation_feedback: Vec::new(),
        }
    }
    #[must_use]
    pub const fn page_count(&self) -> Option<usize> {
        self.page_count
    }
    #[must_use]
    pub fn validation_feedback(&self) -> &[String] {
        &self.validation_feedback
    }
    /// Every response consumes a slot, including malformed responses. Only a
    /// locally valid candidate is eligible for persistence. The final revision
    /// is used even when editorial issues or page-fit problems remain.
    /// # Errors
    /// Returns the final recoverable failure when all four slots are exhausted.
    pub fn advance(&mut self, raw: &str) -> Result<TailoringStep, &'static str> {
        if self.calls >= MAX_TAILORING_CALLS {
            return Err("AI_TAILORING_FAILED");
        }
        self.calls += 1;
        self.page_count = None;
        self.validation_feedback.clear();
        let parsed = materials::validate_refinement_detailed(
            self.source,
            self.baseline,
            self.job,
            raw,
            self.published_revision,
        );
        let mut feedback = Vec::new();
        let mut failure = "AI_MATERIAL_INVALID";
        let mut accepted = None;
        match parsed {
            Err(error) => {
                if matches!(
                    error,
                    materials::MaterialError::Evidence
                        | materials::MaterialError::EvidenceDetail(_)
                ) {
                    failure = "AI_GROUNDING_FAILED";
                }
                self.validation_feedback.push(error.to_string());
                feedback.push(format!("Invalid response: {error}. Return a complete schema v6 candidate with valid same-entry source references and exact list selections."));
            }
            Ok(material) => {
                if !material.review_issues.is_empty() {
                    failure = "AI_REVIEW_FAILED";
                    self.validation_feedback.push(format!("The source and editorial review reported {} unresolved issues. Model-authored review text is excluded from activity diagnostics.", material.review_issues.len()));
                    feedback.extend(material.review_issues.iter().cloned());
                }
                let one_page = match ort_render::render_pdf_with_style(&material.resume, self.style)
                {
                    Ok(pdf) => {
                        self.page_count = Some(pdf.receipt.page_count);
                        feedback.push(content_diagnostics(&material.resume));
                        if pdf.receipt.page_count != 1 {
                            failure = "AI_PAGE_FIT_FAILED";
                            self.validation_feedback.push(format!("Rendered PDF has {} pages; exactly one page is required with the existing layout.", pdf.receipt.page_count));
                            feedback.push(format!("Rendered pageCount={}; target=1. Select and tighten content without changing formatting.",pdf.receipt.page_count));
                        }
                        pdf.receipt.page_count == 1
                    }
                    Err(ort_render::PdfRenderError::LayoutLimit) => {
                        failure = "AI_PAGE_FIT_FAILED";
                        self.validation_feedback
                            .push("The candidate exceeded the PDF renderer's layout limit.".into());
                        feedback.push(content_diagnostics(&material.resume));
                        feedback.push("PDF layout limit exceeded. Substantially reduce content; preserve the strongest direct evidence.".into());
                        false
                    }
                    Err(_) => {
                        failure = "PDF_UNAVAILABLE";
                        feedback.push("The fixed renderer could not render this candidate. Remove malformed or unsupported content without altering protected values.".into());
                        false
                    }
                };
                if self.calls == MAX_TAILORING_CALLS
                    || (one_page && material.review_issues.is_empty())
                {
                    accepted = Some(material);
                }
            }
        }
        // Slot two is always the independent editorial/source review.
        if self.calls >= 2
            && let Some(material) = accepted
        {
            return Ok(TailoringStep::Ready(Box::new(material)));
        }
        if self.calls == MAX_TAILORING_CALLS {
            return Err(failure);
        }
        let mut next = self.input.clone();
        next["qualityPhase"] = json!(if self.calls == 1 {
            "review"
        } else if self.calls + 1 == MAX_TAILORING_CALLS {
            "final_revision"
        } else {
            "correction"
        });
        next["callNumber"] = json!(self.calls + 1);
        next["validationFeedback"] = json!(feedback);
        next["previousCandidate"] = serde_json::from_str(raw).unwrap_or_else(
            |_| json!({"malformedResponse":raw.chars().take(8000).collect::<String>()}),
        );
        Ok(TailoringStep::Continue(next))
    }
}

fn content_diagnostics(resume: &ResumeDocument) -> String {
    let entries: Vec<_> = resume.sections.iter().flat_map(|s| &s.entries).collect();
    let characters: usize = entries
        .iter()
        .map(|e| {
            e.bullets
                .iter()
                .map(|b| b.text.chars().count())
                .sum::<usize>()
                + e.field_text(ort_domain::FieldRole::Paragraph, "\n")
                    .chars()
                    .count()
        })
        .sum();
    format!(
        "Candidate contains {} sections, {} entries and {characters} description characters.",
        resume.sections.len(),
        entries.len()
    )
}

fn reordered(
    old: &[ort_domain::EntityId],
    new: &[ort_domain::EntityId],
) -> std::collections::HashSet<ort_domain::EntityId> {
    let old_common: Vec<_> = old.iter().copied().filter(|id| new.contains(id)).collect();
    let new_common: Vec<_> = new.iter().copied().filter(|id| old.contains(id)).collect();
    old_common
        .into_iter()
        .zip(new_common)
        .filter_map(|(old, new)| (old != new).then_some(new))
        .collect()
}
fn names(values: &[&str]) -> String {
    if values.is_empty() {
        return String::new();
    }
    let samples: Vec<_> = values
        .iter()
        .take(3)
        .map(|s| {
            s.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(32)
                .collect::<String>()
        })
        .collect();
    format!(" ({})", samples.join(", "))
}

/// Compute visible structural changes from documents, never from model claims.
#[must_use]
pub fn change_summary(before: &ResumeDocument, after: &ResumeDocument) -> Vec<String> {
    let mut removed = Vec::new();
    let mut restored = Vec::new();
    let mut moved = Vec::new();
    let mut lists = 0;
    let mut rewritten = 0;
    let section_moves = reordered(
        &before.sections.iter().map(|s| s.id).collect::<Vec<_>>(),
        &after.sections.iter().map(|s| s.id).collect::<Vec<_>>(),
    );
    for s in &before.sections {
        if !after.sections.iter().any(|v| v.id == s.id) {
            removed.push(s.heading.as_str());
        }
        for e in &s.entries {
            if !after
                .sections
                .iter()
                .flat_map(|v| &v.entries)
                .any(|v| v.id == e.id)
            {
                removed.push(e.heading.as_str());
            }
        }
    }
    for s in &after.sections {
        let old = before.sections.iter().find(|v| v.id == s.id);
        if old.is_none() {
            restored.push(s.heading.as_str());
        }
        if section_moves.contains(&s.id) {
            moved.push(s.heading.as_str());
        }
        let entry_moves = old
            .map(|old| {
                reordered(
                    &old.entries.iter().map(|e| e.id).collect::<Vec<_>>(),
                    &s.entries.iter().map(|e| e.id).collect::<Vec<_>>(),
                )
            })
            .unwrap_or_default();
        for e in &s.entries {
            let previous = old.and_then(|v| v.entries.iter().find(|v| v.id == e.id));
            if entry_moves.contains(&e.id) {
                moved.push(e.heading.as_str());
            }
            if let Some(previous) = previous {
                if previous.bullets.iter().map(|b| &b.text).collect::<Vec<_>>()
                    != e.bullets.iter().map(|b| &b.text).collect::<Vec<_>>()
                    || previous.field_text(ort_domain::FieldRole::Paragraph, "\n")
                        != e.field_text(ort_domain::FieldRole::Paragraph, "\n")
                {
                    rewritten += 1;
                }
                if previous
                    .fields
                    .iter()
                    .filter(|f| f.selectable())
                    .map(|f| (f.id, f.value.as_str()))
                    .collect::<Vec<_>>()
                    != e.fields
                        .iter()
                        .filter(|f| f.selectable())
                        .map(|f| (f.id, f.value.as_str()))
                        .collect::<Vec<_>>()
                {
                    lists += 1;
                }
            } else {
                restored.push(e.heading.as_str());
            }
        }
    }
    let mut result = Vec::new();
    if !removed.is_empty() || !restored.is_empty() || !moved.is_empty() {
        result.push(format!(
            "Selection: removed {}{}, restored {}{}, reordered {}{} sections/items.",
            removed.len(),
            names(&removed),
            restored.len(),
            names(&restored),
            moved.len(),
            names(&moved)
        ));
    }
    if rewritten > 0 {
        result.push(format!("Updated descriptions in {rewritten} items."));
    }
    if lists > 0 {
        result.push(format!(
            "Changed skill/coursework selection in {lists} items."
        ));
    }
    result
}
