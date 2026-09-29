//! Body-only editing and reordering within the existing resume template.
//! Source references establish provenance, not proof of generated claims.
use std::collections::HashSet;

use ort_domain::{
    Bullet, DocumentLimits, EntityId, NamedField, ResumeDocument, ResumeEntry, ResumeSection,
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{AlertCandidate, MaterialError, RoleInfo, TailoredMaterial, validate_alerts};

const PARAGRAPH: &str = "__ort_body_paragraph__";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TemplateResponse {
    schema_version: u16,
    tailoring_plan: Vec<String>,
    role_info: Option<RoleInfo>,
    template_sections: Vec<TemplateSection>,
    alerts: Vec<AlertCandidate>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TemplateSection {
    section_id: EntityId,
    entries: Vec<TemplateEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TemplateEntry {
    entry_id: EntityId,
    source_entry_ids: Vec<EntityId>,
    main_info: MainInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MainInfo {
    format: BodyFormat,
    items: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum BodyFormat {
    Bullets,
    Paragraph,
}

fn date_text(entry: &ResumeEntry) -> String {
    std::iter::once(entry.date_range.clone())
        .chain(
            entry
                .dates
                .iter()
                .flatten()
                .map(ort_domain::ResumeDate::display_text),
        )
        .filter(|text| !text.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn details_text(entry: &ResumeEntry) -> String {
    entry
        .fields
        .iter()
        .filter(|field| {
            field.label != PARAGRAPH && !field.label.trim().eq_ignore_ascii_case("extra")
        })
        .map(|field| field.value.as_str())
        .collect::<Vec<_>>()
        .join(" | ")
}

fn extra_text(entry: &ResumeEntry) -> String {
    entry
        .fields
        .iter()
        .filter(|field| field.label.trim().eq_ignore_ascii_case("extra"))
        .map(|field| field.value.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Presents protected headers and editable bodies in the output vocabulary.
/// Keeps every source field and link available as evidence, including fields
/// not currently visible in the template. Internal ordering/UUID metadata for
/// bullets, dates, and links is not relevant to the model's editing task.
#[must_use]
pub fn resume_context(document: &ResumeDocument) -> Value {
    json!({
        "contact": document.contact,
        "sections": document.sections.iter().map(|section| json!({
            "sectionId": section.id,
            "heading": section.heading,
            "entries": section.entries.iter().map(|entry| {
                let paragraph = entry.fields.iter().find(|field| field.label == PARAGRAPH);
                json!({
                    "entryId": entry.id,
                    "title": entry.heading,
                    "role": entry.subheading,
                    "details": details_text(entry),
                    "date": date_text(entry),
                    "location": entry.location,
                    "extra": extra_text(entry),
                    "mainInfo": match paragraph.filter(|field| !field.value.trim().is_empty()) {
                        Some(field) => json!({"format":"paragraph","items":[field.value]}),
                        None => json!({"format":"bullets","items":entry.bullets.iter().map(|bullet| &bullet.text).collect::<Vec<_>>()})
                    },
                    "sourceFields": entry.fields.iter().map(|field| json!({"fieldId":field.id,"label":field.label,"value":field.value,"isSkill":field.is_skill})).collect::<Vec<_>>(),
                    "links": entry.links.iter().map(|link| json!({"label":link.label,"url":link.url})).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
    })
}

fn entries(document: &ResumeDocument) -> impl Iterator<Item = &ResumeEntry> {
    document
        .sections
        .iter()
        .flat_map(|section| &section.entries)
}

fn order(index: usize) -> Result<u16, MaterialError> {
    u16::try_from(index).map_err(|_| MaterialError::Invalid)
}

fn field(fields: &mut Vec<NamedField>, label: &str, value: &str) -> Result<(), MaterialError> {
    if !value.trim().is_empty() {
        fields.push(NamedField {
            id: EntityId::new(),
            order: order(fields.len())?,
            label: label.into(),
            value: value.trim().into(),
            is_skill: false,
        });
    }
    Ok(())
}

fn materialize_entry(
    draft: TemplateEntry,
    source: &ResumeDocument,
    section: &ResumeSection,
    index: usize,
    used_entries: &mut HashSet<EntityId>,
) -> Result<ResumeEntry, MaterialError> {
    let mut anchors = HashSet::new();
    if draft.source_entry_ids.is_empty()
        || draft
            .source_entry_ids
            .iter()
            .any(|id| !anchors.insert(*id) || !entries(source).any(|entry| entry.id == *id))
        || !used_entries.insert(draft.entry_id)
        || (entries(source).any(|entry| entry.id == draft.entry_id)
            && !anchors.contains(&draft.entry_id))
    {
        return Err(MaterialError::Invalid);
    }
    let original = section
        .entries
        .iter()
        .find(|entry| entry.id == draft.entry_id)
        .ok_or(MaterialError::Invalid)?;
    // Headers are read-only input. Never ask the model to reproduce them:
    // copy original fields, dates, links, and identities locally.
    let mut fields: Vec<_> = original
        .fields
        .iter()
        .filter(|field| field.label != PARAGRAPH)
        .cloned()
        .collect();
    for (index, field) in fields.iter_mut().enumerate() {
        field.order = order(index)?;
    }
    let mut bullets = Vec::new();
    if draft
        .main_info
        .items
        .iter()
        .any(|text| text.trim().is_empty())
    {
        return Err(MaterialError::Invalid);
    }
    match draft.main_info.format {
        BodyFormat::Paragraph => {
            field(&mut fields, PARAGRAPH, &draft.main_info.items.join("\n\n"))?;
        }
        BodyFormat::Bullets => {
            for (index, text) in draft.main_info.items.into_iter().enumerate() {
                bullets.push(Bullet {
                    id: EntityId::new(),
                    order: order(index)?,
                    text: text.trim().into(),
                });
            }
        }
    }
    let item = ResumeEntry {
        order: order(index)?,
        fields,
        bullets,
        ..original.clone()
    };
    if item.heading.is_empty()
        && item.subheading.is_empty()
        && item.fields.is_empty()
        && item.bullets.is_empty()
    {
        return Err(MaterialError::Invalid);
    }
    Ok(item)
}

pub(super) fn validate(
    source: &ResumeDocument,
    baseline: &ResumeDocument,
    job: &str,
    value: Value,
    published_revision: i64,
) -> Result<TailoredMaterial, MaterialError> {
    let proposed: TemplateResponse =
        serde_json::from_value(value).map_err(|_| MaterialError::Invalid)?;
    if proposed.schema_version != 5 {
        return Err(MaterialError::Version);
    }
    let change_points = validate_plan(proposed.tailoring_plan)?;
    let limits = DocumentLimits::default();
    baseline
        .validate(limits)
        .map_err(|_| MaterialError::Invalid)?;
    if baseline.document_id != source.document_id
        || baseline.schema_version != source.schema_version
        || proposed
            .role_info
            .as_ref()
            .is_some_and(|info| !info.valid())
        || proposed.template_sections.is_empty()
        || proposed.template_sections.len() > limits.sections
        || proposed.template_sections.len() > baseline.sections.len()
    {
        return Err(MaterialError::Invalid);
    }
    let mut resume = baseline.clone();
    resume.sections.clear();
    let mut used_sections = HashSet::new();
    let mut used_entries = HashSet::new();
    let mut entry_count = 0;
    for (index, section) in proposed.template_sections.into_iter().enumerate() {
        if !used_sections.insert(section.section_id) {
            return Err(MaterialError::Invalid);
        }
        let original = baseline
            .sections
            .iter()
            .find(|value| value.id == section.section_id)
            .ok_or(MaterialError::Invalid)?;
        if section.entries.len() > original.entries.len() {
            return Err(MaterialError::Invalid);
        }
        entry_count += section.entries.len();
        if entry_count > limits.entries {
            return Err(MaterialError::Invalid);
        }
        let entries = section
            .entries
            .into_iter()
            .enumerate()
            .map(|(index, entry)| {
                materialize_entry(entry, source, original, index, &mut used_entries)
            })
            .collect::<Result<Vec<_>, _>>()?;
        resume.sections.push(ResumeSection {
            id: section.section_id,
            order: order(index)?,
            heading: original.heading.clone(),
            entries,
        });
    }
    resume
        .validate(limits)
        .map_err(|_| MaterialError::Invalid)?;
    let (alerts, alerts_truncated) =
        validate_alerts(source, job, proposed.alerts, published_revision);
    Ok(TailoredMaterial {
        resume,
        role_info: proposed.role_info,
        change_points,
        alerts,
        alerts_truncated,
    })
}

fn validate_plan(plan: Vec<String>) -> Result<Vec<String>, MaterialError> {
    if !(1..=3).contains(&plan.len()) {
        return Err(MaterialError::Invalid);
    }
    let mut unique = HashSet::new();
    plan.into_iter()
        .map(|point| {
            let trimmed = point.trim();
            let normalized = trimmed
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            if trimmed.is_empty()
                || trimmed.chars().count() > 500
                || trimmed.chars().any(char::is_control)
                || !unique.insert(normalized)
            {
                return Err(MaterialError::Invalid);
            }
            Ok(trimmed.to_owned())
        })
        .collect()
}

fn object(properties: &Value) -> Value {
    let required = properties
        .as_object()
        .expect("schema properties")
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    json!({"type":"object","additionalProperties":false,"required":required,"properties":properties})
}

/// Strict provider schema. Content is free text; layout is always app-owned.
#[must_use]
pub fn resume_output_schema() -> Value {
    let text = json!({"type":"string"});
    let id = json!({"type":"string"});
    let entry = object(&json!({
        "entryId":id,"sourceEntryIds":{"type":"array","items":text},
        "mainInfo":object(&json!({"format":{"type":"string","enum":["bullets","paragraph"]},"items":{"type":"array","items":text}}))
    }));
    let section = object(&json!({"sectionId":id,"entries":{"type":"array","items":entry}}));
    let role = object(&json!({"company":text,"title":text,"location":text}));
    let evidence = object(&json!({"fieldId":text,"value":text}));
    let alert = object(&json!({
        "kind":{"type":"string","enum":["not_found","confirmed_mismatch"]},
        "category":{"type":"string","enum":["degree_level","field_of_study","graduation_date","certification_or_professional_license","named_skill_or_technology","language_proficiency","experience_duration","portfolio_or_work_sample"]},
        "requirement":text,"target":text,"jobExcerpt":text,"resumeEvidence":{"anyOf":[evidence,{"type":"null"}]}
    }));
    object(
        &json!({"schemaVersion":{"type":"integer","enum":[5]},"tailoringPlan":{"type":"array","minItems":1,"maxItems":3,"items":{"type":"string","minLength":1,"maxLength":500}},"roleInfo":{"anyOf":[role,{"type":"null"}]},"templateSections":{"type":"array","items":section},"alerts":{"type":"array","maxItems":10,"items":alert}}),
    )
}

/// Gemini supports the shared JSON schema except string length constraints;
/// keep those checks local and omit unsupported provider keywords.
#[must_use]
pub fn gemini_resume_output_schema() -> Value {
    fn supported(value: &mut Value) {
        match value {
            Value::Object(object) => {
                object.remove("minLength");
                object.remove("maxLength");
                for child in object.values_mut() {
                    supported(child);
                }
            }
            Value::Array(items) => {
                for child in items {
                    supported(child);
                }
            }
            _ => {}
        }
    }
    let mut schema = resume_output_schema();
    supported(&mut schema);
    schema
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::{validate_refinement, validate_tailoring};
    use ort_domain::{CalendarDate, DateEnd, Link, ResumeDate};

    fn source() -> ResumeDocument {
        let mut source = ResumeDocument::empty("Master");
        source.contact.full_name = "Alex Rivera".into();
        source.sections.push(ResumeSection {
            id: EntityId::new(),
            order: 0,
            heading: "Experience".into(),
            entries: vec![ResumeEntry {
                id: EntityId::new(),
                order: 0,
                heading: "North Co".into(),
                subheading: "Engineer".into(),
                date_range: "2021–2024".into(),
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
                    text: "Built Rust tools and maintained SQL reports for the support team."
                        .into(),
                }],
                links: vec![Link {
                    id: None,
                    order: None,
                    label: "Work sample".into(),
                    url: "https://example.com/tools".into(),
                }],
            }],
        });
        source
    }

    fn proposal(source: &ResumeDocument) -> Value {
        let context = resume_context(source);
        json!({"schemaVersion":5,"tailoringPlan":[
            "Emphasize tooling needs by leading with the published Rust support-tool work.",
            "Make the SQL reporting experience easier to find in a specific body bullet.",
            "Keep the published experience context while condensing repeated body details."
        ],"roleInfo":null,"alerts":[],"templateSections":context["sections"].as_array().unwrap().iter().map(|section| {
            json!({"sectionId":section["sectionId"],
                "entries":section["entries"].as_array().unwrap().iter().map(|entry| {
                    json!({"entryId":entry["entryId"],"sourceEntryIds":[entry["entryId"]],
                        "mainInfo":{"format":"bullets","items":["Built Rust tools for the support team.","Maintained SQL reports to support the team's work."]}})
                }).collect::<Vec<_>>()})
        }).collect::<Vec<_>>()})
    }

    #[test]
    fn plan_is_required_bounded_and_contains_one_to_three_distinct_points() {
        let source = source();
        let valid = proposal(&source);
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove("tailoringPlan");
        assert!(validate_tailoring(&source, "Engineer", &missing.to_string(), 1).is_err());
        for invalid in [
            json!([]),
            json!(["One", "Two", "Three", "Four"]),
            json!(["One", " ", "Three"]),
            json!(["One", " one ", "Three"]),
            json!(["A point", "A  point", "Three"]),
            json!(["One", "Two", "x".repeat(501)]),
            json!(["One", "Two", "Hidden\nparagraph"]),
            json!("not an array"),
            Value::Null,
        ] {
            let mut draft = valid.clone();
            draft["tailoringPlan"] = invalid;
            assert!(validate_tailoring(&source, "Engineer", &draft.to_string(), 1).is_err());
            assert!(
                validate_refinement(&source, &source, "Engineer", &draft.to_string(), 1).is_err()
            );
        }
        for count in 1..=3 {
            let mut draft = valid.clone();
            draft["tailoringPlan"]
                .as_array_mut()
                .unwrap()
                .truncate(count);
            let result = validate_tailoring(&source, "Engineer", &draft.to_string(), 1).unwrap();
            assert_eq!(result.change_points.len(), count);
            assert!(
                validate_refinement(&source, &source, "Engineer", &draft.to_string(), 1).is_ok()
            );
        }
        let mut padded = valid.clone();
        padded["tailoringPlan"][0] = json!(format!(
            "  {}  ",
            valid["tailoringPlan"][0].as_str().unwrap()
        ));
        let result = validate_tailoring(&source, "Engineer", &padded.to_string(), 1).unwrap();
        assert_eq!(json!(result.change_points), valid["tailoringPlan"]);
        let refined =
            validate_refinement(&source, &result.resume, "Engineer", &valid.to_string(), 1)
                .unwrap();
        assert_eq!(json!(refined.change_points), valid["tailoringPlan"]);
    }

    #[test]
    fn planned_draft_retains_job_info_and_checks_alerts_against_entire_master() {
        let source = source();
        let mut draft = proposal(&source);
        draft["roleInfo"] =
            json!({"company":"Example Co","title":"Reporting Engineer","location":"Remote"});
        // Rust was intentionally omitted from this output, but it remains in
        // the full master and must not become a missing-qualification alert.
        let entry = &mut draft["templateSections"][0]["entries"][0];
        entry["mainInfo"] =
            json!({"format":"bullets","items":["Maintained SQL reports for the support team."]});
        draft["alerts"] = json!([
            {"kind":"not_found","category":"named_skill_or_technology","requirement":"Rust","target":"Rust","jobExcerpt":"Rust required","resumeEvidence":null},
            {"kind":"not_found","category":"named_skill_or_technology","requirement":"Python","target":"Python","jobExcerpt":"Python required","resumeEvidence":null},
            {"kind":"not_found","category":"named_skill_or_technology","requirement":"Go","target":"Go","jobExcerpt":"Go preferred","resumeEvidence":null}
        ]);
        let result = validate_tailoring(
            &source,
            "Rust required; Python required; Go preferred",
            &draft.to_string(),
            1,
        )
        .unwrap();
        assert_eq!(result.change_points.len(), 3);
        let role = result.role_info.unwrap();
        assert_eq!(
            (
                role.company.as_str(),
                role.title.as_str(),
                role.location.as_str()
            ),
            ("Example Co", "Reporting Engineer", "Remote")
        );
        assert_eq!(result.alerts.len(), 1);
        assert_eq!(result.alerts[0].requirement, "Python");
        assert_eq!(result.resume.sections[0].entries[0].bullets.len(), 1);
    }

    #[test]
    fn body_edit_preserves_protected_fields_and_identity() {
        let source = source();
        let result = validate_tailoring(
            &source,
            "Support tools engineer",
            &proposal(&source).to_string(),
            1,
        )
        .unwrap();
        let entry = &result.resume.sections[0].entries[0];
        assert_eq!(entry.heading, "North Co");
        assert_eq!(entry.subheading, "Engineer");
        assert_eq!(entry.fields, source.sections[0].entries[0].fields);
        assert_eq!(entry.date_range, "2021–2024");
        assert_eq!(entry.location, "Boston");
        assert_eq!(entry.bullets.len(), 2);
        assert_eq!(entry.id, source.sections[0].entries[0].id);
        assert_eq!(entry.links, source.sections[0].entries[0].links);
        assert_eq!(result.resume.contact, source.contact);
        assert_eq!(result.resume.document_id, source.document_id);
        assert_eq!(result.resume.schema_version, source.schema_version);
    }

    #[test]
    fn new_sections_entries_and_renamed_headings_are_rejected() {
        let source = source();
        for (target, key, value) in [
            ("section", "sectionId", Value::Null),
            ("section", "sectionId", json!(EntityId::new())),
            ("section", "heading", json!("Profile")),
            ("entry", "entryId", Value::Null),
            ("entry", "entryId", json!(EntityId::new())),
        ] {
            let mut draft = proposal(&source);
            let section = &mut draft["templateSections"][0];
            if target == "section" {
                section[key] = value;
            } else {
                section["entries"][0][key] = value;
            }
            assert!(validate_tailoring(&source, "Engineer", &draft.to_string(), 1).is_err());
            assert!(
                validate_refinement(&source, &source, "Engineer", &draft.to_string(), 1).is_err()
            );
        }
    }

    #[test]
    fn multiple_details_and_extra_fields_keep_labels_flags_ids_and_whitespace() {
        let mut source = source();
        let entry = &mut source.sections[0].entries[0];
        entry.heading = "  North Co  ".into();
        entry.fields.extend([
            NamedField {
                id: EntityId::new(),
                order: 1,
                label: "Focus".into(),
                value: "  Support tooling  ".into(),
                is_skill: false,
            },
            NamedField {
                id: EntityId::new(),
                order: 2,
                label: " Extra ".into(),
                value: "Rust, SQL".into(),
                is_skill: true,
            },
            NamedField {
                id: EntityId::new(),
                order: 3,
                label: "EXTRA".into(),
                value: "  Internal platform  ".into(),
                is_skill: false,
            },
        ]);
        let draft = proposal(&source);
        let result = validate_tailoring(&source, "Engineer", &draft.to_string(), 1)
            .unwrap()
            .resume;
        assert_eq!(
            result.sections[0].entries[0].fields,
            source.sections[0].entries[0].fields
        );
        assert_eq!(
            result.sections[0].entries[0].heading,
            source.sections[0].entries[0].heading
        );
        let mut normalized = draft;
        normalized["templateSections"][0]["entries"][0]["title"] = json!("North Co");
        assert!(validate_tailoring(&source, "Engineer", &normalized.to_string(), 1).is_err());
    }

    #[test]
    fn protected_regions_are_not_accepted_in_provider_output() {
        let source = source();
        for slot in ["title", "role", "details", "date", "location", "extra"] {
            for changed in ["Changed", "", " "] {
                let mut draft = proposal(&source);
                draft["templateSections"][0]["entries"][0][slot] = json!(changed);
                assert!(
                    validate_tailoring(&source, "Engineer", &draft.to_string(), 1).is_err(),
                    "{slot}"
                );
                assert!(
                    validate_refinement(&source, &source, "Engineer", &draft.to_string(), 1)
                        .is_err(),
                    "{slot}"
                );
            }
        }
    }

    #[test]
    fn refinements_keep_current_only_ids_and_user_reviewed_header_fields() {
        let source = source().upgraded_v2().unwrap();
        let mut current = source.clone();
        current.sections[0].id = EntityId::new();
        current.contact.phone = "555-0100".into();
        let entry = &mut current.sections[0].entries[0];
        entry.id = EntityId::new();
        entry.heading = "Reviewed title".into();
        entry.fields[0].value = "Reviewed skills".into();
        entry.bullets[0].text = "Reviewed wording about Rust tools.".into();
        entry.links[0].url = "https://example.com/reviewed".into();
        let mut next = proposal(&current);
        next["templateSections"][0]["entries"][0]["sourceEntryIds"] =
            json!([source.sections[0].entries[0].id]);
        next["templateSections"][0]["entries"][0]["mainInfo"] =
            resume_context(&current)["sections"][0]["entries"][0]["mainInfo"].clone();
        let result = validate_refinement(&source, &current, "Engineer", &next.to_string(), 1)
            .unwrap()
            .resume;
        let mut expected = current.clone();
        // Body bullet identities are regenerated; protected identities are retained.
        for (actual, expected) in result.sections[0].entries[0]
            .bullets
            .iter()
            .zip(&mut expected.sections[0].entries[0].bullets)
        {
            expected.id = actual.id;
        }
        assert_eq!(result, expected);
        assert!(validate_tailoring(&source, "Engineer", &next.to_string(), 1).is_err());
        // The published header cannot overwrite the user's reviewed header.
        next["templateSections"][0]["entries"][0]["title"] =
            json!(source.sections[0].entries[0].heading);
        assert!(validate_refinement(&source, &current, "Engineer", &next.to_string(), 1).is_err());
    }

    #[test]
    fn sections_and_entries_can_be_reordered_or_removed_but_not_transplanted() {
        let mut document = source();
        let second_entry = source().sections.remove(0).entries.remove(0);
        document.sections[0].entries.push(ResumeEntry {
            order: 1,
            ..second_entry
        });
        let mut education = source().sections.remove(0);
        education.order = 1;
        education.heading = "Education".into();
        document.sections.push(education);
        let mut draft = proposal(&document);
        draft["templateSections"].as_array_mut().unwrap().reverse();
        draft["templateSections"][1]["entries"]
            .as_array_mut()
            .unwrap()
            .reverse();
        for result in [
            validate_tailoring(&document, "Engineer", &draft.to_string(), 1).unwrap(),
            validate_refinement(&document, &document, "Engineer", &draft.to_string(), 1).unwrap(),
        ] {
            assert_eq!(result.resume.sections[0].id, document.sections[1].id);
            assert_eq!(
                result.resume.sections[1].entries[0].id,
                document.sections[0].entries[1].id
            );
            assert_eq!(result.resume.sections[0].order, 0);
            assert_eq!(result.resume.sections[1].entries[0].order, 0);
        }
        let mut transplanted = draft.clone();
        let moved = transplanted["templateSections"][1]["entries"][0].clone();
        let previous = transplanted["templateSections"][0]["entries"][0].clone();
        transplanted["templateSections"][0]["entries"][0] = moved;
        transplanted["templateSections"][1]["entries"][0] = previous;
        assert!(validate_tailoring(&document, "Engineer", &transplanted.to_string(), 1).is_err());
        draft["templateSections"].as_array_mut().unwrap().remove(0);
        draft["templateSections"][0]["entries"]
            .as_array_mut()
            .unwrap()
            .remove(0);
        for result in [
            validate_tailoring(&document, "Engineer", &draft.to_string(), 1).unwrap(),
            validate_refinement(&document, &document, "Engineer", &draft.to_string(), 1).unwrap(),
        ] {
            assert_eq!(result.resume.sections.len(), 1);
            assert_eq!(result.resume.sections[0].entries.len(), 1);
        }
    }

    #[test]
    fn body_can_switch_between_paragraphs_and_bullets_without_altering_skills() {
        let source = source();
        let mut draft = proposal(&source);
        draft["templateSections"][0]["entries"][0]["mainInfo"] =
            json!({"format":"paragraph","items":["Built Rust tools.","Maintained SQL reports."]});
        let paragraph = validate_tailoring(&source, "Engineer", &draft.to_string(), 1)
            .unwrap()
            .resume;
        let entry = &paragraph.sections[0].entries[0];
        assert_eq!(entry.fields[0], source.sections[0].entries[0].fields[0]);
        assert_eq!(entry.fields[1].label, PARAGRAPH);
        assert_eq!(
            entry.fields[1].value,
            "Built Rust tools.\n\nMaintained SQL reports."
        );
        assert!(entry.bullets.is_empty());
        let bullets = validate_refinement(
            &source,
            &paragraph,
            "Engineer",
            &proposal(&paragraph).to_string(),
            1,
        )
        .unwrap()
        .resume;
        assert_eq!(
            bullets.sections[0].entries[0].fields,
            source.sections[0].entries[0].fields
        );
        assert_eq!(bullets.sections[0].entries[0].bullets.len(), 2);
    }

    #[test]
    fn overlay_contract_rejects_older_schemas_that_allow_header_field_removal() {
        let source = source();
        let draft = json!({"schemaVersion":2,"draftedSections":[{"sectionId":source.sections[0].id,
            "entries":[{"entryId":source.sections[0].entries[0].id,"fieldIds":[],"bullets":[{"text":"Built tools."}]}]}],"alerts":[]});
        assert!(
            crate::materials::validate_template_tailoring(
                &source,
                "Engineer",
                &draft.to_string(),
                1
            )
            .is_err()
        );
        assert!(
            crate::materials::validate_template_tailoring(
                &source,
                "Engineer",
                &proposal(&source).to_string(),
                1
            )
            .is_ok()
        );
    }

    #[test]
    fn structured_dates_keep_precision_and_reject_edits() {
        let mut source = source().upgraded_v2().unwrap();
        let entry = &mut source.sections[0].entries[0];
        entry.date_range.clear();
        entry.dates = Some(vec![ResumeDate {
            id: EntityId::new(),
            order: 0,
            label: String::new(),
            start: Some(CalendarDate {
                year: 2021,
                month: None,
                expected: false,
            }),
            end: Some(DateEnd::Present),
        }]);
        let mut draft = proposal(&source);
        let result = validate_tailoring(&source, "Engineer", &draft.to_string(), 1).unwrap();
        assert_eq!(
            result.resume.sections[0].entries[0].dates,
            source.sections[0].entries[0].dates
        );
        assert!(result.resume.sections[0].entries[0].date_range.is_empty());
        draft["templateSections"][0]["entries"][0]["date"] = json!("2021 – Present");
        assert!(validate_tailoring(&source, "Engineer", &draft.to_string(), 1).is_err());
        assert!(validate_refinement(&source, &source, "Engineer", &draft.to_string(), 1).is_err());
    }

    #[test]
    fn refinement_preserves_reviewed_links_and_typed_dates() {
        let source = source().upgraded_v2().unwrap();
        let mut current = source.clone();
        let entry = &mut current.sections[0].entries[0];
        entry.date_range.clear();
        entry.dates = Some(vec![ResumeDate {
            id: EntityId::new(),
            order: 0,
            label: String::new(),
            start: Some(CalendarDate {
                year: 2021,
                month: None,
                expected: false,
            }),
            end: Some(DateEnd::Present),
        }]);
        entry.links[0].url = "https://example.com/reviewed-work".into();
        let mut draft = proposal(&current);
        let result =
            validate_refinement(&source, &current, "Engineer", &draft.to_string(), 1).unwrap();
        let result_entry = &result.resume.sections[0].entries[0];
        assert_eq!(result_entry.id, current.sections[0].entries[0].id);
        assert_eq!(result_entry.dates, current.sections[0].entries[0].dates);
        assert_eq!(result_entry.links, current.sections[0].entries[0].links);
        // Repeating that identity still cannot duplicate an experience.
        let duplicate = draft["templateSections"][0]["entries"][0].clone();
        draft["templateSections"][0]["entries"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(validate_refinement(&source, &current, "Engineer", &draft.to_string(), 1).is_err());
    }

    #[test]
    fn unknown_or_duplicate_identities_and_ungrounded_entries_are_rejected() {
        let source = source();
        let valid = proposal(&source);
        let mut cases = Vec::new();
        for key in ["entryId", "sourceEntryIds"] {
            let mut draft = valid.clone();
            draft["templateSections"][0]["entries"][0][key] = if key == "entryId" {
                json!(EntityId::new())
            } else {
                json!([EntityId::new()])
            };
            cases.push(draft);
        }
        let mut empty = valid.clone();
        empty["templateSections"][0]["entries"][0]["sourceEntryIds"] = json!([]);
        cases.push(empty);
        let mut duplicate = valid.clone();
        let entry = duplicate["templateSections"][0]["entries"][0].clone();
        duplicate["templateSections"][0]["entries"]
            .as_array_mut()
            .unwrap()
            .push(entry);
        cases.push(duplicate);
        let mut duplicate_section = valid.clone();
        let section = duplicate_section["templateSections"][0].clone();
        duplicate_section["templateSections"]
            .as_array_mut()
            .unwrap()
            .push(section);
        cases.push(duplicate_section);
        let mut new_layout = valid.clone();
        new_layout["templateSections"][0]["entries"][0]["html"] =
            json!("<table>new layout</table>");
        cases.push(new_layout);
        let mut missing_slot = valid.clone();
        missing_slot["templateSections"][0]["entries"][0]
            .as_object_mut()
            .unwrap()
            .remove("mainInfo");
        cases.push(missing_slot);
        for draft in cases {
            assert!(validate_tailoring(&source, "Engineer", &draft.to_string(), 1).is_err());
        }
    }

    #[test]
    fn output_limits_apply_to_all_regions_and_body_modes() {
        let source = source();
        for slot in ["title", "role", "details", "date", "location", "extra"] {
            let mut draft = proposal(&source);
            draft["templateSections"][0]["entries"][0][slot] = json!("x".repeat(2001));
            assert!(
                validate_tailoring(&source, "Engineer", &draft.to_string(), 1).is_err(),
                "{slot}"
            );
        }
        for (format, count) in [("bullets", 501), ("paragraph", 2001)] {
            let mut draft = proposal(&source);
            draft["templateSections"][0]["entries"][0]["mainInfo"] =
                json!({"format":format,"items":["x".repeat(count)]});
            assert!(validate_tailoring(&source, "Engineer", &draft.to_string(), 1).is_err());
        }
        let mut empty = proposal(&source);
        empty["templateSections"] = json!([]);
        assert!(validate_tailoring(&source, "Engineer", &empty.to_string(), 1).is_err());
    }

    #[test]
    fn provider_schema_is_strict_and_matches_the_validated_fixture() {
        fn check(value: &Value, schema: &Value) {
            if let Some(branches) = schema.get("anyOf").and_then(Value::as_array) {
                let branch = branches
                    .iter()
                    .find(|branch| (branch["type"] == "null") == value.is_null())
                    .unwrap();
                check(value, branch);
            } else if schema["type"] == "object" {
                let properties = schema["properties"].as_object().unwrap();
                assert_eq!(schema["additionalProperties"], false);
                assert_eq!(
                    schema["required"].as_array().unwrap().len(),
                    properties.len()
                );
                assert_eq!(value.as_object().unwrap().len(), properties.len());
                for (key, child) in properties {
                    check(&value[key], child);
                }
            } else if schema["type"] == "array" {
                for child in value.as_array().unwrap() {
                    check(child, &schema["items"]);
                }
            } else if let Some(variants) = schema["enum"].as_array() {
                assert!(variants.contains(value));
            } else if schema["type"] == "string" {
                assert!(value.is_string());
            }
        }
        check(&proposal(&source()), &resume_output_schema());
        let schema = resume_output_schema();
        let plan = &schema["properties"]["tailoringPlan"];
        assert_eq!(plan["minItems"], 1);
        assert_eq!(plan["maxItems"], 3);
        assert_eq!(plan["items"]["maxLength"], 500);
        assert_eq!(schema["properties"]["schemaVersion"]["enum"], json!([5]));
    }
}
