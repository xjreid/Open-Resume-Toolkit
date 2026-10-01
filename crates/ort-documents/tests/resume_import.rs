use ort_documents::{
    import::{BlockKind, InputFormat, ValidatedExtraction},
    resume_import::map_resume,
};
use ort_domain::DocumentLimits;
use serde_json::json;

fn extraction(lines: &[(&str, BlockKind)]) -> ValidatedExtraction {
    ValidatedExtraction::decode(&serde_json::to_vec(&json!({
        "version": 1, "format": "pdf", "pageCount": 1,
        "blocks": lines.iter().map(|(text, kind)| json!({"page": 1, "kind": kind, "text": text})).collect::<Vec<_>>()
    })).unwrap(), InputFormat::Pdf).unwrap()
}

#[test]
fn maps_contacts_roles_dates_bullets_and_skill_categories_to_canvas_fields() {
    use BlockKind::{Heading as H, ListItem as L, Paragraph as P};
    let source = extraction(&[
        ("Zoë Example", H),
        (
            "zoe@example.org | +1 (212) 555-1234 | Boston, MA | https://example.org",
            P,
        ),
        ("Summary", H),
        ("Builds accessible tools.", P),
        ("Experience", H),
        ("ACME CORP", P),
        ("Senior Engineer", P),
        ("Jan 2022 – Present", P),
        ("Remote", P),
        ("• Built accessible software", L),
        ("SECOND COMPANY | Engineer | New York, NY | 2019 – 2021", P),
        ("- Improved reliability", L),
        ("Education", H),
        ("Example University", P),
        ("BS Computer Science", P),
        ("2019", P),
        ("Technical Skills", H),
        ("Languages: Rust, TypeScript", P),
    ]);
    let document = map_resume(&source).unwrap();
    document.validate(DocumentLimits::default()).unwrap();
    assert_eq!(document.schema_version, 2);
    assert_eq!(document.contact.full_name, "Zoë Example");
    assert_eq!(document.contact.email, "zoe@example.org");
    assert_eq!(document.contact.phone, "+1 (212) 555-1234");
    assert_eq!(document.contact.location, "Boston, MA\nhttps://example.org");
    let experience = &document.sections[1];
    assert_eq!(experience.entries.len(), 2);
    assert_eq!(experience.entries[0].heading, "ACME CORP");
    assert_eq!(experience.entries[0].subheading, "Senior Engineer");
    assert_eq!(experience.entries[0].date_range, "Jan 2022 – Present");
    assert!(experience.entries[0].dates.as_ref().unwrap().is_empty());
    assert_eq!(experience.entries[0].location, "Remote");
    assert_eq!(
        experience.entries[0].bullets[0].text,
        "Built accessible software"
    );
    assert_eq!(experience.entries[1].heading, "SECOND COMPANY");
    assert_eq!(experience.entries[1].subheading, "Engineer");
    assert_eq!(experience.entries[1].location, "New York, NY");
    assert_eq!(document.sections[2].entries[0].date_range, "2019");
    let skills = &document.sections[3].entries[0];
    assert_eq!(skills.heading, "");
    assert_eq!(skills.fields[0].label, "__ort_body_paragraph__");
    assert_eq!(skills.fields[0].value, "Languages: Rust, TypeScript");
}

#[test]
fn retains_mixed_and_unknown_text_in_visible_body_without_duplicate_hidden_bullets() {
    use BlockKind::{Heading as H, ListItem as L, Paragraph as P};
    let source = extraction(&[
        ("Name: Example Person", P),
        ("Strange unlabeled header: literal <script>", P),
        ("Projects", H),
        ("A tool", P),
        ("• First body line", L),
        ("continued on the following line", P),
        ("• Another contribution", L),
        ("COMMUNITY SERVICE", H),
        ("Volunteered with local groups", P),
    ]);
    let document = map_resume(&source).unwrap();
    document.validate(DocumentLimits::default()).unwrap();
    assert_eq!(document.sections[0].heading, "Imported information");
    assert!(
        document.sections[0].entries[0].fields[0]
            .value
            .contains("<script>")
    );
    let project = &document.sections[1].entries[0];
    assert!(project.bullets.is_empty());
    assert_eq!(project.fields[0].label, "__ort_body_paragraph__");
    assert_eq!(
        project.fields[0].value,
        "- First body line\ncontinued on the following line\n- Another contribution"
    );
    assert_eq!(document.sections[2].heading, "COMMUNITY SERVICE");
}

#[test]
fn source_is_immutable_and_oversized_content_is_retained_for_correction() {
    use BlockKind::{Heading as H, Paragraph as P};
    let long = "word ".repeat(600);
    let source = extraction(&[("Summary", H), (&long, P)]);
    let document = map_resume(&source).unwrap();
    assert_eq!(source.blocks()[1].text, long);
    assert_eq!(document.sections[0].entries[0].fields[0].value, long.trim());
    assert!(document.validate(DocumentLimits::default()).is_err());
}

#[test]
fn preserves_header_taglines_and_maps_labeled_or_inline_contacts() {
    use BlockKind::Paragraph as P;
    let source = extraction(&[
        ("Resume", P),
        ("Jane Example", P),
        ("Software Engineer", P),
        (
            "Email: jane@example.org | Phone: +1 212 555 1234 | Location: Brooklyn, NY",
            P,
        ),
        ("Experience", P),
        ("Example Company | Engineer | 2020 – Present", P),
    ]);
    let document = map_resume(&source).unwrap();
    assert_eq!(document.contact.full_name, "Jane Example");
    assert_eq!(document.contact.email, "jane@example.org");
    assert_eq!(document.contact.phone, "+1 212 555 1234");
    assert_eq!(document.contact.location, "Brooklyn, NY");
    assert_eq!(
        document.sections[0].entries[0].fields[0].value,
        "Software Engineer"
    );
    let source = extraction(&[("Jane Example jane@example.org +1 (212) 555-1234", P)]);
    let document = map_resume(&source).unwrap();
    assert_eq!(document.contact.full_name, "Jane Example");
    assert_eq!(document.contact.email, "jane@example.org");
    assert_eq!(document.contact.phone, "+1 (212) 555-1234");
}

#[test]
fn uppercase_skill_items_remain_skills_and_custom_sections_remain_literal() {
    use BlockKind::{ListItem as L, Paragraph as P};
    let source = extraction(&[
        ("Name: Example Person", P),
        ("Skills", P),
        ("AWS", L),
        ("SQL", P),
        ("Awards", P),
        ("Community Award", P),
    ]);
    let document = map_resume(&source).unwrap();
    assert_eq!(document.sections.len(), 2);
    assert_eq!(document.sections[0].entries.len(), 1);
    assert_eq!(
        document.sections[0].entries[0].fields[0].value,
        "- AWS\nSQL"
    );
    assert_eq!(document.sections[1].heading, "Awards");
}

#[test]
fn refuses_a_serialized_candidate_that_cannot_be_displayed_without_truncation() {
    use BlockKind::Heading as H;
    let text = "技能\n".repeat(9_000);
    // Stay within the independently enforced extracted block/total text bounds.
    let source = extraction(&[(&text, H)]);
    assert!(map_resume(&source).is_err());
    assert_eq!(source.blocks()[0].text, text);
}

fn positioned(lines: &[(&str, BlockKind, i32, i32, i32, u32)]) -> ValidatedExtraction {
    let blocks: Vec<_> = lines
        .iter()
        .map(|(text, kind, left, top, right, font)| {
            json!({
                "page": 1, "kind": kind, "text": text,
                "layout": {"left": left * 1000, "top": top * 1000, "right": right * 1000,
                    "bottom": (top + 10) * 1000, "fontSize": font * 1000}
            })
        })
        .collect();
    ValidatedExtraction::decode(
        &serde_json::to_vec(&json!({
            "version":1, "format":"pdf", "pageCount":1, "blocks":blocks
        }))
        .unwrap(),
        InputFormat::Pdf,
    )
    .unwrap()
}

#[test]
fn positions_populate_all_six_header_slots_and_keep_plain_body_out_of_headers() {
    use BlockKind::{Heading as H, Paragraph as P};
    let source = positioned(&[
        ("Experience", H, 72, 70, 200, 14),
        ("Example Company", P, 72, 100, 180, 12),
        ("Rust, TypeScript", P, 205, 100, 300, 11),
        ("Boston, MA", P, 465, 100, 530, 11),
        ("Senior Engineer", P, 72, 119, 190, 11),
        ("Jan 2022 – Present", P, 420, 119, 530, 11),
        ("Hybrid", P, 465, 138, 530, 10),
        (
            "Created resilient services for distributed teams.",
            P,
            72,
            170,
            510,
            11,
        ),
        (
            "In 2023 the team expanded its customer support.",
            P,
            72,
            186,
            510,
            11,
        ),
        ("Another Company", P, 72, 225, 190, 12),
        ("Engineer", P, 72, 244, 150, 11),
        ("2019–2021", P, 465, 244, 530, 11),
        (
            "Delivered tools for partner organizations.",
            P,
            72,
            273,
            510,
            11,
        ),
        ("Education", H, 72, 310, 200, 14),
        ("Example University", P, 72, 340, 200, 12),
        ("BS Computer Science", P, 72, 359, 250, 11),
        ("2019", P, 490, 359, 530, 11),
    ]);
    let document = map_resume(&source).unwrap();
    document.validate(DocumentLimits::default()).unwrap();
    assert_eq!(document.sections.len(), 2);
    assert_eq!(document.sections[0].entries.len(), 2);
    let first = &document.sections[0].entries[0];
    assert_eq!(first.heading, "Example Company");
    assert_eq!(first.subheading, "Senior Engineer");
    assert_eq!(first.location, "Boston, MA");
    assert_eq!(first.date_range, "Jan 2022 – Present");
    assert_eq!(
        first
            .fields
            .iter()
            .find(|field| field.label == "Details")
            .unwrap()
            .value,
        "Rust, TypeScript"
    );
    assert_eq!(
        first
            .fields
            .iter()
            .find(|field| field.label == "Extra")
            .unwrap()
            .value,
        "Hybrid"
    );
    assert_eq!(
        first
            .fields
            .iter()
            .find(|field| field.label == "__ort_body_paragraph__")
            .unwrap()
            .value,
        "Created resilient services for distributed teams.\nIn 2023 the team expanded its customer support."
    );
    assert_eq!(document.sections[0].entries[1].heading, "Another Company");
    assert_eq!(
        document.sections[1].entries[0].subheading,
        "BS Computer Science"
    );
}

#[test]
fn nonbulleted_skill_categories_and_wrapped_lists_share_one_visible_body() {
    use BlockKind::{Heading as H, ListItem as L, Paragraph as P};
    for source in [
        extraction(&[
            ("Skills", H),
            ("Programming Languages: Rust, TypeScript", H),
            ("Languages", P),
            ("Python, SQL", P),
            ("", P),
            ("TOOLS", H),
            ("Git, Docker, Linux", P),
            ("• Cloud platforms", L),
            ("Education", H),
            ("Example University", P),
        ]),
        positioned(&[
            ("Skills", H, 72, 70, 200, 14),
            (
                "Programming Languages: Rust, TypeScript",
                P,
                72,
                100,
                450,
                11,
            ),
            ("Python, SQL", P, 72, 116, 250, 11),
            ("TOOLS", H, 72, 148, 200, 11),
            ("Git, Docker, Linux", P, 72, 164, 300, 11),
            ("Education", H, 72, 200, 200, 14),
            ("Example University", P, 72, 230, 250, 12),
        ]),
    ] {
        let document = map_resume(&source).unwrap();
        document.validate(DocumentLimits::default()).unwrap();
        assert_eq!(document.sections.len(), 2);
        let skills = &document.sections[0];
        assert_eq!(skills.entries.len(), 1);
        let entry = &skills.entries[0];
        assert!(entry.heading.is_empty() && entry.subheading.is_empty());
        assert_eq!(entry.fields.len(), 1);
        assert_eq!(entry.fields[0].label, "__ort_body_paragraph__");
        assert!(
            entry.fields[0]
                .value
                .contains("Programming Languages: Rust, TypeScript\n")
        );
        assert!(entry.fields[0].value.contains("TOOLS\nGit, Docker, Linux"));
        assert!(entry.fields[0].value.contains("Python, SQL"));
        assert_eq!(document.sections[1].heading, "Education");
    }
}

#[test]
fn prose_dates_and_short_nonbulleted_descriptions_are_body_content() {
    use BlockKind::{Heading as H, Paragraph as P};
    let document = map_resume(&extraction(&[
        ("Projects", H),
        ("Example Project", P),
        ("Built tools in 2020 for 2021 planning.", P),
        ("2022 brought further improvements", P),
        ("TECHNOLOGIES: Rust, SQL", P),
        ("Summary", H),
        ("In 2023 I led a team of 20.", P),
    ]))
    .unwrap();
    let entry = &document.sections[0].entries[0];
    assert_eq!(document.sections[0].entries.len(), 1);
    assert!(entry.date_range.is_empty() && entry.subheading.is_empty());
    assert_eq!(
        entry.fields[0].value,
        "Built tools in 2020 for 2021 planning.\n2022 brought further improvements\nTECHNOLOGIES: Rust, SQL"
    );
}

#[test]
fn custom_section_style_separates_sections_without_turning_companies_into_sections() {
    use BlockKind::{Heading as H, ListItem as L, Paragraph as P};
    let source = positioned(&[
        ("Experience", H, 72, 70, 200, 14),
        ("ACME CORPORATION", P, 72, 100, 250, 12),
        ("Engineer", P, 72, 119, 150, 11),
        ("2020–Present", P, 465, 119, 530, 11),
        ("• Built reliable tools", L, 80, 150, 350, 11),
        ("COMMUNITY ENGAGEMENT", P, 72, 190, 350, 14),
        ("Local Group", P, 72, 220, 250, 12),
        ("• Helped community members", L, 80, 250, 350, 11),
        ("Skills", H, 72, 290, 200, 14),
        ("AWS, SQL, Git", P, 72, 320, 350, 11),
    ]);
    let document = map_resume(&source).unwrap();
    document.validate(DocumentLimits::default()).unwrap();
    assert_eq!(
        document
            .sections
            .iter()
            .map(|section| section.heading.as_str())
            .collect::<Vec<_>>(),
        ["Experience", "COMMUNITY ENGAGEMENT", "Skills"]
    );
    assert_eq!(document.sections[0].entries[0].heading, "ACME CORPORATION");
}

#[test]
fn skill_category_labels_and_colon_section_headings_do_not_split_skills() {
    use BlockKind::{Heading as H, Paragraph as P};
    let document = map_resume(&extraction(&[
        ("Skills", H),
        ("Leadership", P),
        ("Team management, coaching", P),
        ("Communication", P),
        ("Writing, presentations", P),
        ("Education:", H),
        ("Example University", P),
    ]))
    .unwrap();
    assert_eq!(document.sections.len(), 2);
    assert_eq!(document.sections[0].entries.len(), 1);
    assert_eq!(
        document.sections[0].entries[0].fields[0].value,
        "Leadership\nTeam management, coaching\nCommunication\nWriting, presentations"
    );
    assert_eq!(document.sections[1].heading, "Education");
}

#[test]
fn paragraphs_with_extra_spacing_do_not_become_new_entries() {
    use BlockKind::{Heading as H, Paragraph as P};
    let document = map_resume(&positioned(&[
        ("Projects", H, 72, 70, 200, 14),
        ("Example Project", P, 72, 100, 200, 12),
        ("Built useful tools for teams.", P, 72, 140, 450, 11),
        ("Additional technical work", P, 72, 180, 450, 11),
        ("Across multiple platforms", P, 72, 215, 450, 11),
    ]))
    .unwrap();
    assert_eq!(document.sections[0].entries.len(), 1);
    assert_eq!(
        document.sections[0].entries[0].fields[0].value,
        "Built useful tools for teams.\nAdditional technical work\nAcross multiple platforms"
    );
}

#[test]
fn repeated_page_heading_keeps_a_single_logical_section() {
    let source = ValidatedExtraction::decode(
        &serde_json::to_vec(&json!({
            "version":1, "format":"pdf", "pageCount":2, "blocks":[
                {"page":1,"kind":"heading","text":"Skills"},
                {"page":1,"kind":"paragraph","text":"Languages: Rust, SQL"},
                {"page":2,"kind":"heading","text":"Skills"},
                {"page":2,"kind":"paragraph","text":"Tools: Git, Docker"}
            ]
        }))
        .unwrap(),
        InputFormat::Pdf,
    )
    .unwrap();
    let document = map_resume(&source).unwrap();
    assert_eq!(document.sections.len(), 1);
    assert_eq!(document.sections[0].entries.len(), 1);
    assert_eq!(
        document.sections[0].entries[0].fields[0].value,
        "Languages: Rust, SQL\nTools: Git, Docker"
    );
}

#[test]
fn company_abbreviations_and_same_row_details_remain_headers() {
    use BlockKind::{Heading as H, Paragraph as P};
    let document = map_resume(&positioned(&[
        ("Experience", H, 72, 70, 200, 14),
        ("Example Systems Inc.", P, 72, 100, 210, 12),
        ("Tech: Rust, SQL", P, 230, 100, 330, 11),
        ("Remote", P, 470, 100, 530, 11),
        ("Engineer", P, 72, 119, 200, 11),
        ("2020–Present", P, 450, 119, 530, 11),
        ("Built robust systems.", P, 72, 155, 450, 11),
    ]))
    .unwrap();
    let entry = &document.sections[0].entries[0];
    assert_eq!(entry.heading, "Example Systems Inc.");
    assert_eq!(
        entry
            .fields
            .iter()
            .find(|field| field.label == "Details")
            .unwrap()
            .value,
        "Tech: Rust, SQL"
    );
    assert_eq!(entry.subheading, "Engineer");
    assert_eq!(entry.location, "Remote");
}
