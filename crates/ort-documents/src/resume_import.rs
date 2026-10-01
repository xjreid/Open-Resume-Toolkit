//! Deterministic, editable v2 draft from bounded extracted text and layout.
//! No persistence or parser authority. Ambiguous text stays visible in the body.
use crate::import::{
    BlockKind, ImportError, SectionKind, TextLayout, ValidatedExtraction, section_kind,
};
use ort_domain::{Bullet, EntityId, NamedField, ResumeDocument, ResumeEntry, ResumeSection};

const BODY: &str = "__ort_body_paragraph__";

#[derive(Clone, Copy)]
struct Line<'a> {
    text: &'a str,
    hint: BlockKind,
    layout: Option<TextLayout>,
    page: u16,
}
#[derive(Default)]
struct Placement {
    title: Option<TextLayout>,
    last: Option<TextLayout>,
    page: u16,
}

/// Maps suggestions into the same six header slots and body used by the canvas.
/// # Errors
/// Rejects candidates too large to display safely without dropping content.
pub fn map_resume(source: &ValidatedExtraction) -> Result<ResumeDocument, ImportError> {
    let mut document = ResumeDocument::empty("Imported Resume");
    document.schema_version = 2;
    let lines: Vec<_> = source
        .blocks()
        .iter()
        .flat_map(|block| {
            block.text.split('\n').map(move |text| Line {
                text: text.trim(),
                hint: block.kind,
                layout: block.layout,
                page: block.page,
            })
        })
        .collect();
    let mut section: Option<usize> = None;
    let mut section_layout = None;
    let mut in_header = true;
    let mut separated = false;
    let mut placement = Placement::default();
    for (index, &line) in lines.iter().enumerate() {
        let text = line.text;
        if text.is_empty() {
            separated = true;
            continue;
        }
        let next_has_date = next_header_date(&lines, index);
        let is_section = is_section_heading(
            line,
            section.map(|index| &document.sections[index]),
            section_layout,
            placement.last,
            separated,
            next_has_date,
        );
        if is_section
            && section.is_some_and(|index| {
                document.sections[index]
                    .heading
                    .eq_ignore_ascii_case(text.trim_end_matches(':'))
                    && placement.page != 0
                    && line.page != placement.page
            })
        {
            section_layout = line.layout;
            continue;
        }
        if is_section {
            in_header = false;
            section = Some(append_section(&mut document, text.trim_end_matches(':'))?);
            section_layout = line.layout;
            placement = Placement::default();
            separated = false;
            continue;
        }
        if in_header {
            if map_contact_header(&mut document, text) {
                continue;
            }
            if custom_heading(text, line.hint) && !document.contact.full_name.is_empty() {
                in_header = false;
                section = Some(append_section(&mut document, text.trim_end_matches(':'))?);
                section_layout = line.layout;
                continue;
            }
            if section.is_none() {
                section = Some(append_section(&mut document, "Imported information")?);
            }
        }
        // Only the adjacent header rows can establish a right column. Bound
        // lookahead independently of the extracted text size.
        let nearby: Vec<_> = lines[index..]
            .iter()
            .take(12)
            .take_while(|candidate| {
                candidate.text == text || section_kind(candidate.text).is_none()
            })
            .copied()
            .collect();
        let right_column = right_column(&nearby, placement.title.or(line.layout));
        let current = document
            .sections
            .get_mut(section.ok_or(ImportError::InvalidExtraction)?)
            .ok_or(ImportError::InvalidExtraction)?;
        map_entry(
            current,
            line,
            separated,
            next_has_date,
            right_column,
            &mut placement,
        )?;
        separated = false;
    }
    if serde_json::to_vec(&document)
        .map_err(|_| ImportError::InvalidExtraction)?
        .len()
        > ort_domain::DocumentLimits::default().serialized_bytes
    {
        return Err(ImportError::LimitExceeded);
    }
    Ok(document)
}

fn next_header_date(lines: &[Line<'_>], index: usize) -> bool {
    let origin = lines[index];
    lines
        .iter()
        .skip(index + 1)
        .take(6)
        .take_while(|line| !line.text.is_empty() && section_kind(line.text).is_none())
        .any(|line| {
            bullet_text(line.text, line.hint).is_none()
                && split_date(line.text).1.is_some()
                && match (line.layout, origin.layout) {
                    (Some(next), Some(current)) => {
                        line.page == origin.page
                            && i64::from(next.top) - i64::from(current.top)
                                <= i64::from(current.font_size) * 4
                    }
                    _ => true,
                }
        })
}
fn map_contact_header(document: &mut ResumeDocument, text: &str) -> bool {
    if ["resume", "curriculum vitae", "cv"].contains(&text.to_lowercase().as_str()) {
        text.clone_into(&mut document.title);
        true
    } else if contact(document, text) {
        true
    } else if document.contact.full_name.is_empty() && name_like(text) {
        text.clone_into(&mut document.contact.full_name);
        true
    } else {
        false
    }
}
fn is_section_heading(
    line: Line<'_>,
    current: Option<&ResumeSection>,
    section_layout: Option<TextLayout>,
    prior: Option<TextLayout>,
    separated: bool,
    next_has_date: bool,
) -> bool {
    let text = line.text;
    let current_kind = current.and_then(|section| section_kind(&section.heading));
    let known = section_kind(text);
    let styled = section_style(line, section_layout, prior);
    // Categories inside Skills stay in the body, including an unbulleted
    // "Languages" label. A distinct section uses section-level typography.
    let skill_category = current_kind == Some(SectionKind::Skills)
        && (known == Some(SectionKind::Languages)
            || (known.is_none()
                && (text.ends_with(':')
                    || [
                        "leadership",
                        "communication",
                        "tools",
                        "technologies",
                        "frameworks",
                        "databases",
                        "platforms",
                        "programming languages",
                        "training",
                    ]
                    .contains(&text.to_lowercase().as_str()))))
        && !styled;
    bullet_text(text, line.hint).is_none()
        && !skill_category
        && (known.is_some()
            || (named_custom_heading(text) && (line.layout.is_none() || styled))
            || (current.is_some()
                && !next_has_date
                && !body_like(text)
                && if line.layout.is_some() {
                    text.chars().count() <= 80 && styled
                } else {
                    custom_heading(text, line.hint)
                        && separated
                        && current_kind != Some(SectionKind::Skills)
                }))
}
fn same_row(left: TextLayout, right: TextLayout) -> bool {
    (i64::from(left.bottom) - i64::from(right.bottom)).abs()
        <= i64::from(left.font_size.max(right.font_size)) / 2
}
fn section_style(line: Line<'_>, heading: Option<TextLayout>, prior: Option<TextLayout>) -> bool {
    match (line.layout, heading) {
        (Some(layout), Some(heading)) => {
            layout.font_size >= heading.font_size * 95 / 100
                && (i64::from(layout.left) - i64::from(heading.left)).abs()
                    <= i64::from(heading.font_size)
                && prior.is_none_or(|prior| {
                    i64::from(layout.top) - i64::from(prior.bottom)
                        > i64::from(layout.font_size) / 2
                })
        }
        _ => false,
    }
}
fn right_column(lines: &[Line<'_>], title: Option<TextLayout>) -> Option<i32> {
    let title = title?;
    let date_left = lines
        .iter()
        .filter(|line| {
            line.layout.is_some()
                && bullet_text(line.text, line.hint).is_none()
                && split_date(line.text).1.is_some()
        })
        .filter_map(|line| line.layout)
        .filter(|layout| {
            i64::from(layout.left) - i64::from(title.left) > i64::from(title.font_size) * 10
        })
        .map(|layout| layout.left)
        .min();
    date_left
        .map(|left| left - i32::try_from(title.font_size * 2).unwrap_or(0))
        .or_else(|| {
            let right = lines
                .iter()
                .filter_map(|line| line.layout)
                .map(|layout| layout.right)
                .max()?;
            Some(title.left + (right - title.left) * 2 / 3)
        })
}
fn body_like(text: &str) -> bool {
    let words = text.split_whitespace().count();
    let first = text.split_whitespace().next().unwrap_or("").to_lowercase();
    text.chars().count() > 140
        || words > 18
        || text.contains([':', ';'])
        || text.ends_with(['!', '?'])
        || (text.ends_with('.')
            && words >= 3
            && !["inc.", "ltd.", "corp.", "co.", "llc.", "l.l.c."].contains(
                &text
                    .split_whitespace()
                    .last()
                    .unwrap_or("")
                    .to_lowercase()
                    .as_str(),
            ))
        || (words >= 4
            && [
                "built",
                "led",
                "managed",
                "developed",
                "created",
                "designed",
                "implemented",
                "delivered",
                "improved",
                "collaborated",
                "responsible",
                "supported",
                "worked",
                "increased",
                "reduced",
                "achieved",
                "provided",
            ]
            .contains(&first.as_str()))
}
fn map_entry(
    current: &mut ResumeSection,
    line: Line<'_>,
    separated: bool,
    next_has_date: bool,
    right_column: Option<i32>,
    placement: &mut Placement,
) -> Result<(), ImportError> {
    let text = line.text;
    let section_type = section_kind(&current.heading);
    let is_body_section = matches!(
        section_type,
        Some(SectionKind::Summary | SectionKind::Skills | SectionKind::Languages)
    ) || current.heading == "Imported information";
    let bullet = bullet_text(text, line.hint);
    if is_body_section {
        return map_body_section(current, line, placement);
    }
    let (title, date) = split_date(text);
    let positioned = line.layout.zip(placement.title);
    let row = positioned.is_some_and(|(layout, anchor)| same_row(layout, anchor));
    let on_right = line
        .layout
        .zip(right_column)
        .is_some_and(|(layout, column)| layout.left >= column);
    let aligned = positioned.is_some_and(|(layout, anchor)| {
        (i64::from(layout.left) - i64::from(anchor.left)).abs() <= i64::from(anchor.font_size)
    });
    let gap = line
        .layout
        .zip(placement.last)
        .map_or(0, |(layout, prior)| {
            i64::from(layout.top) - i64::from(prior.bottom)
        });
    let header_gap = line
        .layout
        .is_some_and(|layout| gap > i64::from(layout.font_size));
    let previous_body = current.entries.last().is_some_and(has_body);
    let raised = line
        .layout
        .zip(placement.last)
        .is_some_and(|(current, previous)| current.font_size > previous.font_size);
    let starts_entry = bullet.is_none()
        && !title.is_empty()
        && ((current.entries.is_empty() && !body_like(text))
            || (!body_like(text)
                && !on_right
                && !row
                && if line.layout.is_some() && placement.title.is_some() {
                    aligned
                        && ((previous_body && (next_has_date || (header_gap && raised)))
                            || (line.hint == BlockKind::Heading && header_gap))
                } else {
                    date.is_some()
                        || line.hint == BlockKind::Heading
                        || (previous_body && (separated || next_has_date))
                }));
    if current.entries.is_empty() || starts_entry {
        current.entries.push(empty_entry(current.entries.len())?);
        *placement = Placement {
            title: line.layout,
            last: None,
            page: line.page,
        };
    }
    let entry = current
        .entries
        .last_mut()
        .ok_or(ImportError::InvalidExtraction)?;
    if let Some(value) = bullet {
        if entry.fields.iter().any(|field| field.label == BODY) {
            append_body(entry, &format!("- {value}"));
        } else {
            entry.bullets.push(Bullet {
                id: EntityId::new(),
                order: checked_order(entry.bullets.len())?,
                text: value.into(),
            });
        }
    } else if starts_entry {
        set_header(entry, title, line.layout.is_some());
        if let Some(date) = date {
            date.clone_into(&mut entry.date_range);
        }
    } else if has_body(entry) || (body_like(text) && !row) {
        append_body(entry, text);
    } else if let Some(date) = date.filter(|_| entry.date_range.is_empty()) {
        date.clone_into(&mut entry.date_range);
        if !title.is_empty() {
            if on_right {
                set_slot(entry, "Extra", title);
            } else if entry.subheading.is_empty() {
                title.clone_into(&mut entry.subheading);
            } else {
                append_body(entry, title);
            }
        }
    } else {
        map_header_continuation(entry, line, placement, right_column);
    }
    placement.last = line.layout;
    placement.page = line.page;
    Ok(())
}
fn map_body_section(
    current: &mut ResumeSection,
    line: Line<'_>,
    placement: &mut Placement,
) -> Result<(), ImportError> {
    if current.entries.is_empty() {
        current.entries.push(empty_entry(0)?);
    }
    let entry = current
        .entries
        .last_mut()
        .ok_or(ImportError::InvalidExtraction)?;
    if let Some(value) = bullet_text(line.text, line.hint) {
        append_body(entry, &format!("- {value}"));
    } else {
        append_body(entry, line.text);
    }
    placement.last = line.layout;
    placement.page = line.page;
    Ok(())
}
fn map_header_continuation(
    entry: &mut ResumeEntry,
    line: Line<'_>,
    placement: &Placement,
    right_column: Option<i32>,
) {
    let text = line.text;
    if let Some((layout, anchor)) = line.layout.zip(placement.title) {
        let row = same_row(layout, anchor);
        let on_right = right_column.is_some_and(|column| layout.left >= column);
        let aligned =
            (i64::from(layout.left) - i64::from(anchor.left)).abs() <= i64::from(anchor.font_size);
        let gap = placement
            .last
            .map_or(0, |prior| i64::from(layout.top) - i64::from(prior.bottom));
        let close = line.page == placement.page
            && i64::from(layout.top) - i64::from(anchor.bottom) <= i64::from(anchor.font_size) * 4
            && gap <= i64::from(anchor.font_size) * 3 / 2;
        if row && !on_right && layout.left >= anchor.right && !slot_exists(entry, "Details") {
            set_slot(entry, "Details", text);
        } else if on_right && close && entry.location.is_empty() {
            text.clone_into(&mut entry.location);
        } else if on_right && close && !slot_exists(entry, "Extra") {
            set_slot(entry, "Extra", text);
        } else if aligned && close && !row && entry.subheading.is_empty() {
            text.clone_into(&mut entry.subheading);
        } else {
            append_body(entry, text);
        }
    } else if entry.location.is_empty() && location_like(text) {
        text.clone_into(&mut entry.location);
    } else if entry.subheading.is_empty() && text.chars().count() < 160 {
        text.clone_into(&mut entry.subheading);
    } else {
        append_body(entry, text);
    }
}
fn slot_exists(entry: &ResumeEntry, label: &str) -> bool {
    entry.fields.iter().any(|field| field.label == label)
}
fn set_slot(entry: &mut ResumeEntry, label: &str, value: &str) {
    if slot_exists(entry, label) {
        append_body(entry, value);
    } else {
        entry.fields.push(NamedField {
            id: EntityId::new(),
            order: u16::try_from(entry.fields.len()).unwrap_or(u16::MAX),
            label: label.into(),
            value: value.into(),
            is_skill: false,
        });
    }
}
fn append_section(document: &mut ResumeDocument, heading: &str) -> Result<usize, ImportError> {
    let index = document.sections.len();
    document.sections.push(ResumeSection {
        id: EntityId::new(),
        order: checked_order(index)?,
        heading: heading.into(),
        entries: vec![],
    });
    Ok(index)
}
fn checked_order(index: usize) -> Result<u16, ImportError> {
    u16::try_from(index).map_err(|_| ImportError::LimitExceeded)
}

fn empty_entry(order: usize) -> Result<ResumeEntry, ImportError> {
    Ok(ResumeEntry {
        id: EntityId::new(),
        order: checked_order(order)?,
        heading: String::new(),
        subheading: String::new(),
        date_range: String::new(),
        dates: Some(vec![]),
        location: String::new(),
        fields: vec![],
        bullets: vec![],
        links: vec![],
    })
}
fn has_body(entry: &ResumeEntry) -> bool {
    !entry.bullets.is_empty() || entry.fields.iter().any(|field| field.label == BODY)
}
fn set_header(entry: &mut ResumeEntry, text: &str, positioned: bool) {
    let mut parts = text
        .split(['|', '\t'])
        .map(str::trim)
        .filter(|part| !part.is_empty());
    parts.next().unwrap_or(text).clone_into(&mut entry.heading);
    for part in parts {
        if entry.location.is_empty() && location_like(part) {
            part.clone_into(&mut entry.location);
        } else if positioned && !slot_exists(entry, "Details") {
            set_slot(entry, "Details", part);
        } else if !positioned && entry.subheading.is_empty() {
            part.clone_into(&mut entry.subheading);
        } else {
            append_body(entry, part);
        }
    }
}
fn append_body(entry: &mut ResumeEntry, text: &str) {
    // The canvas presents one body area. Merge mixed paragraph/list content so
    // no extracted content is hidden behind a second body representation.
    if !entry.bullets.is_empty() {
        let prior = entry
            .bullets
            .iter()
            .map(|bullet| format!("- {}", bullet.text))
            .collect::<Vec<_>>()
            .join("\n");
        entry.bullets.clear();
        append_body(entry, &prior);
    }
    if let Some(field) = entry.fields.iter_mut().find(|field| field.label == BODY) {
        field.value.push('\n');
        field.value.push_str(text);
    } else {
        entry.fields.push(NamedField {
            id: EntityId::new(),
            order: u16::try_from(entry.fields.len()).unwrap_or(u16::MAX),
            label: BODY.into(),
            value: text.to_owned(),
            is_skill: false,
        });
    }
}
fn named_custom_heading(text: &str) -> bool {
    [
        "awards",
        "honors",
        "publications",
        "volunteer experience",
        "volunteering",
        "community service",
        "interests",
        "professional development",
        "training",
        "references",
        "leadership",
    ]
    .contains(&text.trim_end_matches(':').to_lowercase().as_str())
}
fn custom_heading(text: &str, hint: BlockKind) -> bool {
    bullet_text(text, hint).is_none()
        && text.chars().count() <= 80
        && (hint == BlockKind::Heading
            || named_custom_heading(text)
            || (text.chars().any(char::is_alphabetic)
                && !text.contains(['@', '.', '|'])
                && text
                    .chars()
                    .filter(|c| c.is_alphabetic())
                    .all(char::is_uppercase)))
}
fn name_like(text: &str) -> bool {
    let words = text.split_whitespace().count();
    (2..=6).contains(&words)
        && text.chars().count() <= 80
        && !text.to_lowercase().contains("resume")
        && !text.to_lowercase().contains("curriculum")
        && text
            .chars()
            .all(|c| c.is_alphabetic() || c.is_whitespace() || "-'’.".contains(c))
}
fn location_like(text: &str) -> bool {
    text.chars().count() < 100
        && (text.contains(',')
            || matches!(
                text.to_lowercase().as_str(),
                "remote" | "hybrid" | "on-site"
            ))
}
fn bullet_text(text: &str, hint: BlockKind) -> Option<&str> {
    for prefix in ["•", "●", "▪", "◦", "- ", "* ", "– "] {
        if let Some(rest) = text.strip_prefix(prefix) {
            return Some(rest.trim());
        }
    }
    (hint == BlockKind::ListItem).then_some(text)
}

// Preserve date wording and precision as legacy free text. No invented months.
fn split_date(text: &str) -> (&str, Option<&str>) {
    let words: Vec<_> = text.split_whitespace().collect();
    let year = words.iter().position(|word| {
        word.as_bytes().windows(4).any(|part| {
            part.iter().all(u8::is_ascii_digit)
                && std::str::from_utf8(part)
                    .ok()
                    .and_then(|value| value.parse::<u16>().ok())
                    .is_some_and(|year| (1900..=2199).contains(&year))
        })
    });
    let Some(year) = year else {
        return (text, None);
    };
    let mut start = if year > 0 && month(words[year - 1]) {
        year - 1
    } else {
        year
    };
    if start > 0 && words[start - 1].eq_ignore_ascii_case("expected") {
        start -= 1;
    }
    let date_words = &words[start..];
    // Reject prose such as "In 2022 I built ..." and date-like skill names.
    // Every suffix token must be date vocabulary, punctuation or digits.
    if date_words.len() > 12 || !date_words.iter().all(|word| date_word(word)) {
        return (text, None);
    }
    let offset = text
        .match_indices(words[start])
        .next()
        .map_or(0, |(offset, _)| offset);
    let title =
        text[..offset].trim_end_matches(|c: char| c.is_whitespace() || "|·,–—-".contains(c));
    (title, Some(text[offset..].trim()))
}
fn date_word(word: &str) -> bool {
    word.split(['-', '–', '—', '/']).all(|part| {
        let part = part.trim_matches(['.', ',', '(', ')', '|']);
        part.is_empty()
            || part.chars().all(|c| c.is_ascii_digit())
            || month(part)
            || [
                "to", "present", "current", "expected", "since", "spring", "summer", "fall",
                "autumn", "winter",
            ]
            .contains(&part.to_lowercase().as_str())
    })
}
fn month(word: &str) -> bool {
    [
        "jan",
        "january",
        "feb",
        "february",
        "mar",
        "march",
        "apr",
        "april",
        "may",
        "jun",
        "june",
        "jul",
        "july",
        "aug",
        "august",
        "sep",
        "sept",
        "september",
        "oct",
        "october",
        "nov",
        "november",
        "dec",
        "december",
    ]
    .contains(&word.trim_end_matches('.').to_lowercase().as_str())
}
fn contact(document: &mut ResumeDocument, text: &str) -> bool {
    let lower = text.to_lowercase();
    let labeled = [
        "name:",
        "full name:",
        "email:",
        "e-mail:",
        "phone:",
        "telephone:",
        "mobile:",
        "location:",
        "address:",
    ]
    .iter()
    .any(|label| lower.starts_with(label));
    let has_contact = labeled
        || text.contains('@')
        || text.contains("https://")
        || text.contains("http://")
        || text.contains("linkedin.com/")
        || text.contains("github.com/")
        || phone_like(text);
    if !has_contact {
        return false;
    }
    for part in text
        .split(['|', '•', '·', '\t'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        contact_part(document, part);
    }
    true
}
fn contact_part(document: &mut ResumeDocument, part: &str) {
    let lower = part.to_lowercase();
    for (labels, field) in [
        (
            &["name:", "full name:"][..],
            &mut document.contact.full_name,
        ),
        (&["email:", "e-mail:"][..], &mut document.contact.email),
        (
            &["phone:", "telephone:", "mobile:"][..],
            &mut document.contact.phone,
        ),
        (
            &["location:", "address:"][..],
            &mut document.contact.location,
        ),
    ] {
        if let Some(label) = labels.iter().find(|label| lower.starts_with(**label))
            && field.is_empty()
        {
            part[label.len()..].trim().clone_into(field);
            return;
        }
    }
    if let Some(address) = part.split_whitespace().find(|word| word.contains('@'))
        && document.contact.email.is_empty()
    {
        address
            .trim_matches([',', ';'])
            .clone_into(&mut document.contact.email);
        let rest = part.replace(address, "");
        if !rest.trim().is_empty() {
            contact_remainder(document, rest.trim());
        }
        return;
    }
    contact_remainder(document, part);
}
fn phone_like(value: &str) -> bool {
    let digits = value.chars().filter(char::is_ascii_digit).count();
    (7..=15).contains(&digits)
        && value
            .chars()
            .all(|c| c.is_ascii_digit() || c.is_whitespace() || "+()-./".contains(c))
}
fn contact_remainder(document: &mut ResumeDocument, value: &str) {
    if phone_like(value) && document.contact.phone.is_empty() {
        value.clone_into(&mut document.contact.phone);
    } else if document.contact.full_name.is_empty() && name_like(value) {
        value.clone_into(&mut document.contact.full_name);
    } else if let Some((start, end)) =
        phone_span(value).filter(|_| document.contact.phone.is_empty())
    {
        value[start..end]
            .trim()
            .clone_into(&mut document.contact.phone);
        for part in [&value[..start], &value[end..]]
            .map(str::trim)
            .into_iter()
            .filter(|part| !part.is_empty())
        {
            contact_remainder(document, part);
        }
    } else {
        if !document.contact.location.is_empty() {
            document.contact.location.push('\n');
        }
        // URLs stay visible and editable in the canvas's free-form contact area.
        document.contact.location.push_str(value);
    }
}
fn phone_span(value: &str) -> Option<(usize, usize)> {
    // URL digits and email addresses are not phone numbers.
    if value.contains("http") || value.contains(".com/") || value.contains('@') {
        return None;
    }
    let mut start = None;
    for (offset, c) in value
        .char_indices()
        .chain(std::iter::once((value.len(), '\0')))
    {
        if start.is_none() && (c.is_ascii_digit() || c == '+') {
            start = Some(offset);
        }
        if !(c.is_ascii_digit() || c.is_whitespace() || "+()-./".contains(c))
            && let Some(from) = start.take()
            && phone_like(&value[from..offset])
        {
            return Some((from, offset));
        }
    }
    None
}
