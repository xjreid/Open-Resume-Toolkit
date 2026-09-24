//! Editorial freedom within the existing seven-region resume template.
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
    section_id: Option<EntityId>,
    heading: String,
    entries: Vec<TemplateEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TemplateEntry {
    entry_id: Option<EntityId>,
    source_entry_ids: Vec<EntityId>,
    title: String,
    role: String,
    details: String,
    date: String,
    location: String,
    extra: String,
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

/// Presents editable regions in the same vocabulary as the output contract.
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
                    "details": entry.fields.iter().filter(|field| field.label != PARAGRAPH && !field.label.trim().eq_ignore_ascii_case("extra")).map(|field| field.value.as_str()).collect::<Vec<_>>().join(" | "),
                    "date": date_text(entry),
                    "location": entry.location,
                    "extra": entry.fields.iter().filter(|field| field.label.trim().eq_ignore_ascii_case("extra")).map(|field| field.value.as_str()).collect::<Vec<_>>().join("\n"),
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

// Models sometimes return null for a retained entry. Recover an unambiguous
// unchanged identity so that doing so cannot silently drop its links or typed
// dates. A genuinely derived summary/skills entry has a different identity.
fn retained_id(
    draft: &TemplateEntry,
    source: &ResumeDocument,
    baseline: &ResumeDocument,
) -> Result<Option<EntityId>, MaterialError> {
    if draft.entry_id.is_some() || (draft.title.trim().is_empty() && draft.role.trim().is_empty()) {
        return Ok(draft.entry_id);
    }
    let candidates: HashSet<_> = entries(baseline)
        .chain(entries(source))
        .filter(|entry| {
            entry.heading.trim() == draft.title.trim()
                && entry.subheading.trim() == draft.role.trim()
                && entry.location.trim() == draft.location.trim()
                && date_text(entry).trim() == draft.date.trim()
        })
        .map(|entry| entry.id)
        .collect();
    if candidates.len() > 1 {
        return Err(MaterialError::Invalid);
    }
    Ok(candidates.into_iter().next())
}

fn materialize_entry(
    draft: TemplateEntry,
    source: &ResumeDocument,
    baseline: &ResumeDocument,
    index: usize,
    used_entries: &mut HashSet<EntityId>,
) -> Result<ResumeEntry, MaterialError> {
    let mut anchors = HashSet::new();
    if draft.source_entry_ids.is_empty()
        || draft
            .source_entry_ids
            .iter()
            .any(|id| !anchors.insert(*id) || !entries(source).any(|entry| entry.id == *id))
    {
        return Err(MaterialError::Invalid);
    }
    let entry_id = retained_id(&draft, source, baseline)?;
    let original = match entry_id {
        Some(id) => {
            if !used_entries.insert(id)
                || (entries(source).any(|entry| entry.id == id) && !anchors.contains(&id))
            {
                return Err(MaterialError::Invalid);
            }
            Some(
                entries(baseline)
                    .chain(entries(source))
                    .find(|entry| entry.id == id)
                    .ok_or(MaterialError::Invalid)?,
            )
        }
        None => None,
    };
    let mut fields = Vec::new();
    field(&mut fields, "Details", &draft.details)?;
    field(&mut fields, "Extra", &draft.extra)?;
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
    // Retain structured date identities/precision if the displayed date is
    // unchanged. An intentional edit occupies the existing free-text date slot.
    let unchanged_date = original.filter(|entry| date_text(entry) == draft.date.trim());
    let item = ResumeEntry {
        id: entry_id.unwrap_or_default(),
        order: order(index)?,
        heading: draft.title.trim().into(),
        subheading: draft.role.trim().into(),
        date_range: unchanged_date.map_or_else(
            || draft.date.trim().into(),
            |entry| entry.date_range.clone(),
        ),
        dates: if baseline.schema_version == 2 {
            Some(
                unchanged_date
                    .and_then(|entry| entry.dates.clone())
                    .unwrap_or_default(),
            )
        } else {
            None
        },
        location: draft.location.trim().into(),
        fields,
        bullets,
        links: original.map_or_else(Vec::new, |entry| entry.links.clone()),
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
    if proposed.schema_version != 4 {
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
    {
        return Err(MaterialError::Invalid);
    }
    let mut resume = baseline.clone();
    resume.sections.clear();
    let mut used_sections = HashSet::new();
    let mut used_entries = HashSet::new();
    let mut entry_count = 0;
    for (index, section) in proposed.template_sections.into_iter().enumerate() {
        if let Some(id) = section.section_id
            && (!used_sections.insert(id)
                || !baseline
                    .sections
                    .iter()
                    .chain(&source.sections)
                    .any(|section| section.id == id))
        {
            return Err(MaterialError::Invalid);
        }
        entry_count += section.entries.len();
        if section.entries.is_empty() || entry_count > limits.entries {
            return Err(MaterialError::Invalid);
        }
        let entries = section
            .entries
            .into_iter()
            .enumerate()
            .map(|(index, entry)| {
                materialize_entry(entry, source, baseline, index, &mut used_entries)
            })
            .collect::<Result<Vec<_>, _>>()?;
        resume.sections.push(ResumeSection {
            id: section.section_id.unwrap_or_default(),
            order: order(index)?,
            heading: section.heading.trim().into(),
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
    if plan.len() != 3 {
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
    let id = json!({"type":["string","null"]});
    let entry = object(&json!({
        "entryId":id,"sourceEntryIds":{"type":"array","items":text},
        "title":text,"role":text,"details":text,"date":text,"location":text,"extra":text,
        "mainInfo":object(&json!({"format":{"type":"string","enum":["bullets","paragraph"]},"items":{"type":"array","items":text}}))
    }));
    let section =
        object(&json!({"sectionId":id,"heading":text,"entries":{"type":"array","items":entry}}));
    let role = object(&json!({"company":text,"title":text,"location":text}));
    let evidence = object(&json!({"fieldId":text,"value":text}));
    let alert = object(&json!({
        "kind":{"type":"string","enum":["not_found","confirmed_mismatch"]},
        "category":{"type":"string","enum":["degree_level","field_of_study","graduation_date","certification_or_professional_license","named_skill_or_technology","language_proficiency","experience_duration","portfolio_or_work_sample"]},
        "requirement":text,"target":text,"jobExcerpt":text,"resumeEvidence":{"anyOf":[evidence,{"type":"null"}]}
    }));
    object(
        &json!({"schemaVersion":{"type":"integer","enum":[4]},"tailoringPlan":{"type":"array","minItems":3,"maxItems":3,"items":{"type":"string","minLength":1,"maxLength":500}},"roleInfo":{"anyOf":[role,{"type":"null"}]},"templateSections":{"type":"array","items":section},"alerts":{"type":"array","items":alert}}),
    )
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
        let entry = &source.sections[0].entries[0];
        json!({"schemaVersion":4,"tailoringPlan":[
            "Emphasize the role's tooling needs by leading with the published Rust support-tool work.",
            "Give SQL reporting its own specific bullet to make the reporting experience easier to find.",
            "Group the supported Rust and SQL skills beside the title and omit repeated technology lists."
        ],"roleInfo":null,"alerts":[],"templateSections":[{
            "sectionId":source.sections[0].id,"heading":"Relevant Experience","entries":[{
                "entryId":entry.id,"sourceEntryIds":[entry.id],"title":"North Co","role":"Engineer",
                "details":"Rust | SQL","date":date_text(entry),"location":"Boston","extra":"Support tools",
                "mainInfo":{"format":"bullets","items":["Built Rust tools for the support team.","Maintained SQL reports to support the team's work."]}
            }]
        }]})
    }

    #[test]
    fn plan_is_required_bounded_and_contains_three_distinct_points() {
        let source = source();
        let valid = proposal(&source);
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove("tailoringPlan");
        assert!(validate_tailoring(&source, "Engineer", &missing.to_string(), 1).is_err());
        for invalid in [
            json!([]),
            json!(["One", "Two"]),
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
        entry["details"] = json!("SQL");
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
    fn seven_regions_materialize_without_changing_template_or_identity() {
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
        assert_eq!(
            entry
                .fields
                .iter()
                .map(|field| (field.label.as_str(), field.value.as_str()))
                .collect::<Vec<_>>(),
            vec![("Details", "Rust | SQL"), ("Extra", "Support tools")]
        );
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
    fn derived_summary_and_regrouped_entries_fit_existing_template() {
        let source = source();
        let mut draft = proposal(&source);
        let mut summary = draft["templateSections"][0].clone();
        summary["sectionId"] = Value::Null;
        summary["heading"] = json!("Profile");
        let entry = &mut summary["entries"][0];
        entry["entryId"] = Value::Null;
        for slot in ["title", "role", "details", "date", "location", "extra"] {
            entry[slot] = json!("");
        }
        entry["mainInfo"] = json!({"format":"paragraph","items":["Engineer with experience building Rust tools and maintaining SQL reports for support teams."]});
        draft["templateSections"]
            .as_array_mut()
            .unwrap()
            .insert(0, summary);
        // A retained experience may move into a new section.
        draft["templateSections"][1]["sectionId"] = Value::Null;
        let result =
            validate_tailoring(&source, "Support engineer", &draft.to_string(), 1).unwrap();
        assert_eq!(result.resume.sections.len(), 2);
        let summary = &result.resume.sections[0].entries[0];
        assert!(summary.bullets.is_empty());
        assert_eq!(summary.fields[0].label, PARAGRAPH);
        assert!(summary.links.is_empty());
        assert_ne!(summary.id, source.sections[0].entries[0].id);
        assert_eq!(
            result.resume.sections[1].entries[0].id,
            source.sections[0].entries[0].id
        );
    }

    #[test]
    fn refinements_keep_current_only_ids_dates_contact_links_and_reviewed_text() {
        let source = source().upgraded_v2().unwrap();
        let mut first = proposal(&source);
        first["templateSections"][0]["sectionId"] = Value::Null;
        first["templateSections"][0]["entries"][0]["entryId"] = Value::Null;
        first["templateSections"][0]["entries"][0]["title"] = json!("Tools and reporting");
        first["templateSections"][0]["entries"][0]["role"] = json!("");
        let mut current = validate_tailoring(&source, "Support engineer", &first.to_string(), 1)
            .unwrap()
            .resume;
        current.contact.phone = "555-0100".into();
        let entry = &mut current.sections[0].entries[0];
        entry.heading = "Reviewed title".into();
        entry.bullets[0].text = "Reviewed, specific wording about Rust tools.".into();
        entry.links = source.sections[0].entries[0].links.clone();
        let context = resume_context(&current);
        let mut next = proposal(&source);
        next["templateSections"][0]["sectionId"] = json!(current.sections[0].id);
        let next_entry = &mut next["templateSections"][0]["entries"][0];
        next_entry["entryId"] = json!(current.sections[0].entries[0].id);
        next_entry["title"] = context["sections"][0]["entries"][0]["title"].clone();
        next_entry["mainInfo"] = context["sections"][0]["entries"][0]["mainInfo"].clone();
        next_entry["extra"] = json!("Tools and reporting");
        let result =
            validate_refinement(&source, &current, "Support engineer", &next.to_string(), 1)
                .unwrap()
                .resume;
        assert_eq!(result.sections[0].id, current.sections[0].id);
        assert_eq!(
            result.sections[0].entries[0].id,
            current.sections[0].entries[0].id
        );
        assert_eq!(result.sections[0].entries[0].heading, "Reviewed title");
        assert_eq!(
            result.sections[0].entries[0].bullets[0].text,
            current.sections[0].entries[0].bullets[0].text
        );
        assert_eq!(
            result.sections[0].entries[0].links,
            current.sections[0].entries[0].links
        );
        assert_eq!(result.contact, current.contact);
        // Those IDs are valid only when refining that reviewed document.
        assert!(validate_tailoring(&source, "Support engineer", &next.to_string(), 1).is_err());
    }

    #[test]
    fn unchanged_structured_dates_keep_precision_and_edited_dates_do_not_duplicate() {
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
        let result = validate_tailoring(&source, "Engineer", &draft.to_string(), 1).unwrap();
        assert_eq!(
            result.resume.sections[0].entries[0].date_range,
            "2021 – Present"
        );
        assert_eq!(result.resume.sections[0].entries[0].dates, Some(vec![]));
    }

    #[test]
    fn null_id_on_a_retained_refinement_recovers_reviewed_links_and_typed_dates() {
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
        draft["templateSections"][0]["entries"][0]["entryId"] = Value::Null;
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
            .remove("role");
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
        assert_eq!(plan["minItems"], 3);
        assert_eq!(plan["maxItems"], 3);
        assert_eq!(plan["items"]["maxLength"], 500);
        assert_eq!(schema["properties"]["schemaVersion"]["enum"], json!([4]));
    }
}
