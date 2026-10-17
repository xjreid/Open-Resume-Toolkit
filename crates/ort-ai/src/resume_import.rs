//! PDF import only: provider attachments, a strict seven-slot proposal, and
//! native-owned resume identities. No text extraction or resume rewriting.
use crate::{
    AiError, ApiKey, HttpRequest, NormalizedRequest, OperationType, Provider, ProviderAdapter,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use ort_domain::{
    Bullet, ContactDetails, DocumentLimits, EntityId, Link, NamedField, ResumeDocument,
    ResumeEntry, ResumeSection,
};
use serde::Deserialize;
use serde_json::{Value, json};
use url::Url;

mod schema;
pub use schema::{SYSTEM, input, output_schema, provider_output_schema};
mod pdf_links;
pub use pdf_links::{PdfHyperlink, input_with_pdf_hyperlinks, valid_pdf_hyperlinks};
mod contacts;

pub const MAX_PDF_BYTES: usize = 10 * 1024 * 1024;

fn empty_if_null<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Proposal {
    contact: Contact,
    sections: Vec<Section>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Contact {
    #[serde(default, deserialize_with = "empty_if_null")]
    full_name: String,
    #[serde(default, deserialize_with = "empty_if_null")]
    email: String,
    #[serde(default, deserialize_with = "empty_if_null")]
    phone: String,
    #[serde(default, deserialize_with = "empty_if_null")]
    location: String,
    #[serde(default, deserialize_with = "empty_if_null")]
    links: Vec<ContactLink>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContactLink {
    #[serde(default, deserialize_with = "empty_if_null")]
    label: String,
    url: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Section {
    heading: String,
    entries: Vec<Entry>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Entry {
    #[serde(
        rename = "leftHeading",
        alias = "title",
        default,
        deserialize_with = "empty_if_null"
    )]
    title: String,
    #[serde(
        rename = "leftSubheading",
        alias = "role",
        default,
        deserialize_with = "empty_if_null"
    )]
    role: String,
    #[serde(
        rename = "headingSuffix",
        alias = "details",
        default,
        deserialize_with = "empty_if_null"
    )]
    details: String,
    #[serde(
        rename = "rightTop",
        alias = "date",
        default,
        deserialize_with = "empty_if_null"
    )]
    date: String,
    #[serde(
        rename = "rightMiddle",
        alias = "location",
        default,
        deserialize_with = "empty_if_null"
    )]
    location: String,
    #[serde(
        rename = "rightBottom",
        alias = "extra",
        default,
        deserialize_with = "empty_if_null"
    )]
    extra: String,
    main_info: MainInfo,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
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

/// Builds an inline PDF request without remote file storage or exposing the
/// selected filename. Uses the existing adapters' authentication/endpoints.
/// # Errors
/// Rejects non-import requests, invalid PDFs, and adapter/serialization errors.
pub fn build_request(
    adapter: &dyn ProviderAdapter,
    request: &NormalizedRequest,
    key: &ApiKey,
    pdf: &[u8],
) -> Result<HttpRequest, AiError> {
    if request.operation != OperationType::ImportMapping
        || pdf.len() > MAX_PDF_BYTES
        || !pdf.starts_with(b"%PDF-")
    {
        return Err(AiError::InvalidResponse);
    }
    let mut http = adapter.build_request(request, key)?;
    let mut body: Value =
        serde_json::from_slice(&http.body).map_err(|_| AiError::InvalidResponse)?;
    let data = STANDARD.encode(pdf);
    let mut prompt_input = request.input.clone();
    if adapter.provider() == Provider::Gemini
        && let Some(input) = prompt_input.as_object_mut()
    {
        // Gemini already receives its compatible schema via responseFormat.
        // Do not repeat the richer native schema in its user prompt.
        input.remove("schema");
    }
    let prompt = serde_json::to_string(&prompt_input).map_err(|_| AiError::InvalidResponse)?;
    let schema = provider_output_schema(adapter.provider());
    match adapter.provider() {
        Provider::OpenAi => {
            body["input"] = json!([{"role":"user","content":[{"type":"input_file","filename":"resume.pdf","file_data":format!("data:application/pdf;base64,{data}")},{"type":"input_text","text":prompt}]}]);
            body["text"]["format"] =
                json!({"type":"json_schema","name":"resume_import","strict":true,"schema":schema});
        }
        Provider::Anthropic => {
            body["messages"] = json!([{"role":"user","content":[{"type":"document","source":{"type":"base64","media_type":"application/pdf","data":data}},{"type":"text","text":prompt}]}]);
            body["output_config"]["format"] = json!({"type":"json_schema","schema":schema});
        }
        Provider::Gemini => {
            body["contents"] = json!([{"role":"user","parts":[{"inlineData":{"mimeType":"application/pdf","data":data}},{"text":prompt}]}]);
            body["generationConfig"]
                .as_object_mut()
                .ok_or(AiError::InvalidResponse)?
                .remove("responseMimeType");
            body["generationConfig"]["responseFormat"] =
                json!({"text":{"mimeType":"APPLICATION_JSON","schema":schema}});
            if request.model.starts_with("gemini-3.") {
                body["generationConfig"]["thinkingConfig"] = json!({"thinkingLevel":"LOW"});
            }
        }
    }
    http.body = serde_json::to_vec(&body).map_err(|_| AiError::InvalidResponse)?;
    Ok(http)
}

fn order(index: usize) -> Result<u16, String> {
    u16::try_from(index).map_err(|_| "The response contains too many items.".into())
}
fn field(fields: &mut Vec<NamedField>, label: &str, value: String) -> Result<(), String> {
    if !value.trim().is_empty() {
        fields.push(NamedField {
            id: EntityId::new(),
            order: order(fields.len())?,
            label: label.into(),
            value,
            is_skill: false,
            list_kind: None,
        });
    }
    Ok(())
}

// Only unwrap a single, complete JSON code block. Do not fish an object out of
// prose, concatenate multiple answers, or accept truncated JSON.
fn json_text(text: &str) -> &str {
    let text = text.trim().trim_start_matches('\u{feff}').trim();
    if let Some((opening, body)) = text.split_once('\n')
        && matches!(opening.trim_end(), "```json" | "```")
        && let Some(body) = body.trim_end().strip_suffix("```")
    {
        return body.trim();
    }
    text
}

fn contact_url(source: &str, index: usize) -> Result<String, String> {
    let invalid = || {
        format!(
            "contact.links[{index}].url must be a complete http://, https:// or mailto: URL, or a bare web address with a valid host. Unsafe schemes, Markdown, and unknown destinations are not accepted."
        )
    };
    let source = source.trim();
    if source.is_empty() || source.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(invalid());
    }
    let (candidate, added_scheme) = if let Some(address) = source.strip_prefix("//") {
        (format!("https://{address}"), true)
    } else if let Ok(parsed) = Url::parse(source) {
        if !matches!(parsed.scheme(), "https" | "http" | "mailto") {
            return Err(invalid());
        }
        (source.to_owned(), false)
    } else {
        (format!("https://{source}"), true)
    };
    let parsed = Url::parse(&candidate).map_err(|_| invalid())?;
    if parsed.scheme() == "mailto" {
        if parsed.path().is_empty() {
            return Err(invalid());
        }
    } else {
        let host = parsed.host_str().ok_or_else(invalid)?;
        if !parsed.username().is_empty()
            || parsed.password().is_some()
            || (added_scheme
                && (!host.contains('.')
                    || host.starts_with('.')
                    || host.ends_with('.')
                    || host.contains("..")))
        {
            return Err(invalid());
        }
    }
    Ok(candidate)
}

fn check_text(value: &str, maximum: usize, path: &str) -> Result<(), String> {
    if value.chars().count() > maximum {
        return Err(format!(
            "{path} exceeds the {maximum}-character limit. Split long body text into consecutive items without omitting words."
        ));
    }
    Ok(())
}

fn check_entry(entry: &Entry, path: &str) -> Result<(), String> {
    let limits = DocumentLimits::default();
    for (name, value) in [
        ("leftHeading", &entry.title),
        ("leftSubheading", &entry.role),
        ("headingSuffix", &entry.details),
        ("rightTop", &entry.date),
        ("rightMiddle", &entry.location),
        ("rightBottom", &entry.extra),
    ] {
        check_text(value, limits.field_characters, &format!("{path}.{name}"))?;
    }
    let maximum = match entry.main_info.format {
        BodyFormat::Bullets => limits.bullet_characters,
        BodyFormat::Paragraph => limits.field_characters,
    };
    for (index, item) in entry.main_info.items.iter().enumerate() {
        check_text(item, maximum, &format!("{path}.mainInfo.items[{index}]"))?;
    }
    Ok(())
}

fn paragraphs(fields: &mut Vec<NamedField>, items: Vec<String>) -> Result<(), String> {
    // Several valid paragraph items need not fit in one native field. Pack
    // consecutive items without exceeding its bound or discarding source text.
    let maximum = DocumentLimits::default().field_characters;
    let mut body = String::new();
    let mut characters = 0;
    for item in items {
        let count = item.chars().count();
        if !body.is_empty() && characters + 2 + count > maximum {
            field(
                fields,
                ort_domain::PARAGRAPH_FIELD_LABEL,
                std::mem::take(&mut body),
            )?;
            characters = 0;
        }
        if !body.is_empty() {
            body.push_str("\n\n");
            characters += 2;
        }
        body.push_str(&item);
        characters += count;
    }
    field(fields, ort_domain::PARAGRAPH_FIELD_LABEL, body)
}
/// Validates the entire response before it may enter a review session. Errors
/// contain structural diagnostics, never provider-returned resume text.
/// # Errors
/// Rejects malformed JSON, unknown fields, missing structure, empty output,
/// unsafe links and document limits. Absent/null display text becomes empty;
/// visible bare web addresses gain HTTPS. No partial document is returned.
pub fn decode(text: &str) -> Result<ResumeDocument, String> {
    if text.len() > DocumentLimits::default().serialized_bytes {
        return Err("The JSON response exceeds the resume size limit.".into());
    }
    let mut proposal: Proposal = serde_json::from_str(json_text(text)).map_err(|error| format!("The response does not match the resume JSON schema at line {}, column {} ({:?}). Expected contact and sections, section entries, and mainInfo objects with format and string items. Extra fields and incorrectly typed values are not accepted.", error.line(), error.column(), error.classify()))?;
    contacts::consolidate(&mut proposal);
    let mut document = ResumeDocument::empty("Imported Resume");
    document.schema_version = 2;
    document.contact = ContactDetails {
        full_name: proposal.contact.full_name,
        email: proposal.contact.email,
        phone: proposal.contact.phone,
        location: proposal.contact.location,
        links: proposal
            .contact
            .links
            .into_iter()
            .enumerate()
            .map(|(index, link)| {
                Ok(Link {
                    id: Some(EntityId::new()),
                    order: Some(order(index)?),
                    label: link.label,
                    url: contact_url(&link.url, index)?,
                })
            })
            .collect::<Result<_, String>>()?,
    };
    for (section_index, section) in proposal.sections.into_iter().enumerate() {
        if section.heading.trim().is_empty() {
            return Err(format!(
                "sections[{section_index}].heading must not be empty."
            ));
        }
        let mut entries = Vec::new();
        for (index, entry) in section.entries.into_iter().enumerate() {
            check_entry(
                &entry,
                &format!("sections[{section_index}].entries[{index}]"),
            )?;
            let mut fields = Vec::new();
            field(&mut fields, "Details", entry.details)?;
            field(&mut fields, "Extra", entry.extra)?;
            let mut bullets = Vec::new();
            match entry.main_info.format {
                BodyFormat::Bullets => {
                    for (index, text) in entry.main_info.items.into_iter().enumerate() {
                        bullets.push(Bullet {
                            id: EntityId::new(),
                            order: order(index)?,
                            text,
                        });
                    }
                }
                BodyFormat::Paragraph => paragraphs(&mut fields, entry.main_info.items)?,
            }
            entries.push(ResumeEntry {
                id: EntityId::new(),
                order: order(index)?,
                heading: entry.title,
                subheading: entry.role,
                date_range: entry.date,
                dates: Some(vec![]),
                location: entry.location,
                fields,
                bullets,
                links: vec![],
            });
        }
        document.sections.push(ResumeSection {
            id: EntityId::new(),
            order: order(section_index)?,
            heading: section.heading,
            entries,
        });
    }
    if document
        .sections
        .iter()
        .flat_map(|section| &section.entries)
        .all(|entry| {
            entry.heading.trim().is_empty()
                && entry.subheading.trim().is_empty()
                && entry.date_range.trim().is_empty()
                && entry.location.trim().is_empty()
                && entry.fields.is_empty()
                && entry
                    .bullets
                    .iter()
                    .all(|bullet| bullet.text.trim().is_empty())
        })
    {
        return Err(
            "The provider returned no readable resume sections. Try another PDF or model.".into(),
        );
    }
    document.validate(DocumentLimits::default()).map_err(|error| format!("The mapped resume is invalid: {error}. Check field lengths, section counts, and contact link URLs."))?;
    Ok(document)
}

#[cfg(test)]
#[path = "resume_import_tests.rs"]
mod tests;
