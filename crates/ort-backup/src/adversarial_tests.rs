//! Synthetic hostile containers exercise the public reader, including payloads
//! authenticated with a known test passphrase. Writer rejection alone does not
//! prove the reader rejects an independently constructed archive.

use super::*;
use serde_json::json;

fn request() -> BackupExportRequestV1 {
    BackupExportRequestV1 {
        app_version: "0.0.0-dev".to_owned(),
        created_at: "2026-09-05T12:00:00Z".to_owned(),
        profile: PortableProfileV1 {
            master_draft: Some(PortableResumeRevisionV1 {
                revision: 1,
                document: ResumeDocument::empty("Synthetic hostile-backup control"),
            }),
            ..PortableProfileV1::default()
        },
    }
}

#[test]
#[ignore = "bounded mutation campaign; run explicitly with ORT_RUN_BACKUP_MUTATION_TESTS=1"]
fn bounded_restore_mutation_campaign() {
    assert_eq!(
        std::env::var("ORT_RUN_BACKUP_MUTATION_TESTS").as_deref(),
        Ok("1")
    );
    let passphrase =
        BackupPassphrase::new("synthetic mutation campaign passphrase".into()).unwrap();
    let valid = create_backup(&passphrase, request()).unwrap();
    assert!(restore_backup(&valid, &passphrase).is_ok());
    let mut state = 0x6f72_742d_6d31_0001_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut header_rejections = 0;
    for _ in 0..10_000 {
        let mut candidate = valid.clone();
        for _ in 0..=(next() % 8) {
            let offset = usize::try_from(next() % u64::try_from(HEADER_LEN).unwrap()).unwrap();
            candidate[offset] ^= u8::try_from(next() % 255 + 1).unwrap();
        }
        // Never derive attacker-selected expensive KDF parameters in this
        // bounded campaign. Public restore must agree on every rejected header.
        if inspect_backup(&candidate).is_err() {
            assert_eq!(
                restore_backup(&candidate, &passphrase),
                Err(BackupError::InvalidBackup)
            );
            header_rejections += 1;
        }
    }
    assert!(header_rejections > 5_000);
    for _ in 0..128 {
        let mut candidate = valid.clone();
        let offset = HEADER_LEN
            + usize::try_from(next() % u64::try_from(valid.len() - HEADER_LEN).unwrap()).unwrap();
        candidate[offset] ^= u8::try_from(next() % 255 + 1).unwrap();
        assert!(inspect_backup(&candidate).is_ok());
        assert_eq!(
            restore_backup(&candidate, &passphrase),
            Err(BackupError::InvalidBackup)
        );
    }
    // A trailing valid control proves failures did not poison the reader.
    assert!(restore_backup(&valid, &passphrase).is_ok());
    println!(
        "PASS 10000 seeded header mutations ({header_rejections} rejected before KDF), 128 authenticated-container ciphertext/tag mutations, valid controls before/after"
    );
}

#[test]
fn truncated_extended_and_out_of_policy_containers_are_rejected() {
    let passphrase = BackupPassphrase::new("synthetic boundary passphrase".to_owned()).unwrap();
    let backup = create_backup(&passphrase, request()).unwrap();
    // Every possible truncation includes header, payload, and authentication tag.
    for length in 0..backup.len() {
        assert_eq!(
            inspect_backup(&backup[..length]),
            Err(BackupError::InvalidBackup)
        );
        assert_eq!(
            restore_backup(&backup[..length], &passphrase),
            Err(BackupError::InvalidBackup)
        );
    }
    for suffix in [&[0_u8][..], backup.as_slice()] {
        let mut extended = backup.clone();
        extended.extend_from_slice(suffix);
        assert_eq!(
            restore_backup(&extended, &passphrase),
            Err(BackupError::InvalidBackup)
        );
    }
    for offset in [0, 4, 6, 8, 9, 10, 11, 24, 25, 26, 27] {
        let mut changed = backup.clone();
        changed[offset] = 255;
        assert_eq!(
            restore_backup(&changed, &passphrase),
            Err(BackupError::InvalidBackup)
        );
    }
    for (offset, values) in [
        (
            12,
            vec![0, MIN_MEMORY_KIB - 1, MAX_MEMORY_KIB + 1, u32::MAX],
        ),
        (
            16,
            vec![0, MIN_ITERATIONS - 1, MAX_ITERATIONS + 1, u32::MAX],
        ),
        (20, vec![0, 1, WRITER_LANES - 1, WRITER_LANES + 1, u32::MAX]),
    ] {
        for value in values {
            let mut changed = backup.clone();
            changed[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
            assert_eq!(
                restore_backup(&changed, &passphrase),
                Err(BackupError::InvalidBackup)
            );
        }
    }
    for declared_length in [
        0,
        (TAG_LEN - 1) as u64,
        (MAX_PAYLOAD_BYTES + TAG_LEN + 1) as u64,
        u64::MAX,
    ] {
        let mut changed = backup.clone();
        changed[28..36].copy_from_slice(&declared_length.to_be_bytes());
        assert_eq!(
            restore_backup(&changed, &passphrase),
            Err(BackupError::InvalidBackup)
        );
    }
    // Exercise an actual oversized allocation, not only a dishonest length word.
    let mut oversized = backup.clone();
    oversized.resize(HEADER_LEN + MAX_PAYLOAD_BYTES + TAG_LEN + 1, 0);
    oversized[28..36].copy_from_slice(&((MAX_PAYLOAD_BYTES + TAG_LEN + 1) as u64).to_be_bytes());
    assert_eq!(
        restore_backup(&oversized, &passphrase),
        Err(BackupError::InvalidBackup)
    );
    assert!(
        restore_backup(&backup, &passphrase).is_ok(),
        "valid positive control"
    );
}

#[test]
fn authenticated_hostile_payloads_share_the_public_invalid_backup_error() {
    let passphrase =
        BackupPassphrase::new("synthetic authenticated fixture passphrase".to_owned()).unwrap();
    let backup = create_backup(&passphrase, request()).unwrap();
    let valid = restore_backup(&backup, &passphrase).unwrap();
    let payload = serde_json::to_value(&valid).unwrap();

    let salt = [0x53; SALT_LEN];
    let cipher = cipher_for(
        &passphrase,
        &salt,
        WRITER_MEMORY_KIB,
        WRITER_ITERATIONS,
        WRITER_LANES,
    )
    .unwrap();
    let mut plaintexts = hostile_payloads(&payload);
    plaintexts.push(("malformed JSON", b"{".to_vec()));
    plaintexts.push(("invalid UTF-8", vec![0xff]));
    let valid_json = serde_json::to_string(&valid).unwrap();
    let duplicate = format!(
        "{{\"manifest\":{},{}",
        serde_json::to_string(&valid.manifest).unwrap(),
        &valid_json[1..]
    );
    plaintexts.push(("duplicate manifest", duplicate.into_bytes()));
    let deep = valid_json.replace(
        "\"settings\":{}",
        &format!(
            "\"settings\":{{\"appearance.theme\":{{\"revision\":1,\"value\":{}}}}}",
            "[".repeat(140) + "0" + &"]".repeat(140)
        ),
    );
    assert_ne!(deep, valid_json, "nest the setting's arbitrary JSON value");
    plaintexts.push(("excessive JSON nesting", deep.into_bytes()));
    plaintexts.push((
        "trailing JSON value",
        (valid_json.clone() + " null").into_bytes(),
    ));
    // A valid independently encrypted fixture must pass through the same helper.
    plaintexts.push(("positive control", valid_json.into_bytes()));
    for (index, (name, plaintext)) in plaintexts.into_iter().enumerate() {
        let mut nonce = [0x74; NONCE_LEN];
        nonce[..8].copy_from_slice(&u64::try_from(index).unwrap().to_be_bytes());
        let mut container = build_header(
            1, // This authenticated mutation corpus embeds the unchanged v1.1 payload.
            WRITER_MEMORY_KIB,
            WRITER_ITERATIONS,
            WRITER_LANES,
            u64::try_from(plaintext.len() + TAG_LEN).unwrap(),
            &salt,
            &nonce,
        );
        let encrypted = cipher
            .encrypt(
                &XNonce::try_from(nonce.as_slice()).unwrap(),
                Payload {
                    msg: &plaintext,
                    aad: &container,
                },
            )
            .unwrap();
        container.extend_from_slice(&encrypted);
        assert!(inspect_backup(&container).is_ok(), "{name}: valid header");
        let result = restore_backup(&container, &passphrase);
        if name == "positive control" {
            assert_eq!(result.unwrap(), valid);
        } else {
            assert_eq!(result, Err(BackupError::InvalidBackup), "{name}");
        }
    }
}

fn hostile_payloads(payload: &Value) -> Vec<(&'static str, Vec<u8>)> {
    let mut cases = Vec::new();
    for pointer in [
        "/manifest/formatMajor",
        "/manifest/formatMinor",
        "/manifest/databaseSchema",
        "/manifest/documentSchema",
    ] {
        let mut changed = payload.clone();
        *changed.pointer_mut(pointer).unwrap() = json!(99);
        cases.push((pointer, changed));
    }
    for (name, pointer, value) in [
        ("inventory", "/manifest/inventory/masterDrafts", json!(0)),
        ("hash", "/manifest/profileSha256", json!("0".repeat(64))),
        ("timestamp", "/manifest/createdAt", json!("not-a-timestamp")),
        ("version", "/manifest/appVersion", json!("../hostile")),
        ("revision", "/profile/masterDraft/revision", json!(0)),
        (
            "document",
            "/profile/masterDraft/document/title",
            json!("x".repeat(DocumentLimits::default().field_characters + 1)),
        ),
        ("unknown credential", "/profile", {
            let mut profile = payload["profile"].clone();
            profile["databaseKey"] = json!("SYNTHETIC-EXCLUDED-KEY");
            profile
        }),
        (
            "secret setting",
            "/profile/settings",
            json!({"provider.api_token": {"revision": 1, "value": "SYNTHETIC-EXCLUDED-CREDENTIAL"}}),
        ),
        (
            "oversized setting",
            "/profile/settings",
            json!({"appearance.theme": {"revision": 1, "value": "x".repeat(MAX_SETTING_BYTES + 1)}}),
        ),
    ] {
        let mut changed = payload.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        if pointer.starts_with("/profile/") || pointer == "/profile" {
            // Keep hash/inventory consistent so domain/schema policy must reject
            // the hostile records, even when a producer knows the passphrase.
            if let Ok(profile) =
                serde_json::from_value::<PortableProfileV1>(changed["profile"].clone())
            {
                changed["manifest"]["profileSha256"] = json!(hex::encode(Sha256::digest(
                    serde_json::to_vec(&profile).unwrap()
                )));
            }
            changed["manifest"]["inventory"]["settings"] =
                json!(changed["profile"]["settings"].as_object().unwrap().len());
        }
        cases.push((name, changed));
    }

    cases
        .into_iter()
        .map(|(name, value)| (name, serde_json::to_vec(&value).unwrap()))
        .collect()
}
