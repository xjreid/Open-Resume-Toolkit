//! Conservative, ephemeral list units. Persisted values remain free text.
use ort_domain::NamedField;

pub(super) struct ListItems<'a> {
    pub items: Vec<&'a str>,
    pub separator: &'a str,
}

pub(super) fn list_items(field: &NamedField) -> ListItems<'_> {
    let text = field.value.as_str();
    let atomic = || ListItems {
        items: vec![text],
        separator: "",
    };
    // Prefixes may qualify every item, so never strip or redistribute them.
    if text.contains(':') || text.contains('\\') || has_prose_prefix(text) {
        return atomic();
    }
    let mut stack = Vec::new();
    let mut delimiter = None;
    let mut boundaries = Vec::new();
    for (i, c) in text.char_indices() {
        match c {
            '(' | '[' => stack.push(c),
            ')' | ']' => {
                if stack.pop() != Some(if c == ')' { '(' } else { '[' }) {
                    return atomic();
                }
            }
            ',' | ';' | '|' | '\n' if stack.is_empty() => {
                if delimiter.is_some_and(|previous| previous != c) {
                    return atomic();
                }
                delimiter = Some(c);
                boundaries.push(i);
            }
            _ => {}
        }
    }
    if !stack.is_empty() || boundaries.is_empty() {
        return atomic();
    }
    let mut start = 0;
    let mut items = Vec::new();
    for end in boundaries
        .iter()
        .copied()
        .chain(std::iter::once(text.len()))
    {
        let item = text[start..end].trim();
        if item.is_empty() {
            return atomic();
        }
        items.push(item);
        start = end + 1;
    }
    // Retain the source's first separator and surrounding whitespace.
    let first = boundaries[0];
    let left = text[..first].trim_end().len();
    let right = first + 1 + (text[first + 1..].len() - text[first + 1..].trim_start().len());
    ListItems {
        items,
        separator: &text[left..right],
    }
}

fn has_prose_prefix(text: &str) -> bool {
    let lower = text.trim_start().to_lowercase();
    [
        "- ",
        "* ",
        "• ",
        "proficient in ",
        "familiar with ",
        "experience with ",
        "experienced with ",
        "experienced in ",
        "knowledge of ",
        "including ",
        "such as ",
        "skills include ",
        "technologies include ",
        "coursework includes ",
        "courses include ",
        "relevant coursework ",
    ]
    .iter()
    .any(|prefix| lower.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ort_domain::EntityId;
    fn field(value: &str) -> NamedField {
        NamedField {
            id: EntityId::new(),
            order: 0,
            label: "Skills".into(),
            value: value.into(),
            is_skill: true,
            list_kind: None,
        }
    }
    #[test]
    fn separates_only_uniform_top_level_lists() {
        let source = field("C/C++, SQL (joins, windows), CI-CD");
        let list = list_items(&source);
        assert_eq!(list.items, ["C/C++", "SQL (joins, windows)", "CI-CD"]);
        assert_eq!(list.separator, ", ");
        for (text, expected, separator) in [
            (
                "Rust | SQL [joins, windows] | C/C++",
                vec!["Rust", "SQL [joins, windows]", "C/C++"],
                " | ",
            ),
            ("Rust\r\nSQL\r\nC/C++", vec!["Rust", "SQL", "C/C++"], "\r\n"),
            (
                "SQL (joins [I, II]); Rust",
                vec!["SQL (joins [I, II])", "Rust"],
                "; ",
            ),
        ] {
            let source = field(text);
            let list = list_items(&source);
            assert_eq!(list.items, expected);
            assert_eq!(list.separator, separator);
        }
        for text in [
            "Languages: Rust, SQL",
            "Proficient in Rust, SQL",
            "Coursework includes Algorithms; Systems",
            "- Rust\n- SQL",
            "Rust, SQL | Go",
            "SQL (joins, Rust",
            "Rust,,SQL",
            "Rust,SQL)",
        ] {
            assert_eq!(list_items(&field(text)).items, [text]);
        }
    }
}
