use ort_domain::DocumentLimits;

/// Join consecutive paragraphs only while they fit in one native field.
/// Keep individual paragraphs intact: an oversized item still fails document
/// validation instead of being silently split to evade its per-item limit.
pub(crate) fn pack_paragraphs<'a>(items: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    pack(items, "\n\n")
}

/// Import items are consecutive source lines. Paragraph breaks, when present
/// in the source, are explicit inside an item rather than inferred per item.
pub(crate) fn pack_import_lines<'a>(items: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    pack(items, "\n")
}

fn pack<'a>(items: impl IntoIterator<Item = &'a str>, separator: &str) -> Vec<String> {
    let maximum = DocumentLimits::default().field_characters;
    let separator_characters = separator.chars().count();
    let mut packed = Vec::new();
    let mut body = String::new();
    let mut characters = 0;
    for item in items {
        let count = item.chars().count();
        if !body.is_empty() && characters + separator_characters + count > maximum {
            packed.push(std::mem::take(&mut body));
            characters = 0;
        }
        if !body.is_empty() {
            body.push_str(separator);
            characters += separator_characters;
        }
        body.push_str(item);
        characters += count;
    }
    if !body.is_empty() {
        packed.push(body);
    }
    packed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_to_character_limit_including_separators() {
        let first = "é".repeat(1_000);
        let second = "界".repeat(998);
        let last = "Last paragraph";
        assert_eq!(
            pack_paragraphs([first.as_str(), second.as_str(), last]),
            vec![format!("{first}\n\n{second}"), last.into()]
        );
    }

    #[test]
    fn preserves_oversized_items_for_validation_and_handles_empty_body() {
        let oversized = "a".repeat(2_001);
        assert_eq!(pack_paragraphs([oversized.as_str()]), vec![oversized]);
        assert!(pack_paragraphs([]).is_empty());
    }

    #[test]
    fn import_lines_pack_to_limit_without_inserting_paragraph_gaps() {
        let first = "é".repeat(1_000);
        let second = "界".repeat(999);
        assert_eq!(
            pack_import_lines([first.as_str(), second.as_str(), "Next line"]),
            vec![format!("{first}\n{second}"), "Next line".into()]
        );
    }
}
