//! Source-backed editing within the existing resume template.
//! Evidence references establish traceability, not proof of generated claims.
use super::list_selection::list_items;
use super::{AlertCandidate, MaterialError, RoleInfo, TailoredMaterial, validate_alerts};
use ort_domain::PARAGRAPH_FIELD_LABEL as PARAGRAPH;
use ort_domain::{
    Bullet, DocumentLimits, EntityId, NamedField, ResumeDocument, ResumeEntry, ResumeSection,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TemplateResponse {
    schema_version: u16,
    tailoring_plan: Vec<String>,
    role_priorities: Vec<String>,
    review_issues: Vec<String>,
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
    selected_lists: Vec<ListSelection>,
    main_info: MainInfo,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ListSelection {
    field_id: EntityId,
    item_indexes: Vec<usize>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MainInfo {
    format: BodyFormat,
    items: Vec<BodyItem>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BodyItem {
    text: String,
    source_refs: Vec<EntityId>,
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
#[must_use]
pub fn resume_context(document: &ResumeDocument) -> Value {
    json!({"contact":document.contact,"sections":document.sections.iter().map(|section| json!({
        "sectionId":section.id,"heading":section.heading,"entries":section.entries.iter().map(|entry| {
            let paragraphs: Vec<_> = entry.fields_for_role(ort_domain::FieldRole::Paragraph).filter(|f| !f.value.trim().is_empty()).map(|f| &f.value).collect();
            json!({"entryId":entry.id,"title":entry.heading,"role":entry.subheading,
                "details":entry.field_text(ort_domain::FieldRole::Details," | "),"date":date_text(entry),"location":entry.location,"extra":entry.field_text(ort_domain::FieldRole::Extra,"\n"),
                "mainInfo":if paragraphs.is_empty() { json!({"format":"bullets","items":entry.bullets.iter().map(|b| &b.text).collect::<Vec<_>>()}) } else {json!({"format":"paragraph","items":paragraphs})},
                "sourceBullets":entry.bullets.iter().map(|b|json!({"sourceId":b.id,"text":b.text})).collect::<Vec<_>>(),
                "sourceFields":entry.fields.iter().map(|f|json!({"fieldId":f.id,"label":f.label,"value":f.value,"isSkill":f.is_skill,"listKind":f.list_kind})).collect::<Vec<_>>(),
                "selectableLists":entry.fields.iter().filter(|f|f.selectable()).map(|f|json!({"fieldId":f.id,"items":list_items(f).items})).collect::<Vec<_>>(),
                "links":entry.links.iter().map(|l|json!({"label":l.label,"url":l.url})).collect::<Vec<_>>()})
        }).collect::<Vec<_>>()
    })).collect::<Vec<_>>()})
}
fn entries(document: &ResumeDocument) -> impl Iterator<Item = &ResumeEntry> {
    document.sections.iter().flat_map(|s| &s.entries)
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
            list_kind: None,
        });
    }
    Ok(())
}
fn select_fields(
    selected_lists: Vec<ListSelection>,
    original: &ResumeEntry,
    published: Option<&ResumeEntry>,
) -> Result<Vec<NamedField>, MaterialError> {
    let mut selected_fields = Vec::new();
    let mut selected = HashSet::new();
    for selection in selected_lists {
        if !selected.insert(selection.field_id) || selection.item_indexes.is_empty() {
            return Err(MaterialError::InvalidDetail("selectedLists contains a duplicate fieldId or an empty itemIndexes array. Omit the field to remove it.".into()));
        }
        // Selections must use published units; reviewed-only metadata stays protected.
        let f = published
            .and_then(|e| e.fields.iter().find(|f| f.id == selection.field_id))
            .ok_or_else(|| {
                MaterialError::InvalidDetail(format!(
                    "selectedLists.fieldId {} is not a published field in this entry.",
                    selection.field_id
                ))
            })?;
        if !f.selectable() {
            return Err(MaterialError::InvalidDetail(format!(
                "Field {} is protected; omit it from selectedLists.",
                f.id
            )));
        }
        let units = list_items(f);
        let mut indexes = HashSet::new();
        let mut values = Vec::new();
        for i in selection.item_indexes {
            if !indexes.insert(i) {
                return Err(MaterialError::InvalidDetail(format!(
                    "Field {} repeats item index {i}; each index may appear once.",
                    f.id
                )));
            }
            values.push(*units.items.get(i).ok_or_else(|| MaterialError::InvalidDetail(format!("Field {} has {} selectable items; index {i} is out of range. Use zero-based published indexes.", f.id, units.items.len())))?);
        }
        let mut retained = original
            .fields
            .iter()
            .find(|v| v.id == f.id)
            .unwrap_or(f)
            .clone();
        retained.value.clone_from(&f.value);
        // Selecting every item in its original order preserves even whitespace.
        if values != units.items {
            retained.value = values.join(units.separator);
        }
        selected_fields.push(retained);
    }
    let mut selections = selected_fields.into_iter();
    let mut fields = Vec::new();
    for f in original.fields.iter().filter(|f| f.label != PARAGRAPH) {
        if published.is_some_and(|e| {
            e.fields
                .iter()
                .any(|source| source.id == f.id && source.selectable())
        }) {
            if let Some(selected) = selections.next() {
                fields.push(selected);
            }
        } else {
            fields.push(f.clone());
        }
    }
    fields.extend(selections);
    for (i, f) in fields.iter_mut().enumerate() {
        f.order = order(i)?;
    }
    Ok(fields)
}

fn materialize_entry(
    draft: TemplateEntry,
    source: &ResumeDocument,
    section: &ResumeSection,
    index: usize,
    used: &mut HashSet<EntityId>,
) -> Result<ResumeEntry, MaterialError> {
    let original = section
        .entries
        .iter()
        .find(|e| e.id == draft.entry_id)
        .ok_or_else(|| {
            MaterialError::InvalidDetail(format!(
                "entryId {} does not belong to sectionId {}.",
                draft.entry_id, section.id
            ))
        })?;
    let mut anchors = HashSet::new();
    if draft.source_entry_ids.len() != 1
        || !used.insert(draft.entry_id)
        || draft
            .source_entry_ids
            .iter()
            .any(|id| !anchors.insert(*id) || !entries(source).any(|e| e.id == *id))
        || (entries(source).any(|e| e.id == draft.entry_id)
            && (anchors.len() != 1 || !anchors.contains(&draft.entry_id)))
    {
        return Err(MaterialError::InvalidDetail(format!(
            "entryId {} is repeated or has an invalid sourceEntryIds anchor. Use exactly one published anchor; a published entry must anchor itself.",
            draft.entry_id
        )));
    }
    // Published entries may cite only their own evidence. Current-only entries
    // retain explicit published anchors; no model-created identities are allowed.
    let evidence: HashSet<_> = entries(source)
        .filter(|e| anchors.contains(&e.id))
        .flat_map(|e| {
            e.bullets
                .iter()
                .map(|b| b.id)
                .chain(e.fields.iter().map(|f| f.id))
        })
        .collect();
    for (item_index, item) in draft.main_info.items.iter().enumerate() {
        let mut refs = HashSet::new();
        if item.text.trim().is_empty()
            || item.source_refs.is_empty()
            || item
                .source_refs
                .iter()
                .any(|id| !refs.insert(*id) || !evidence.contains(id))
        {
            return Err(MaterialError::EvidenceDetail(format!(
                "Entry {} mainInfo.items[{item_index}] has empty text or missing, duplicate, unknown, or cross-entry sourceRefs. Cite only published bullet/field IDs in this entry's anchor.",
                draft.entry_id
            )));
        }
    }
    let published = entries(source).find(|e| e.id == draft.entry_id);
    let mut fields = select_fields(draft.selected_lists, original, published)?;
    let mut bullets = Vec::new();
    match draft.main_info.format {
        BodyFormat::Paragraph => field(
            &mut fields,
            PARAGRAPH,
            &draft
                .main_info
                .items
                .iter()
                .map(|i| i.text.trim())
                .collect::<Vec<_>>()
                .join("\n\n"),
        )?,
        BodyFormat::Bullets => {
            for (i, item) in draft.main_info.items.into_iter().enumerate() {
                bullets.push(Bullet {
                    id: EntityId::new(),
                    order: order(i)?,
                    text: item.text.trim().into(),
                });
            }
        }
    }
    if original.heading.is_empty()
        && original.subheading.is_empty()
        && fields.is_empty()
        && bullets.is_empty()
    {
        return Err(MaterialError::Invalid);
    }
    Ok(ResumeEntry {
        order: order(index)?,
        fields,
        bullets,
        ..original.clone()
    })
}

pub(super) fn validate(
    source: &ResumeDocument,
    baseline: &ResumeDocument,
    job: &str,
    value: Value,
    published_revision: i64,
) -> Result<TailoredMaterial, MaterialError> {
    let proposed: TemplateResponse = serde_json::from_value(value).map_err(|error| {
        // Only known schema field names enter durable diagnostics; never echo
        // arbitrary model output from serde's error message.
        let message = error.to_string();
        let missing = message.strip_prefix("missing field `").and_then(|v| v.strip_suffix('`'));
        let known = ["schemaVersion", "tailoringPlan", "rolePriorities", "reviewIssues", "templateSections", "alerts", "sectionId", "entries", "entryId", "sourceEntryIds", "selectedLists", "mainInfo", "fieldId", "itemIndexes", "format", "items", "text", "sourceRefs"];
        MaterialError::InvalidDetail(if let Some(field) = missing.filter(|field| known.contains(field)) {
            format!("Missing required schema v6 field: {field}. Return the complete candidate, including empty arrays where appropriate.")
        } else {
            "Candidate JSON does not match schema v6. Check field types, UUID identities, format=bullets/paragraph, and remove unexpected properties.".into()
        })
    })?;
    if proposed.schema_version != 6 {
        return Err(MaterialError::Version);
    }
    let change_points = validate_strings("tailoringPlan", proposed.tailoring_plan, 1, 3)?;
    let role_priorities = validate_strings("rolePriorities", proposed.role_priorities, 1, 6)?;
    let review_issues = validate_strings("reviewIssues", proposed.review_issues, 0, 10)?;
    let limits = DocumentLimits::default();
    baseline
        .validate(limits)
        .map_err(|_| MaterialError::Invalid)?;
    if baseline.document_id != source.document_id
        || baseline.schema_version != source.schema_version
        || proposed.role_info.as_ref().is_some_and(|i| !i.valid())
        || proposed.template_sections.is_empty()
        || proposed.template_sections.len() > limits.sections
    {
        return Err(MaterialError::Invalid);
    }
    // Union permits restoration. Reviewed headers and retained entries win.
    let mut available = baseline.sections.clone();
    for s in &source.sections {
        if let Some(current) = available.iter_mut().find(|c| c.id == s.id) {
            for e in &s.entries {
                if !current.entries.iter().any(|c| c.id == e.id)
                    && !entries(baseline).any(|c| c.id == e.id)
                {
                    current.entries.push(e.clone());
                }
            }
        } else {
            available.push(s.clone());
        }
    }
    let mut resume = baseline.clone();
    resume.sections.clear();
    let mut used_sections = HashSet::new();
    let mut used_entries = HashSet::new();
    for (i, s) in proposed.template_sections.into_iter().enumerate() {
        if !used_sections.insert(s.section_id) || s.entries.is_empty() {
            return Err(MaterialError::InvalidDetail("templateSections repeats a sectionId or contains an empty entries array. Omit empty sections.".into()));
        }
        let original = available
            .iter()
            .find(|c| c.id == s.section_id)
            .ok_or_else(|| {
                MaterialError::InvalidDetail(format!(
                    "sectionId {} is not available in the published master or reviewed baseline.",
                    s.section_id
                ))
            })?;
        // Reject moving published identities between sections, including manual baseline moves.
        if s.entries.iter().any(|e| {
            source.sections.iter().any(|other| {
                other.id != s.section_id && other.entries.iter().any(|v| v.id == e.entry_id)
            })
        }) {
            return Err(MaterialError::InvalidDetail(format!(
                "Section {} contains an entry from another published section. Keep entries in their original sections.",
                s.section_id
            )));
        }
        let items = s
            .entries
            .into_iter()
            .enumerate()
            .map(|(j, e)| materialize_entry(e, source, original, j, &mut used_entries))
            .collect::<Result<Vec<_>, _>>()?;
        resume.sections.push(ResumeSection {
            id: s.section_id,
            order: order(i)?,
            heading: original.heading.clone(),
            entries: items,
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
        role_priorities,
        review_issues,
        alerts,
        alerts_truncated,
    })
}
fn validate_strings(
    name: &str,
    values: Vec<String>,
    min: usize,
    max: usize,
) -> Result<Vec<String>, MaterialError> {
    if !(min..=max).contains(&values.len()) {
        return Err(MaterialError::InvalidDetail(format!(
            "{name} must contain {min}–{max} unique, nonempty, single-line strings, each at most 500 characters."
        )));
    }
    let mut unique = HashSet::new();
    values
        .into_iter()
        .map(|p| {
            let p = p.trim();
            if p.is_empty()
                || p.chars().count() > 500
                || p.chars().any(char::is_control)
                || !unique.insert(
                    p.split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .to_lowercase(),
                )
            {
                Err(MaterialError::InvalidDetail(format!("{name} must contain {min}–{max} unique, nonempty, single-line strings, each at most 500 characters.")))
            } else {
                Ok(p.to_owned())
            }
        })
        .collect()
}
fn object(properties: &Value) -> Value {
    json!({"type":"object","additionalProperties":false,"required":properties.as_object().expect("properties").keys().collect::<Vec<_>>(),"properties":properties})
}
#[must_use]
pub fn resume_output_schema() -> Value {
    let text = json!({"type":"string"});
    let strings = |min, max| json!({"type":"array","minItems":min,"maxItems":max,"items":{"type":"string","minLength":1,"maxLength":500}});
    let item =
        object(&json!({"text":text,"sourceRefs":{"type":"array","minItems":1,"items":text}}));
    let list = object(
        &json!({"fieldId":text,"itemIndexes":{"type":"array","minItems":1,"items":{"type":"integer","minimum":0}}}),
    );
    let entry = object(
        &json!({"entryId":text,"sourceEntryIds":{"type":"array","minItems":1,"items":text},"selectedLists":{"type":"array","items":list},"mainInfo":object(&json!({"format":{"type":"string","enum":["bullets","paragraph"]},"items":{"type":"array","items":item}}))}),
    );
    let section =
        object(&json!({"sectionId":text,"entries":{"type":"array","minItems":1,"items":entry}}));
    let role = object(&json!({"company":text,"title":text,"location":text}));
    let evidence = object(&json!({"fieldId":text,"value":text}));
    let alert = object(
        &json!({"kind":{"type":"string","enum":["not_found","confirmed_mismatch"]},"category":{"type":"string","enum":["degree_level","field_of_study","graduation_date","certification_or_professional_license","named_skill_or_technology","language_proficiency","experience_duration","portfolio_or_work_sample"]},"requirement":text,"target":text,"jobExcerpt":text,"resumeEvidence":{"anyOf":[evidence,{"type":"null"}]}}),
    );
    object(
        &json!({"schemaVersion":{"type":"integer","enum":[6]},"tailoringPlan":strings(1,3),"rolePriorities":strings(1,6),"reviewIssues":strings(0,10),"roleInfo":{"anyOf":[role,{"type":"null"}]},"templateSections":{"type":"array","minItems":1,"items":section},"alerts":{"type":"array","maxItems":10,"items":alert}}),
    )
}
#[must_use]
pub fn gemini_resume_output_schema() -> Value {
    fn supported(v: &mut Value) {
        match v {
            Value::Object(o) => {
                o.remove("minLength");
                o.remove("maxLength");
                for c in o.values_mut() {
                    supported(c);
                }
            }
            Value::Array(a) => {
                for c in a {
                    supported(c);
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
    use crate::materials::{
        validate_refinement as validate_current_refinement,
        validate_tailoring as validate_current_tailoring,
    };
    // Adapt pre-v6 editorial fixtures to the new wire shape. New evidence and
    // list-permission tests below invoke the production validator directly.
    fn fixture_wire(source: &ResumeDocument, current: &ResumeDocument, raw: &str) -> String {
        let Ok(mut v) = serde_json::from_str::<Value>(raw) else {
            return raw.into();
        };
        if v["schemaVersion"] != 6 {
            return raw.into();
        }
        v.as_object_mut()
            .unwrap()
            .entry("rolePriorities")
            .or_insert(json!(["Source-backed tooling"]));
        v.as_object_mut()
            .unwrap()
            .entry("reviewIssues")
            .or_insert(json!([]));
        if let Some(sections) = v["templateSections"].as_array_mut() {
            for s in sections {
                if let Some(es) = s["entries"].as_array_mut() {
                    for e in es {
                        let id: Option<EntityId> =
                            serde_json::from_value(e["entryId"].clone()).ok();
                        let original = id.and_then(|id| {
                            entries(source)
                                .find(|e| e.id == id)
                                .or_else(|| entries(current).find(|e| e.id == id))
                        });
                        if let Some(original) = original {
                            e.as_object_mut().unwrap().entry("selectedLists").or_insert_with(||json!(original.fields.iter().filter(|f|f.selectable()).map(|f|json!({"fieldId":f.id,"itemIndexes":(0..list_items(f).items.len()).collect::<Vec<_>>()})).collect::<Vec<_>>()));
                            let refs: Vec<_> = original
                                .bullets
                                .iter()
                                .map(|b| b.id)
                                .chain(original.fields.iter().map(|f| f.id))
                                .collect();
                            if let Some(items) = e["mainInfo"]["items"].as_array_mut() {
                                for item in items {
                                    if let Some(text) = item.as_str() {
                                        *item = json!({"text":text,"sourceRefs":refs});
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        v.to_string()
    }
    fn validate_tailoring(
        source: &ResumeDocument,
        job: &str,
        raw: &str,
        revision: i64,
    ) -> Result<TailoredMaterial, MaterialError> {
        validate_current_tailoring(source, job, &fixture_wire(source, source, raw), revision)
    }
    fn validate_refinement(
        source: &ResumeDocument,
        current: &ResumeDocument,
        job: &str,
        raw: &str,
        revision: i64,
    ) -> Result<TailoredMaterial, MaterialError> {
        validate_current_refinement(
            source,
            current,
            job,
            &fixture_wire(source, current, raw),
            revision,
        )
    }
    use ort_domain::{CalendarDate, DateEnd, Link, ResumeDate};

    pub(super) fn source() -> ResumeDocument {
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
                    list_kind: None,
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

    pub(super) fn proposal(source: &ResumeDocument) -> Value {
        let context = resume_context(source);
        let v = json!({"schemaVersion":6,"tailoringPlan":[
            "Emphasize tooling needs by leading with the published Rust support-tool work.",
            "Make the SQL reporting experience easier to find in a specific body bullet.",
            "Keep the published experience context while condensing repeated body details."
        ],"roleInfo":null,"alerts":[],"templateSections":context["sections"].as_array().unwrap().iter().map(|section| {
            json!({"sectionId":section["sectionId"],
                "entries":section["entries"].as_array().unwrap().iter().map(|entry| {
                    json!({"entryId":entry["entryId"],"sourceEntryIds":[entry["entryId"]],
                        "mainInfo":{"format":"bullets","items":["Built Rust tools for the support team.","Maintained SQL reports to support the team's work."]}})
                }).collect::<Vec<_>>()})
        }).collect::<Vec<_>>()});
        serde_json::from_str(&fixture_wire(source, source, &v.to_string())).unwrap()
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
                list_kind: None,
            },
            NamedField {
                id: EntityId::new(),
                order: 2,
                label: " Extra ".into(),
                value: "Rust, SQL".into(),
                is_skill: true,
                list_kind: None,
            },
            NamedField {
                id: EntityId::new(),
                order: 3,
                label: "EXTRA".into(),
                value: "  Internal platform  ".into(),
                is_skill: false,
                list_kind: None,
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
        next["templateSections"][0]["entries"][0]["selectedLists"] = json!([]);
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
            &proposal(&source).to_string(),
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
        assert_eq!(schema["properties"]["schemaVersion"]["enum"], json!([6]));
    }
}

#[cfg(test)]
mod v6_tests {
    use super::tests::{proposal, source};
    use super::*;
    use crate::materials::validate_refinement;
    fn check(
        source: &ResumeDocument,
        current: &ResumeDocument,
        v: &Value,
    ) -> Result<TailoredMaterial, MaterialError> {
        validate_refinement(source, current, "Rust required", &v.to_string(), 1)
    }
    #[test]
    fn evidence_is_required_and_cannot_cross_entries() {
        let mut source = source();
        let mut other = source.sections[0].entries[0].clone();
        other.id = EntityId::new();
        other.order = 1;
        other.bullets[0].id = EntityId::new();
        other.fields[0].id = EntityId::new();
        source.sections[0].entries.push(other.clone());
        for refs in [
            json!([]),
            json!([EntityId::new()]),
            json!([other.bullets[0].id]),
            json!([
                source.sections[0].entries[0].bullets[0].id,
                source.sections[0].entries[0].bullets[0].id
            ]),
        ] {
            let mut v = proposal(&source);
            v["templateSections"][0]["entries"][0]["mainInfo"]["items"][0]["sourceRefs"] = refs;
            assert_eq!(
                check(&source, &source, &v).unwrap_err(),
                MaterialError::Evidence
            );
        }
        let mut v = proposal(&source);
        v["templateSections"][0]["entries"][0]["sourceEntryIds"] =
            json!([source.sections[0].entries[0].id, other.id]);
        assert!(check(&source, &source, &v).is_err());
    }
    #[test]
    fn exact_list_items_can_be_removed_reordered_and_restored() {
        let source = source();
        let mut v = proposal(&source);
        v["templateSections"][0]["entries"][0]["selectedLists"][0]["itemIndexes"] = json!([1, 0]);
        let first = check(&source, &source, &v).unwrap().resume;
        assert_eq!(first.sections[0].entries[0].fields[0].value, "SQL, Rust");
        v["templateSections"][0]["entries"][0]["selectedLists"][0]["itemIndexes"] = json!([0]);
        let reduced = check(&source, &first, &v).unwrap().resume;
        assert_eq!(reduced.sections[0].entries[0].fields[0].value, "Rust");
        v["templateSections"][0]["entries"][0]["selectedLists"][0]["itemIndexes"] = json!([1]);
        assert_eq!(
            check(&source, &reduced, &v).unwrap().resume.sections[0].entries[0].fields[0].value,
            "SQL"
        );
        for invalid in [json!([2]), json!([0, 0]), json!([])] {
            v["templateSections"][0]["entries"][0]["selectedLists"][0]["itemIndexes"] = invalid;
            assert!(check(&source, &source, &v).is_err());
        }
    }
    #[test]
    fn list_selection_cannot_change_text_or_protected_fields() {
        let mut source = source();
        let mut protected = source.sections[0].entries[0].fields[0].clone();
        protected.id = EntityId::new();
        protected.order = 1;
        protected.is_skill = false;
        source.sections[0].entries[0].fields.push(protected.clone());
        let mut v = proposal(&source);
        v["templateSections"][0]["entries"][0]["selectedLists"][0]["fieldId"] = json!(protected.id);
        assert!(check(&source, &source, &v).is_err());
        let mut v = proposal(&source);
        v["templateSections"][0]["entries"][0]["selectedLists"][0]["value"] = json!("Python");
        assert!(check(&source, &source, &v).is_err());
        let mut v = proposal(&source);
        v["templateSections"][0]["entries"][0]["selectedLists"] = json!([]);
        let result = check(&source, &source, &v).unwrap();
        assert_eq!(result.resume.sections[0].entries[0].fields.len(), 1);
        assert_eq!(
            result.resume.sections[0].entries[0].fields[0].value,
            protected.value
        );
    }
    #[test]
    fn reviewed_only_entries_cannot_combine_evidence_from_multiple_experiences() {
        let mut source = source();
        let mut other = tests::source().sections.remove(0).entries.remove(0);
        other.order = 1;
        source.sections[0].entries.push(other);
        let mut current = source.clone();
        current.sections[0].entries[0].id = EntityId::new();
        let mut v = proposal(&source);
        let entry = &mut v["templateSections"][0]["entries"][0];
        entry["entryId"] = json!(current.sections[0].entries[0].id);
        entry["selectedLists"] = json!([]);
        entry["sourceEntryIds"] = json!([
            source.sections[0].entries[0].id,
            source.sections[0].entries[1].id
        ]);
        assert!(check(&source, &current, &v).is_err());
        v["templateSections"][0]["entries"][0]["sourceEntryIds"] =
            json!([source.sections[0].entries[0].id]);
        assert!(check(&source, &current, &v).is_ok());
    }
    #[test]
    fn reviewed_only_lists_are_protected_and_cannot_supply_selection_items() {
        let source = source();
        let mut current = source.clone();
        let mut added = current.sections[0].entries[0].fields[0].clone();
        added.id = EntityId::new();
        added.order = 1;
        added.value = "Reviewed-only wording".into();
        current.sections[0].entries[0].fields.push(added.clone());
        let mut v = proposal(&source);
        let result = check(&source, &current, &v).unwrap();
        assert_eq!(result.resume.sections[0].entries[0].fields[1], added);
        v["templateSections"][0]["entries"][0]["selectedLists"]
            .as_array_mut()
            .unwrap()
            .push(json!({"fieldId": added.id, "itemIndexes": [0]}));
        assert!(check(&source, &current, &v).is_err());
    }
    #[test]
    fn coursework_classification_is_explicit_and_legacy_flags_work() {
        let mut source = source();
        let f = &mut source.sections[0].entries[0].fields[0];
        f.is_skill = false;
        f.list_kind = Some(ort_domain::ListKind::Coursework);
        f.value = "Algorithms; Systems (I, II)".into();
        let mut v = proposal(&source);
        v["templateSections"][0]["entries"][0]["selectedLists"][0]["itemIndexes"] = json!([1]);
        assert_eq!(
            check(&source, &source, &v).unwrap().resume.sections[0].entries[0].fields[0].value,
            "Systems (I, II)"
        );
        let mut old =
            serde_json::to_value(source.sections[0].entries[0].fields[0].clone()).unwrap();
        old.as_object_mut().unwrap().remove("listKind");
        old["isSkill"] = json!(true);
        assert!(
            serde_json::from_value::<NamedField>(old)
                .unwrap()
                .selectable()
        );
    }
    #[test]
    fn refinement_restores_omitted_sections_and_keeps_reviewed_headers() {
        let source = source();
        let mut current = source.clone();
        current.sections.clear();
        current.contact.phone = "Reviewed phone".into();
        let restored = check(&source, &current, &proposal(&source)).unwrap().resume;
        assert_eq!(restored.sections[0].id, source.sections[0].id);
        assert_eq!(restored.contact.phone, "Reviewed phone");
        let mut current = restored;
        current.sections[0].entries[0].heading = "Reviewed title".into();
        assert_eq!(
            check(&source, &current, &proposal(&source))
                .unwrap()
                .resume
                .sections[0]
                .entries[0]
                .heading,
            "Reviewed title"
        );
    }
    #[test]
    fn missing_review_contract_and_v5_are_rejected() {
        let source = source();
        for key in ["rolePriorities", "reviewIssues", "selectedLists"] {
            let mut v = proposal(&source);
            if key == "selectedLists" {
                v["templateSections"][0]["entries"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove(key);
            } else {
                v.as_object_mut().unwrap().remove(key);
            }
            assert!(check(&source, &source, &v).is_err());
        }
        let mut v = proposal(&source);
        v["schemaVersion"] = json!(5);
        assert!(check(&source, &source, &v).is_err());
    }
}
