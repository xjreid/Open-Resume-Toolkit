//! Dates are the one semantic exception to positional import slots. Preserve
//! month/year precision in the selector; ambiguous prose stays where it was.
use super::Entry;
use ort_domain::{CalendarDate, DateEnd, EntityId, ResumeDate};

pub(super) fn map_to_date_slot(entry: &mut Entry, path: &str) -> Result<Vec<ResumeDate>, String> {
    let mut dates = Vec::new();
    for slot in [
        &mut entry.title,
        &mut entry.details,
        &mut entry.role,
        &mut entry.date,
        &mut entry.location,
        &mut entry.extra,
    ] {
        extract_lines(slot, &mut dates, true);
    }
    entry.main_info.items.retain_mut(|item| {
        let extracted = extract_lines(item, &mut dates, false);
        !extracted || !item.trim().is_empty()
    });
    // Validate unrecognized date-like text rather than silently moving an
    // invalid date to Location or leaving free text in the date selector.
    if !entry.date.trim().is_empty() {
        if entry.date.chars().any(|c| c.is_ascii_digit())
            || matches!(
                entry.date.trim().to_lowercase().as_str(),
                "present" | "current" | "now"
            )
        {
            dates.push(std::mem::take(&mut entry.date));
        } else {
            // Displaced non-date text stays on the right in source order.
            entry.extra = [&entry.location, &entry.extra]
                .into_iter()
                .filter(|text| !text.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            entry.location = std::mem::take(&mut entry.date);
        }
    }
    let mut result: Option<ResumeDate> = None;
    for text in dates {
        let (start, end) = parse(&text).ok_or_else(|| format!(
            "{path}.rightTop must be a single date or range using month/year, year only, or Present as the end. Normalize source dates to that format without inventing missing months; keep non-date qualifiers in another detail slot."
        ))?;
        if let Some(date) = &result {
            if date.start != start || date.end != end {
                return Err(format!(
                    "{path} contains multiple distinct date details. The date selector accepts one date or range per entry; represent distinct dated items as separate entries without omitting content."
                ));
            }
        } else {
            result = Some(ResumeDate {
                id: EntityId::new(),
                order: 0,
                label: String::new(),
                start,
                end,
            });
        }
        // Qualifiers are not part of the selector's supported display format.
        // Retain them as ordinary detail text rather than silently dropping them.
        for qualifier in qualifiers(&text) {
            if !entry.extra.lines().any(|line| line == qualifier) {
                if !entry.extra.is_empty() {
                    entry.extra.push('\n');
                }
                entry.extra.push_str(qualifier);
            }
        }
    }
    entry.date.clear();
    Ok(result.into_iter().collect())
}

fn extract_lines(value: &mut String, dates: &mut Vec<String>, header: bool) -> bool {
    let mut extracted = false;
    let mut kept = Vec::new();
    for line in value.split('\n') {
        if is_date(line) {
            dates.push(line.trim().to_owned());
            extracted = true;
        } else if header
            && let Some((text, date)) = line.rsplit_once(['|', '\t', '·'])
            && is_date(date)
        {
            dates.push(date.trim().to_owned());
            kept.push(text.trim_end());
            extracted = true;
        } else {
            kept.push(line);
        }
    }
    if extracted {
        *value = kept.join("\n");
    }
    extracted
}

fn is_date(text: &str) -> bool {
    parse(text).is_some()
}

fn parse(text: &str) -> Option<(Option<CalendarDate>, Option<DateEnd>)> {
    let lower = text.trim().to_lowercase();
    let text = lower.trim_matches(['(', ')']);
    let text = ["graduation:", "graduated:", "issued:", "expires:"]
        .iter()
        .find_map(|label| text.strip_prefix(label))
        .unwrap_or(text)
        .trim();
    if matches!(text, "present" | "current" | "now") {
        return Some((None, Some(DateEnd::Present)));
    }
    if let Some(date) = calendar(text) {
        return Some((Some(date), None));
    }
    for separator in ["–", "—", "-", " to "] {
        for (offset, _) in text.match_indices(separator) {
            if let Some(start) = calendar(text[..offset].trim()) {
                let end = text[offset + separator.len()..].trim();
                let end = if matches!(end, "present" | "current" | "now") {
                    Some(DateEnd::Present)
                } else {
                    calendar(end).map(|value| DateEnd::Date { value })
                };
                if let Some(end) = end {
                    return Some((Some(start), Some(end)));
                }
            }
        }
    }
    None
}

fn calendar(text: &str) -> Option<CalendarDate> {
    let text = text
        .strip_prefix("expected ")
        .or_else(|| text.strip_prefix("since "))
        .unwrap_or(text)
        .trim();
    let date = |year, month| CalendarDate {
        year,
        month,
        expected: false,
    };
    if let Some(year) = year(text) {
        return Some(date(year, None));
    }
    let words: Vec<_> = text.split_whitespace().collect();
    if let [month, date_year] = words.as_slice() {
        let year = year(date_year)?;
        if ["spring", "summer", "fall", "autumn", "winter"].contains(month) {
            return Some(date(year, None));
        }
        return month_number(month).map(|month| date(year, Some(month)));
    }
    if let [first, second, date_year] = words.as_slice() {
        let year = year(date_year)?;
        let day = |value: &str| {
            value
                .trim_end_matches(',')
                .parse::<u8>()
                .ok()
                .filter(|day| (1..=31).contains(day))
        };
        if day(second).is_some() {
            return month_number(first).map(|month| date(year, Some(month)));
        }
        if day(first).is_some() {
            return month_number(second).map(|month| date(year, Some(month)));
        }
    }
    let parts: Vec<_> = text.split(['/', '-']).collect();
    match parts.as_slice() {
        [left, right] => {
            if let Some(year) = year(left) {
                return numeric_month(right).map(|month| date(year, Some(month)));
            }
            if let Some(year) = year(right) {
                return numeric_month(left).map(|month| date(year, Some(month)));
            }
        }
        [date_year, month, day] if day.parse::<u8>().is_ok_and(|day| (1..=31).contains(&day)) => {
            return Some(date(year(date_year)?, Some(numeric_month(month)?)));
        }
        _ => {}
    }
    None
}

fn year(text: &str) -> Option<u16> {
    (text.len() == 4 && text.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| {
            text.parse::<u16>()
                .ok()
                .filter(|year| (1900..=2199).contains(year))
        })
        .flatten()
}

fn numeric_month(text: &str) -> Option<u8> {
    (text.len() <= 2 && text.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| {
            text.parse::<u8>()
                .ok()
                .filter(|month| (1..=12).contains(month))
        })
        .flatten()
}

fn month_number(text: &str) -> Option<u8> {
    match text.trim_end_matches('.') {
        "jan" | "january" => Some(1),
        "feb" | "february" => Some(2),
        "mar" | "march" => Some(3),
        "apr" | "april" => Some(4),
        "may" => Some(5),
        "jun" | "june" => Some(6),
        "jul" | "july" => Some(7),
        "aug" | "august" => Some(8),
        "sep" | "sept" | "september" => Some(9),
        "oct" | "october" => Some(10),
        "nov" | "november" => Some(11),
        "dec" | "december" => Some(12),
        _ => None,
    }
}

fn qualifiers(text: &str) -> Vec<&'static str> {
    text.split(|c: char| !c.is_alphabetic())
        .filter_map(|word| match word.to_lowercase().as_str() {
            "expected" => Some("Expected"),
            "since" => Some("Since"),
            "spring" => Some("Spring"),
            "summer" => Some("Summer"),
            "fall" => Some("Fall"),
            "autumn" => Some("Autumn"),
            "winter" => Some("Winter"),
            "graduation" => Some("Graduation"),
            "graduated" => Some("Graduated"),
            "issued" => Some("Issued"),
            "expires" => Some("Expires"),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_date_details_without_treating_prose_or_version_numbers_as_dates() {
        for date in [
            "2024",
            "Expected May 2027",
            "Graduation: June 2025",
            "Fall 2020–Spring 2024",
            "05/2020 - 06/2022",
            "2020-05–Present",
            "Jan. 2022 to Current",
        ] {
            assert!(is_date(date), "{date}");
        }
        for text in [
            "SQL Server 2022",
            "Project 2024",
            "In 2024 I built a tool.",
            "2024 users",
            "13/2024",
            "https://example.com/2024",
            "[2024](https://example.com)",
        ] {
            assert!(!is_date(text), "{text}");
        }
    }
}
