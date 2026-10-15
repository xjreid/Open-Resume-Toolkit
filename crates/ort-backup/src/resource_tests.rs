//! Resource regressions exercise the real container limit and canonical bytes.
use super::*;

fn content_heavy_tracker() -> Value {
    serde_json::to_value(ort_domain::TrackerEntry {
        company: "Synthetic company".into(),
        title: "Synthetic role".into(),
        location: "Remote".into(),
        date_applied: "2026-10-14".into(),
        status: "applied".into(),
        custom_status: String::new(),
        source_url: String::new(),
        resume: None,
        cover_letter: None,
        cover_contact: None,
        answers: (0..30)
            .map(|_| ort_domain::ApprovedAnswer {
                question: "Q".repeat(2_000),
                answer: "A".repeat(4_000),
            })
            .collect(),
        style: ort_domain::DocumentStyle::default(),
    })
    .unwrap()
}

#[test]
fn near_limit_profile_round_trips_and_oversized_profile_is_rejected() {
    let value = content_heavy_tracker();
    let profile = PortableProfileV1 {
        tracker_entries: (0..360)
            .map(|_| PortableTrackerEntryV1 {
                id: uuid::Uuid::now_v7().to_string(),
                revision: 1,
                value: value.clone(),
            })
            .collect(),
        ..PortableProfileV1::default()
    };
    let size = serialized_size(&profile, MAX_PAYLOAD_BYTES).unwrap();
    assert!(size > MAX_PAYLOAD_BYTES * 95 / 100);
    let passphrase = BackupPassphrase::new("synthetic near-limit archive".into()).unwrap();
    let bytes = create_backup(
        &passphrase,
        BackupExportRequestV1 {
            app_version: "0.0.0-dev".into(),
            created_at: "2026-10-14T12:00:00Z".into(),
            profile,
        },
    )
    .unwrap();
    let restored = restore_backup(&bytes, &passphrase).unwrap();
    assert_eq!(restored.profile.tracker_entries.len(), 360);
    assert_eq!(restored.profile.tracker_entries[0].value, value);
    drop(bytes);
    let mut oversized = restored.profile;
    for _ in 0..20 {
        oversized.tracker_entries.push(PortableTrackerEntryV1 {
            id: uuid::Uuid::now_v7().to_string(),
            revision: 1,
            value: value.clone(),
        });
    }
    assert!(matches!(
        create_backup(
            &passphrase,
            BackupExportRequestV1 {
                app_version: "0.0.0-dev".into(),
                created_at: "2026-10-14T12:00:00Z".into(),
                profile: oversized,
            }
        ),
        Err(BackupError::InvalidContent)
    ));
}
