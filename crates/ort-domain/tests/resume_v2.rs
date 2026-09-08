use ort_domain::{
    CalendarDate, DateEnd, DocumentLimits, EntityId, Link, ResumeDate, ResumeDocument, ResumeEntry,
    ResumeSection, ValidationError,
};

fn legacy() -> ResumeDocument {
    let mut document = ResumeDocument::empty("Synthetic");
    document.contact.links.push(Link {
        id: None,
        order: None,
        label: "Portfolio".into(),
        url: "https://example.org".into(),
    });
    document.sections.push(ResumeSection {
        id: EntityId::new(),
        order: 0,
        heading: "Experience".into(),
        entries: vec![ResumeEntry {
            id: EntityId::new(),
            order: 0,
            heading: "Synthetic role".into(),
            subheading: String::new(),
            date_range: "Circa 2020 / ongoing".into(),
            dates: None,
            location: String::new(),
            fields: vec![],
            bullets: vec![],
            links: vec![],
        }],
    });
    document
}
fn date() -> ResumeDate {
    ResumeDate {
        id: EntityId::new(),
        order: 0,
        label: "Graduation".into(),
        start: Some(CalendarDate {
            year: 2027,
            month: None,
            expected: true,
        }),
        end: None,
    }
}
#[test]
fn upgrading_is_explicit_lossless_and_idempotent_and_legacy_bytes_survive_reads() {
    let original = legacy();
    let bytes = serde_json::to_vec(&original).unwrap();
    let decoded: ResumeDocument = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
    let upgraded = original.upgraded_v2().unwrap();
    assert_eq!(original.schema_version, 1);
    assert_eq!(upgraded.schema_version, 2);
    assert_eq!(upgraded.document_id, original.document_id);
    assert_eq!(
        upgraded.sections[0].entries[0].date_range,
        "Circa 2020 / ongoing"
    );
    assert_eq!(upgraded.sections[0].entries[0].dates, Some(vec![]));
    assert_eq!(upgraded.contact.links[0].order, Some(0));
    assert!(upgraded.contact.links[0].id.is_some());
    assert_eq!(upgraded.upgraded_v2().unwrap(), upgraded);
    assert_eq!(serde_json::to_vec(&original).unwrap(), bytes);
}
#[test]
fn schema_shapes_ids_order_and_date_bounds_are_enforced() {
    let mut document = legacy().upgraded_v2().unwrap();
    document.sections[0].entries[0].date_range.clear();
    document.sections[0].entries[0].dates = Some(vec![date()]);
    let valid = serde_json::to_value(&document).unwrap();
    for pointer in [
        "/contact/links/0/id",
        "/contact/links/0/order",
        "/sections/0/entries/0/dates",
    ] {
        let mut bad = valid.clone();
        *bad.pointer_mut(pointer).unwrap() = serde_json::Value::Null;
        assert!(
            serde_json::from_value::<ResumeDocument>(bad).is_err(),
            "{pointer}"
        );
    }
    for (pointer, value) in [
        ("/schemaVersion", serde_json::json!(1)),
        ("/schemaVersion", serde_json::json!(3)),
        ("/contact/links/0/order", serde_json::json!(1)),
        (
            "/contact/links/0/id",
            serde_json::json!(document.document_id),
        ),
        (
            "/sections/0/entries/0/dates/0/id",
            serde_json::json!(document.document_id),
        ),
        ("/sections/0/entries/0/dates/0/order", serde_json::json!(1)),
        (
            "/sections/0/entries/0/dates/0/start/year",
            serde_json::json!(0),
        ),
        (
            "/sections/0/entries/0/dates/0/start/month",
            serde_json::json!(13),
        ),
        (
            "/sections/0/entries/0/dateRange",
            serde_json::json!("conflicting free text"),
        ),
    ] {
        let mut bad = valid.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(
            serde_json::from_value::<ResumeDocument>(bad)
                .unwrap()
                .validate(DocumentLimits::default())
                .is_err(),
            "{pointer}"
        );
    }
    document.contact.links[0].id = None;
    assert_eq!(
        document.validate(DocumentLimits::default()),
        Err(ValidationError::SchemaMismatch)
    );
}
#[test]
fn date_precision_empty_fields_expected_and_present_are_not_invented() {
    let mut date = date();
    assert_eq!(date.display_text(), "Graduation: Expected 2027");
    date.label.clear();
    date.start = None;
    assert_eq!(date.display_text(), "");
    date.end = Some(DateEnd::Present);
    assert_eq!(date.display_text(), "Present");
    date.start = Some(CalendarDate {
        year: 2020,
        month: Some(9),
        expected: false,
    });
    assert_eq!(date.display_text(), "Sep 2020–Present");
    date.end = Some(DateEnd::Date {
        value: CalendarDate {
            year: 2020,
            month: None,
            expected: false,
        },
    });
    assert!(
        !date.is_reversed(),
        "unknown end month may be after September"
    );
    date.end = Some(DateEnd::Date {
        value: CalendarDate {
            year: 2019,
            month: None,
            expected: false,
        },
    });
    assert!(date.is_reversed());
    let mut document = legacy().upgraded_v2().unwrap();
    document.sections[0].entries[0].date_range.clear();
    document.sections[0].entries[0].dates = Some(vec![date]);
    assert!(
        document.validate(DocumentLimits::default()).is_ok(),
        "reversed dates warn but are savable"
    );
}
