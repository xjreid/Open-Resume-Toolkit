//! Versioned application-material responses. Provider text is untrusted until
//! these validators resolve it against the exact published source and job text.
use std::collections::HashSet;

use ort_domain::{DocumentLimits, EntityId, ResumeDocument};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MATERIALS_SCHEMA_VERSION: u16 = 1;
pub const MAX_JOB_CHARS: usize = 20_000;
pub const MAX_QUESTION_CHARS: usize = 2_000;
pub const MAX_ANSWER_CHARS: usize = 4_000;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MaterialError {
    #[error("invalid application material")]
    Invalid,
    #[error("unsupported application-material schema")]
    Version,
    #[error("the question requires a personal answer")]
    PersonalQuestion,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TailorResponse {
    pub schema_version: u16,
    #[serde(default)]
    pub resume: Option<ResumeDocument>,
    #[serde(default)]
    pub selected_sections: Option<Vec<SectionSelection>>,
    pub alerts: Vec<AlertCandidate>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SectionSelection {
    pub section_id: EntityId,
    pub entries: Vec<EntrySelection>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntrySelection {
    pub entry_id: EntityId,
    pub bullet_ids: Vec<EntityId>,
}

fn materialize_selection(
    source: &ResumeDocument,
    selected: Vec<SectionSelection>,
) -> Result<ResumeDocument, MaterialError> {
    if selected.is_empty() || selected.len() > source.sections.len() {
        return Err(MaterialError::Invalid);
    }
    let mut resume = source.clone();
    let mut section_ids = HashSet::new();
    resume.sections = selected
        .into_iter()
        .enumerate()
        .map(|(section_order, section)| {
            if !section_ids.insert(section.section_id) {
                return Err(MaterialError::Invalid);
            }
            let original = source
                .sections
                .iter()
                .find(|item| item.id == section.section_id)
                .ok_or(MaterialError::Invalid)?;
            if section.entries.len() > original.entries.len() {
                return Err(MaterialError::Invalid);
            }
            let mut output = original.clone();
            let mut entry_ids = HashSet::new();
            output.entries = section
                .entries
                .into_iter()
                .enumerate()
                .map(|(entry_order, entry)| {
                    if !entry_ids.insert(entry.entry_id) {
                        return Err(MaterialError::Invalid);
                    }
                    let original_entry = original
                        .entries
                        .iter()
                        .find(|item| item.id == entry.entry_id)
                        .ok_or(MaterialError::Invalid)?;
                    if entry.bullet_ids.len() > original_entry.bullets.len() {
                        return Err(MaterialError::Invalid);
                    }
                    let mut item = original_entry.clone();
                    let mut bullet_ids = HashSet::new();
                    item.bullets = entry
                        .bullet_ids
                        .into_iter()
                        .enumerate()
                        .map(|(bullet_order, id)| {
                            if !bullet_ids.insert(id) {
                                return Err(MaterialError::Invalid);
                            }
                            let mut bullet = original_entry
                                .bullets
                                .iter()
                                .find(|item| item.id == id)
                                .ok_or(MaterialError::Invalid)?
                                .clone();
                            bullet.order =
                                u16::try_from(bullet_order).map_err(|_| MaterialError::Invalid)?;
                            Ok(bullet)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    item.order = u16::try_from(entry_order).map_err(|_| MaterialError::Invalid)?;
                    Ok(item)
                })
                .collect::<Result<Vec<_>, _>>()?;
            output.order = u16::try_from(section_order).map_err(|_| MaterialError::Invalid)?;
            Ok(output)
        })
        .collect::<Result<Vec<_>, _>>()?;
    resume
        .validate(DocumentLimits::default())
        .map_err(|_| MaterialError::Invalid)?;
    Ok(resume)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertKind {
    NotFound,
    ConfirmedMismatch,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertCategory {
    DegreeLevel,
    FieldOfStudy,
    GraduationDate,
    CertificationOrProfessionalLicense,
    NamedSkillOrTechnology,
    LanguageProficiency,
    ExperienceDuration,
    PortfolioOrWorkSample,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlertCandidate {
    pub kind: AlertKind,
    pub category: AlertCategory,
    pub requirement: String,
    pub target: String,
    pub job_excerpt: String,
    pub resume_evidence: Option<AlertEvidence>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlertEvidence {
    pub field_id: EntityId,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualificationAlert {
    pub id: String,
    pub kind: AlertKind,
    pub category: AlertCategory,
    pub requirement: String,
    pub job_excerpt: String,
    pub job_start: usize,
    pub job_end: usize,
    pub mandatory_reason: String,
    pub resume_evidence: Option<AlertEvidence>,
    pub validation_version: u16,
    pub published_revision: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TailoredMaterial {
    pub resume: ResumeDocument,
    pub change_points: Vec<String>,
    pub alerts: Vec<QualificationAlert>,
    pub alerts_truncated: bool,
}

fn source_text(source: &ResumeDocument) -> String {
    let mut facts = Vec::new();
    facts.extend([
        source.contact.full_name.as_str(),
        source.contact.email.as_str(),
        source.contact.phone.as_str(),
        source.contact.location.as_str(),
    ]);
    for section in &source.sections {
        facts.push(&section.heading);
        for entry in &section.entries {
            facts.extend([
                entry.heading.as_str(),
                entry.subheading.as_str(),
                entry.date_range.as_str(),
                entry.location.as_str(),
            ]);
            facts.extend(entry.fields.iter().map(|field| field.value.as_str()));
            facts.extend(entry.bullets.iter().map(|bullet| bullet.text.as_str()));
        }
    }
    facts.join(" ").to_lowercase()
}

/// Accepts a proposal only when every field and piece of prose is an exact
/// published-source fact. Selection and ordering are the model's only powers;
/// users may subsequently edit the workspace copy themselves.
///
/// # Errors
/// Returns an error for malformed output, an unsupported schema, or any
/// selected content that cannot be resolved to the published source.
#[allow(clippy::too_many_lines)]
pub fn validate_tailoring(
    source: &ResumeDocument,
    job: &str,
    raw: &str,
    published_revision: i64,
) -> Result<TailoredMaterial, MaterialError> {
    source
        .validate(DocumentLimits::default())
        .map_err(|_| MaterialError::Invalid)?;
    if published_revision < 1
        || job.trim().is_empty()
        || job.chars().count() > MAX_JOB_CHARS
        || raw.len() > 512 * 1024
    {
        return Err(MaterialError::Invalid);
    }
    let proposed: TailorResponse = serde_json::from_str(raw).map_err(|_| MaterialError::Invalid)?;
    if proposed.schema_version != MATERIALS_SCHEMA_VERSION {
        return Err(MaterialError::Version);
    }
    let resume = match (proposed.resume, proposed.selected_sections) {
        (Some(resume), None) => resume,
        (None, Some(selection)) => materialize_selection(source, selection)?,
        _ => return Err(MaterialError::Invalid),
    };
    resume
        .validate(DocumentLimits::default())
        .map_err(|_| MaterialError::Invalid)?;
    if resume.schema_version != source.schema_version
        || resume.document_id != source.document_id
        || resume.title != source.title
        || resume.contact != source.contact
        || resume.sections.is_empty()
        || resume.sections.len() > source.sections.len()
    {
        return Err(MaterialError::Invalid);
    }
    let mut section_ids = HashSet::new();
    let mut removed_sections = 0;
    let mut removed_entries = 0;
    let mut removed_bullets = 0;
    for section in &resume.sections {
        if !section_ids.insert(section.id) {
            return Err(MaterialError::Invalid);
        }
        let original = source
            .sections
            .iter()
            .find(|value| value.id == section.id)
            .ok_or(MaterialError::Invalid)?;
        if section.heading != original.heading || section.entries.len() > original.entries.len() {
            return Err(MaterialError::Invalid);
        }
        removed_entries += original.entries.len() - section.entries.len();
        let mut entry_ids = HashSet::new();
        for entry in &section.entries {
            if !entry_ids.insert(entry.id) {
                return Err(MaterialError::Invalid);
            }
            let old = original
                .entries
                .iter()
                .find(|value| value.id == entry.id)
                .ok_or(MaterialError::Invalid)?;
            if entry.heading != old.heading
                || entry.subheading != old.subheading
                || entry.date_range != old.date_range
                || entry.dates != old.dates
                || entry.location != old.location
                || entry.fields != old.fields
                || entry.links != old.links
                || entry.bullets.len() > old.bullets.len()
            {
                return Err(MaterialError::Invalid);
            }
            removed_bullets += old.bullets.len() - entry.bullets.len();
            let mut bullet_ids = HashSet::new();
            for bullet in &entry.bullets {
                if !bullet_ids.insert(bullet.id)
                    || !old
                        .bullets
                        .iter()
                        .any(|value| value.id == bullet.id && value.text == bullet.text)
                {
                    return Err(MaterialError::Invalid);
                }
            }
        }
    }
    removed_sections += source.sections.len() - resume.sections.len();
    let mut change_points = Vec::new();
    if removed_sections > 0 {
        change_points.push(format!(
            "Focused the resume by omitting {removed_sections} section(s)."
        ));
    }
    if removed_entries > 0 {
        change_points.push(format!(
            "Selected relevant experience by omitting {removed_entries} entry/entries."
        ));
    }
    if removed_bullets > 0 {
        change_points.push(format!(
            "Condensed details by omitting {removed_bullets} bullet(s)."
        ));
    }
    if change_points.is_empty() {
        change_points.push("Kept the published resume content.".into());
    }
    let alerts_truncated = proposed.alerts.len() > 10;
    let mut alerts = Vec::new();
    let mut seen = HashSet::new();
    let source_lc = source_text(source);
    for candidate in proposed.alerts.into_iter().take(20) {
        let excerpt = candidate.job_excerpt.trim();
        let target = candidate.target.trim();
        if excerpt.is_empty()
            || excerpt.chars().count() > 500
            || target.len() < 2
            || target.len() > 100
            || candidate.requirement.is_empty()
            || candidate.requirement.chars().count() > 500
        {
            continue;
        }
        let excerpt_lc = excerpt.to_lowercase();
        let target_lc = target.to_lowercase();
        let Some(job_start) = job.find(excerpt) else {
            continue;
        };
        let job_end = job_start + excerpt.len();
        if !contains_whole_phrase(&excerpt_lc, &target_lc)
            || !contains_whole_phrase(&candidate.requirement.to_lowercase(), &target_lc)
            || !explicitly_required(&excerpt_lc)
            || excluded_requirement(&excerpt_lc)
            || !category_matches(&candidate.category, &excerpt_lc, &target_lc, source)
        {
            continue;
        }
        let evidence = match candidate.kind {
            AlertKind::NotFound => {
                if candidate.resume_evidence.is_some()
                    || contains_whole_phrase(&source_lc, &target_lc)
                    || (matches!(candidate.category, AlertCategory::GraduationDate)
                        && source
                            .sections
                            .iter()
                            .flat_map(|section| &section.entries)
                            .flat_map(|entry| &entry.fields)
                            .any(|field| {
                                field.label.to_lowercase().contains("graduat")
                                    && valid_year(field.value.trim())
                            }))
                    || matches!(candidate.category, AlertCategory::ExperienceDuration)
                {
                    continue;
                }
                None
            }
            AlertKind::ConfirmedMismatch => {
                if !matches!(candidate.category, AlertCategory::GraduationDate)
                    || !excerpt_lc.contains("graduat")
                    || !valid_year(&target_lc)
                {
                    continue;
                }
                let Some(value) = candidate.resume_evidence.as_ref() else {
                    continue;
                };
                let evidence_year = value.value.trim();
                if !valid_year(evidence_year) || evidence_year == target_lc {
                    continue;
                }
                let resolves = source
                    .sections
                    .iter()
                    .flat_map(|section| &section.entries)
                    .flat_map(|entry| &entry.fields)
                    .any(|field| {
                        field.id == value.field_id
                            && field.label.to_lowercase().contains("graduat")
                            && field.value.trim() == evidence_year
                    });
                if !resolves {
                    continue;
                }
                Some(value.clone())
            }
        };
        let key = format!("{excerpt_lc}:{target_lc}");
        if !seen.insert(key.clone()) {
            continue;
        }
        alerts.push(QualificationAlert {
            id: Uuid::now_v7().to_string(),
            kind: candidate.kind,
            category: candidate.category,
            requirement: candidate.requirement,
            job_excerpt: candidate.job_excerpt,
            job_start,
            job_end,
            mandatory_reason: mandatory_reason(&excerpt_lc).to_owned(),
            resume_evidence: evidence,
            validation_version: MATERIALS_SCHEMA_VERSION,
            published_revision,
        });
        if alerts.len() == 10 {
            break;
        }
    }
    Ok(TailoredMaterial {
        resume,
        change_points,
        alerts,
        alerts_truncated,
    })
}

fn mandatory_reason(text: &str) -> &'static str {
    if text.contains("required") {
        "explicit_required"
    } else if text.contains("must have") || text.contains("must possess") {
        "must_have"
    } else if text.contains("minimum") {
        "minimum"
    } else {
        "mandatory"
    }
}

fn contains_whole_phrase(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    haystack.match_indices(needle).any(|(start, _)| {
        let before = haystack[..start].chars().next_back();
        let after = haystack[start + needle.len()..].chars().next();
        !before.is_some_and(char::is_alphanumeric) && !after.is_some_and(char::is_alphanumeric)
    })
}

fn has_word_stem(text: &str, stem: &str) -> bool {
    text.split(|character: char| !character.is_alphanumeric())
        .any(|word| word.starts_with(stem))
}

fn valid_year(value: &str) -> bool {
    value.len() == 4 && value.starts_with("20") && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn category_matches(
    category: &AlertCategory,
    excerpt: &str,
    target: &str,
    source: &ResumeDocument,
) -> bool {
    match category {
        AlertCategory::DegreeLevel => ["degree", "bachelor", "master", "doctorate", "phd"]
            .iter()
            .any(|word| excerpt.contains(word)),
        AlertCategory::FieldOfStudy => ["degree in", "major in", "field of study", "studied"]
            .iter()
            .any(|word| excerpt.contains(word)),
        AlertCategory::GraduationDate => excerpt.contains("graduat") && valid_year(target),
        AlertCategory::CertificationOrProfessionalLicense => [
            "certification",
            "certified",
            "professional license",
            "licensed",
        ]
        .iter()
        .any(|word| excerpt.contains(word)),
        AlertCategory::NamedSkillOrTechnology => {
            target.chars().any(char::is_alphabetic) && target.split_whitespace().count() <= 4
        }
        AlertCategory::PortfolioOrWorkSample => {
            ["portfolio", "work sample", "sample work"]
                .iter()
                .any(|word| excerpt.contains(word))
                && source.contact.links.is_empty()
                && source
                    .sections
                    .iter()
                    .all(|section| section.entries.iter().all(|entry| entry.links.is_empty()))
        }
        AlertCategory::LanguageProficiency => {
            ["language", "fluent", "proficient", "fluency"]
                .iter()
                .any(|word| contains_whole_phrase(excerpt, word))
                && target
                    .chars()
                    .all(|character| character.is_alphabetic() || character == ' ')
                && target.split_whitespace().count() <= 2
        }
        AlertCategory::ExperienceDuration => false,
    }
}

fn explicitly_required(text: &str) -> bool {
    [
        "required",
        "must have",
        "must possess",
        "minimum",
        "mandatory",
    ]
    .iter()
    .any(|term| contains_whole_phrase(text, term))
        && ![
            "not required",
            "no requirement",
            "not mandatory",
            "no minimum",
        ]
        .iter()
        .any(|term| contains_whole_phrase(text, term))
        && ![
            "preferred",
            "nice to have",
            "nice-to-have",
            "bonus",
            "desired",
            "recommended",
        ]
        .iter()
        .any(|term| contains_whole_phrase(text, term))
}

fn excluded_requirement(text: &str) -> bool {
    [
        "citizen",
        "visa",
        "work authoriz",
        "sponsorship",
        "disabilit",
        "medical",
        "criminal",
        "background check",
        "security clearance",
        "drug test",
        "age",
        "gender",
        "race",
        "religion",
        "relocat",
        "travel",
        "salary",
        "compensation",
        "driver",
        "driving",
        "consent",
        "signature",
    ]
    .iter()
    .any(|term| {
        if term.contains(' ') || *term == "age" {
            contains_whole_phrase(text, term)
        } else {
            has_word_stem(text, term)
        }
    })
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceSelection {
    pub schema_version: u16,
    pub evidence_ids: Vec<EntityId>,
}

fn selected_evidence(
    raw: &str,
    source: &ResumeDocument,
    max: usize,
) -> Result<Vec<String>, MaterialError> {
    let response: EvidenceSelection =
        serde_json::from_str(raw).map_err(|_| MaterialError::Invalid)?;
    if response.schema_version != MATERIALS_SCHEMA_VERSION {
        return Err(MaterialError::Version);
    }
    if response.evidence_ids.is_empty() || response.evidence_ids.len() > max {
        return Err(MaterialError::Invalid);
    }
    let mut seen = HashSet::new();
    let mut selected = Vec::new();
    for id in response.evidence_ids {
        if !seen.insert(id) {
            return Err(MaterialError::Invalid);
        }
        let text = source
            .sections
            .iter()
            .flat_map(|section| &section.entries)
            .find_map(|entry| {
                entry
                    .bullets
                    .iter()
                    .find(|bullet| bullet.id == id)
                    .map(|bullet| bullet.text.as_str())
                    .or_else(|| {
                        entry
                            .fields
                            .iter()
                            .find(|field| field.id == id)
                            .map(|field| field.value.as_str())
                    })
            })
            .ok_or(MaterialError::Invalid)?;
        if text.trim().is_empty() || text.len() > 500 {
            return Err(MaterialError::Invalid);
        }
        selected.push(text.trim().to_owned());
    }
    Ok(selected)
}

/// Builds a cover letter using only IDs that resolve to the published resume.
///
/// # Errors
/// Returns an error for malformed selections or unsupported evidence IDs.
pub fn validate_cover_letter(
    raw: &str,
    source: &ResumeDocument,
    _job: &str,
) -> Result<String, MaterialError> {
    let evidence = selected_evidence(raw, source, 5)?;
    let name = source.contact.full_name.trim();
    let signature = if name.is_empty() { "Applicant" } else { name };
    Ok(format!(
        "Dear Hiring Team,\n\nI am writing about the role described in your job posting. My published resume includes the following relevant experience:\n\n{}\n\nI would welcome the opportunity to discuss how this background relates to your requirements.\n\nSincerely,\n{}",
        evidence.join("\n\n"),
        signature
    ))
}

pub fn question_requires_personal_answer(question: &str) -> bool {
    let words = question
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let phrase = format!(" {} ", words.join(" "));
    [
        "citizen",
        "visa",
        "sponsor",
        "disabilit",
        "medical",
        "criminal",
        "convict",
        "arrest",
        "race",
        "gender",
        "religion",
        "ethnic",
        "signature",
        "consent",
        "attest",
        "truthful",
        "immigration",
        "passport",
    ]
    .iter()
    .any(|stem| words.iter().any(|word| word.starts_with(stem)))
        || words.iter().any(|word| word == "age")
        || [
            "work authorization",
            "authorized to work",
            "eligible to work",
            "work permit",
            "salary history",
            "previous salary",
            "past salary",
            "background check",
            "declaration of truth",
            "legal status",
            "social security number",
            "security clearance",
            "drug test",
        ]
        .iter()
        .any(|text| phrase.contains(&format!(" {text} ")))
}

/// Builds an editable answer from verified published-resume evidence.
///
/// # Errors
/// Returns an error for personal questions, malformed selections, unsupported
/// evidence IDs, or answers exceeding the requested limit.
pub fn validate_answer(
    raw: &str,
    source: &ResumeDocument,
    question: &str,
    limit: Option<usize>,
) -> Result<String, MaterialError> {
    if question_requires_personal_answer(question) {
        return Err(MaterialError::PersonalQuestion);
    }
    let evidence = selected_evidence(raw, source, 3)?;
    let text = evidence.join(" ");
    if text.chars().count() > MAX_ANSWER_CHARS
        || limit.is_some_and(|limit| text.chars().count() > limit)
    {
        return Err(MaterialError::Invalid);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ort_domain::{Bullet, NamedField, ResumeEntry, ResumeSection};
    use serde_json::json;

    fn source() -> ResumeDocument {
        let mut source = ResumeDocument::empty("Resume");
        source.contact.full_name = "Alex Rivera".into();
        source.sections = vec![ResumeSection {
            id: EntityId::new(),
            order: 0,
            heading: "Experience".into(),
            entries: vec![ResumeEntry {
                id: EntityId::new(),
                order: 0,
                heading: "Engineer".into(),
                subheading: "North Co".into(),
                date_range: "2021–2024".into(),
                dates: None,
                location: String::new(),
                fields: vec![NamedField {
                    id: EntityId::new(),
                    order: 0,
                    label: "Skill".into(),
                    value: "Rust".into(),
                    is_skill: true,
                }],
                bullets: vec![Bullet {
                    id: EntityId::new(),
                    order: 0,
                    text: "Built reliable tools".into(),
                }],
                links: vec![],
            }],
        }];
        source
    }

    #[test]
    fn rejects_fabricated_tailoring_and_malformed_output() {
        let source = source();
        let job = "Python required";
        assert_eq!(
            validate_tailoring(&source, job, "not JSON", 1).unwrap_err(),
            MaterialError::Invalid
        );
        let mut changed = source.clone();
        changed.sections[0].entries[0].bullets[0].text = "Increased revenue by 50%".into();
        let raw = json!({"schemaVersion":1,"resume":changed,"alerts":[]}).to_string();
        assert_eq!(
            validate_tailoring(&source, job, &raw, 1).unwrap_err(),
            MaterialError::Invalid
        );
    }

    #[test]
    fn compact_selection_copies_exact_published_facts() {
        let mut source = source();
        let entry = &mut source.sections[0].entries[0];
        entry.bullets.push(Bullet {
            id: EntityId::new(),
            order: 1,
            text: "Reviewed systems".into(),
        });
        let section_id = source.sections[0].id;
        let entry_id = source.sections[0].entries[0].id;
        let second_bullet = source.sections[0].entries[0].bullets[1].id;
        let raw = json!({"schemaVersion":1,"selectedSections":[{"sectionId":section_id,
            "entries":[{"entryId":entry_id,"bulletIds":[second_bullet]}]}],"alerts":[]})
        .to_string();
        let result = validate_tailoring(&source, "Rust required", &raw, 1).unwrap();
        assert_eq!(
            result.resume.sections[0].entries[0].bullets[0].text,
            "Reviewed systems"
        );
        assert_eq!(result.resume.sections[0].entries[0].bullets[0].order, 0);
        assert_eq!(result.resume.contact, source.contact);
        assert_eq!(result.change_points.len(), 1);
        let invalid = json!({"schemaVersion":1,"selectedSections":[{"sectionId":section_id,
            "entries":[{"entryId":entry_id,"bulletIds":[EntityId::new()]}]}],"alerts":[]})
        .to_string();
        assert_eq!(
            validate_tailoring(&source, "Rust required", &invalid, 1).unwrap_err(),
            MaterialError::Invalid
        );
    }

    #[test]
    fn required_alert_is_bounded_and_preferred_is_dropped() {
        let source = source();
        let alerts = vec![
            json!({"kind":"not_found","category":"named_skill_or_technology","requirement":"Python",
                "target":"Python","jobExcerpt":"Python required","resumeEvidence":null}),
            json!({"kind":"not_found","category":"named_skill_or_technology","requirement":"Go",
                "target":"Go","jobExcerpt":"Go preferred","resumeEvidence":null}),
        ];
        let raw = json!({"schemaVersion":1,"resume":source,"alerts":alerts}).to_string();
        let result = validate_tailoring(&source, "Python required; Go preferred", &raw, 1).unwrap();
        assert_eq!(result.alerts.len(), 1);
        assert_eq!(result.alerts[0].requirement, "Python");
    }

    #[test]
    fn alert_validation_respects_word_boundaries_and_claim_text() {
        let mut source = source();
        source.sections[0].entries[0].fields[0].value = "JavaScript".into();
        let job = "Java required; Python not required; TypeScript preferred; Language proficiency required";
        let candidates = vec![
            json!({"kind":"not_found","category":"named_skill_or_technology","requirement":"Java",
                "target":"Java","jobExcerpt":"Java required","resumeEvidence":null}),
            json!({"kind":"not_found","category":"named_skill_or_technology","requirement":"Python",
                "target":"Python","jobExcerpt":"Python not required","resumeEvidence":null}),
            json!({"kind":"not_found","category":"named_skill_or_technology","requirement":"TypeScript",
                "target":"TypeScript","jobExcerpt":"TypeScript preferred","resumeEvidence":null}),
            json!({"kind":"not_found","category":"named_skill_or_technology","requirement":"Kotlin",
                "target":"Java","jobExcerpt":"Java required","resumeEvidence":null}),
        ];
        let raw = json!({"schemaVersion":1,"resume":source,"alerts":candidates}).to_string();
        let validated = validate_tailoring(&source, job, &raw, 1).unwrap();
        assert_eq!(validated.alerts.len(), 1);
        assert_eq!(validated.alerts[0].requirement, "Java");
        assert!(!excluded_requirement("language proficiency required"));
        assert!(excluded_requirement("minimum age required"));
    }

    #[test]
    fn required_language_alert_uses_exact_job_span_and_published_absence() {
        let source = source();
        let job = "Spanish language required; French language preferred";
        let candidates = vec![
            json!({"kind":"not_found","category":"language_proficiency","requirement":"Spanish",
                "target":"Spanish","jobExcerpt":"Spanish language required","resumeEvidence":null}),
            json!({"kind":"not_found","category":"language_proficiency","requirement":"French",
                "target":"French","jobExcerpt":"French language preferred","resumeEvidence":null}),
        ];
        let raw = json!({"schemaVersion":1,"resume":source,"alerts":candidates}).to_string();
        let validated = validate_tailoring(&source, job, &raw, 1).unwrap();
        assert_eq!(validated.alerts.len(), 1);
        assert_eq!(validated.alerts[0].requirement, "Spanish");
        assert_eq!(
            (validated.alerts[0].job_start, validated.alerts[0].job_end),
            (0, 25)
        );
    }

    #[test]
    fn personal_questions_and_unsupported_evidence_are_rejected() {
        let source = source();
        assert!(question_requires_personal_answer(
            "Are you legally authorized to work here?"
        ));
        assert!(question_requires_personal_answer("What is your age?"));
        assert!(!question_requires_personal_answer(
            "Describe how you manage security engineering projects."
        ));
        assert_eq!(
            validate_answer("{}", &source, "Are you a citizen?", None).unwrap_err(),
            MaterialError::PersonalQuestion
        );
        let wrong = json!({"schemaVersion":1,"evidenceIds":[EntityId::new()]}).to_string();
        assert_eq!(
            validate_cover_letter(&wrong, &source, "Engineer required").unwrap_err(),
            MaterialError::Invalid
        );
        let evidence = source.sections[0].entries[0].bullets[0].id;
        let raw = json!({"schemaVersion":1,"evidenceIds":[evidence]}).to_string();
        let letter = validate_cover_letter(&raw, &source, "Engineer required").unwrap();
        assert!(letter.contains("Built reliable tools"));
        assert_eq!(
            validate_answer(&raw, &source, "Describe your work", Some(5)).unwrap_err(),
            MaterialError::Invalid
        );
    }

    #[test]
    fn confirmed_graduation_mismatch_requires_typed_source_evidence() {
        let mut source = source();
        source.sections[0].entries[0].fields.push(NamedField {
            id: EntityId::new(),
            order: 1,
            label: "Graduation year".into(),
            value: "2028".into(),
            is_skill: false,
        });
        let field_id = source.sections[0].entries[0].fields[1].id;
        let candidate = json!({"kind":"confirmed_mismatch","category":"graduation_date",
            "requirement":"Graduation in 2027", "target":"2027",
            "jobExcerpt":"Graduation in 2027 required", "resumeEvidence":{"fieldId":field_id,"value":"2028"}});
        let raw = json!({"schemaVersion":1,"resume":source,"alerts":[candidate]}).to_string();
        let valid = validate_tailoring(&source, "Graduation in 2027 required", &raw, 1).unwrap();
        assert_eq!(valid.alerts.len(), 1);
        assert_eq!(
            valid.alerts[0]
                .resume_evidence
                .as_ref()
                .map(|value| value.value.as_str()),
            Some("2028")
        );
        assert_eq!(
            (valid.alerts[0].job_start, valid.alerts[0].job_end),
            (0, 27)
        );
        let invalid = json!({"schemaVersion":1,"resume":source,"alerts":[{
            "kind":"confirmed_mismatch","category":"graduation_date",
            "requirement":"Graduation in 2027", "target":"2027",
            "jobExcerpt":"Graduation in 2027 required", "resumeEvidence":{"fieldId":field_id,"value":"2026"}
        }]}).to_string();
        let rejected =
            validate_tailoring(&source, "Graduation in 2027 required", &invalid, 1).unwrap();
        assert!(rejected.alerts.is_empty());
    }
}
