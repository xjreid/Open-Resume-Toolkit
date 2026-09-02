//! Password-protected, portable Open Resume Toolkit backup container v1.
//!
//! The format has a fixed, bounded clear header authenticated as AEAD
//! associated data. Its encrypted payload contains canonical portable records,
//! never a live database, OS-vault key, provider credential, or diagnostic log.

use std::collections::BTreeMap;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use jiff::Timestamp;
use ort_domain::{DocumentLimits, ResumeDocument};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

const MAGIC: &[u8; 4] = b"ORTB";
const FORMAT_MAJOR: u16 = 1;
const FORMAT_MINOR: u16 = 0;
const KDF_ARGON2ID: u8 = 1;
const HEADER_LEN: usize = 76;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
const KEY_LEN: usize = 32;
const WRITER_MEMORY_KIB: u32 = 64 * 1_024;
const WRITER_ITERATIONS: u32 = 3;
const WRITER_LANES: u32 = 4;
const MIN_MEMORY_KIB: u32 = 64 * 1_024;
const MAX_MEMORY_KIB: u32 = 256 * 1_024;
const MIN_ITERATIONS: u32 = 3;
const MAX_ITERATIONS: u32 = 10;
const MAX_PAYLOAD_BYTES: usize = 64 * 1_024 * 1_024;
const MAX_PUBLISHED_RESUMES: usize = 100;
const MAX_SETTINGS: usize = 128;
const MAX_SETTING_BYTES: usize = 64 * 1_024;
const MAX_PASSPHRASE_BYTES: usize = 1_024;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BackupError {
    #[error("the backup passphrase is invalid")]
    InvalidPassphrase,
    #[error("the portable backup content is invalid")]
    InvalidContent,
    #[error("the backup is invalid or the passphrase is incorrect")]
    InvalidBackup,
    #[error("backup cryptography is unavailable")]
    CryptoUnavailable,
}

/// Owned passphrase memory that clears itself when dropped.
pub struct BackupPassphrase(String);

impl BackupPassphrase {
    /// Takes ownership of a bounded non-empty UTF-8 passphrase.
    ///
    /// # Errors
    /// Returns `InvalidPassphrase` for empty values or values over 1024 bytes.
    pub fn new(mut passphrase: String) -> Result<Self, BackupError> {
        if passphrase.is_empty() || passphrase.len() > MAX_PASSPHRASE_BYTES {
            passphrase.zeroize();
            return Err(BackupError::InvalidPassphrase);
        }
        Ok(Self(passphrase))
    }

    fn expose_for<T>(&self, operation: impl FnOnce(&[u8]) -> T) -> T {
        operation(self.0.as_bytes())
    }
}

impl std::fmt::Debug for BackupPassphrase {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("BackupPassphrase([REDACTED])")
    }
}

impl Drop for BackupPassphrase {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableResumeRevisionV1 {
    pub revision: i64,
    pub document: ResumeDocument,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortablePublishedResumeV1 {
    pub published_revision: i64,
    pub draft_revision: i64,
    pub document: ResumeDocument,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableSettingV1 {
    pub revision: i64,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableProfileV1 {
    pub master_draft: Option<PortableResumeRevisionV1>,
    pub published_resumes: Vec<PortablePublishedResumeV1>,
    pub settings: BTreeMap<String, PortableSettingV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupInventoryV1 {
    pub master_drafts: u16,
    pub published_resumes: u16,
    pub settings: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupManifestV1 {
    pub format_major: u16,
    pub format_minor: u16,
    pub app_version: String,
    pub database_schema: u16,
    pub document_schema: u16,
    pub created_at: String,
    pub inventory: BackupInventoryV1,
    pub profile_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortableBackupV1 {
    pub manifest: BackupManifestV1,
    pub profile: PortableProfileV1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BackupExportRequestV1 {
    pub app_version: String,
    pub created_at: String,
    pub profile: PortableProfileV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackupHeaderInfo {
    pub format_major: u16,
    pub format_minor: u16,
    pub memory_kib: u32,
    pub iterations: u32,
    pub lanes: u32,
    pub ciphertext_bytes: u64,
}

/// Creates an authenticated `.ort-backup` byte sequence.
///
/// # Errors
/// Returns a safe validation or cryptographic availability error.
pub fn create_backup(
    passphrase: &BackupPassphrase,
    request: BackupExportRequestV1,
) -> Result<Vec<u8>, BackupError> {
    let mut salt = [0_u8; SALT_LEN];
    let mut nonce = [0_u8; NONCE_LEN];
    getrandom::fill(&mut salt).map_err(|_| BackupError::CryptoUnavailable)?;
    getrandom::fill(&mut nonce).map_err(|_| BackupError::CryptoUnavailable)?;
    create_backup_with_entropy(passphrase, request, salt, nonce)
}

/// Parses and validates the bounded clear header without deriving a key.
///
/// # Errors
/// Returns `InvalidBackup` for unsupported or out-of-policy input.
pub fn inspect_backup(bytes: &[u8]) -> Result<BackupHeaderInfo, BackupError> {
    let parsed = parse_header(bytes)?;
    Ok(parsed.info)
}

/// Authenticates, decrypts, and validates all portable records.
///
/// Wrong passphrases, ciphertext changes, truncation, and malformed encrypted
/// payloads intentionally share the same non-oracular error.
///
/// # Errors
/// Returns `InvalidBackup` for every untrusted-reader failure.
pub fn restore_backup(
    bytes: &[u8],
    passphrase: &BackupPassphrase,
) -> Result<PortableBackupV1, BackupError> {
    restore_backup_inner(bytes, passphrase).map_err(|_| BackupError::InvalidBackup)
}

fn create_backup_with_entropy(
    passphrase: &BackupPassphrase,
    request: BackupExportRequestV1,
    salt: [u8; SALT_LEN],
    nonce: [u8; NONCE_LEN],
) -> Result<Vec<u8>, BackupError> {
    validate_export_metadata(&request.app_version, &request.created_at)?;
    validate_profile(&request.profile)?;
    let profile_json =
        serde_json::to_vec(&request.profile).map_err(|_| BackupError::InvalidContent)?;
    if profile_json.len() > MAX_PAYLOAD_BYTES {
        return Err(BackupError::InvalidContent);
    }
    let manifest = BackupManifestV1 {
        format_major: FORMAT_MAJOR,
        format_minor: FORMAT_MINOR,
        app_version: request.app_version,
        database_schema: 1,
        document_schema: 1,
        created_at: request.created_at,
        inventory: inventory_for(&request.profile)?,
        profile_sha256: hex::encode(Sha256::digest(&profile_json)),
    };
    let payload = PortableBackupV1 {
        manifest,
        profile: request.profile,
    };
    let plaintext =
        Zeroizing::new(serde_json::to_vec(&payload).map_err(|_| BackupError::InvalidContent)?);
    if plaintext.len() > MAX_PAYLOAD_BYTES {
        return Err(BackupError::InvalidContent);
    }
    let ciphertext_len = plaintext
        .len()
        .checked_add(TAG_LEN)
        .ok_or(BackupError::InvalidContent)?;
    let ciphertext_len_u64 =
        u64::try_from(ciphertext_len).map_err(|_| BackupError::InvalidContent)?;
    let header = build_header(
        WRITER_MEMORY_KIB,
        WRITER_ITERATIONS,
        WRITER_LANES,
        ciphertext_len_u64,
        &salt,
        &nonce,
    );
    let cipher = cipher_for(
        passphrase,
        &salt,
        WRITER_MEMORY_KIB,
        WRITER_ITERATIONS,
        WRITER_LANES,
    )?;
    let xnonce = XNonce::try_from(nonce.as_slice()).map_err(|_| BackupError::CryptoUnavailable)?;
    let ciphertext = cipher
        .encrypt(
            &xnonce,
            Payload {
                msg: plaintext.as_slice(),
                aad: &header,
            },
        )
        .map_err(|_| BackupError::CryptoUnavailable)?;
    if ciphertext.len() != ciphertext_len {
        return Err(BackupError::CryptoUnavailable);
    }
    let mut backup = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    backup.extend_from_slice(&header);
    backup.extend_from_slice(&ciphertext);
    Ok(backup)
}

fn restore_backup_inner(
    bytes: &[u8],
    passphrase: &BackupPassphrase,
) -> Result<PortableBackupV1, BackupError> {
    let parsed = parse_header(bytes)?;
    let cipher = cipher_for(
        passphrase,
        parsed.salt,
        parsed.info.memory_kib,
        parsed.info.iterations,
        parsed.info.lanes,
    )?;
    let xnonce = XNonce::try_from(parsed.nonce).map_err(|_| BackupError::InvalidBackup)?;
    let plaintext = Zeroizing::new(
        cipher
            .decrypt(
                &xnonce,
                Payload {
                    msg: parsed.ciphertext,
                    aad: &bytes[..HEADER_LEN],
                },
            )
            .map_err(|_| BackupError::InvalidBackup)?,
    );
    if plaintext.len() > MAX_PAYLOAD_BYTES {
        return Err(BackupError::InvalidBackup);
    }
    let payload: PortableBackupV1 =
        serde_json::from_slice(&plaintext).map_err(|_| BackupError::InvalidBackup)?;
    validate_payload(&payload)?;
    Ok(payload)
}

fn cipher_for(
    passphrase: &BackupPassphrase,
    salt: &[u8],
    memory_kib: u32,
    iterations: u32,
    lanes: u32,
) -> Result<XChaCha20Poly1305, BackupError> {
    let params = Params::new(memory_kib, iterations, lanes, Some(KEY_LEN))
        .map_err(|_| BackupError::CryptoUnavailable)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0_u8; KEY_LEN];
    let derivation =
        passphrase.expose_for(|bytes| argon2.hash_password_into(bytes, salt, &mut key));
    if derivation.is_err() {
        key.zeroize();
        return Err(BackupError::CryptoUnavailable);
    }
    let cipher =
        XChaCha20Poly1305::new_from_slice(&key).map_err(|_| BackupError::CryptoUnavailable)?;
    key.zeroize();
    Ok(cipher)
}

fn build_header(
    memory_kib: u32,
    iterations: u32,
    lanes: u32,
    ciphertext_len: u64,
    salt: &[u8; SALT_LEN],
    nonce: &[u8; NONCE_LEN],
) -> Vec<u8> {
    let mut header = Vec::with_capacity(HEADER_LEN);
    header.extend_from_slice(MAGIC);
    header.extend_from_slice(&FORMAT_MAJOR.to_be_bytes());
    header.extend_from_slice(&FORMAT_MINOR.to_be_bytes());
    header.push(KDF_ARGON2ID);
    header.extend_from_slice(&[0_u8; 3]);
    header.extend_from_slice(&memory_kib.to_be_bytes());
    header.extend_from_slice(&iterations.to_be_bytes());
    header.extend_from_slice(&lanes.to_be_bytes());
    header.push(u8::try_from(SALT_LEN).expect("salt length fits in header"));
    header.push(u8::try_from(NONCE_LEN).expect("nonce length fits in header"));
    header.extend_from_slice(&[0_u8; 2]);
    header.extend_from_slice(&ciphertext_len.to_be_bytes());
    header.extend_from_slice(salt);
    header.extend_from_slice(nonce);
    debug_assert_eq!(header.len(), HEADER_LEN);
    header
}

struct ParsedHeader<'a> {
    info: BackupHeaderInfo,
    salt: &'a [u8],
    nonce: &'a [u8],
    ciphertext: &'a [u8],
}

fn parse_header(bytes: &[u8]) -> Result<ParsedHeader<'_>, BackupError> {
    if bytes.len() < HEADER_LEN || &bytes[..4] != MAGIC {
        return Err(BackupError::InvalidBackup);
    }
    let format_major = read_u16(bytes, 4)?;
    let format_minor = read_u16(bytes, 6)?;
    let memory_kib = read_u32(bytes, 12)?;
    let iterations = read_u32(bytes, 16)?;
    let lanes = read_u32(bytes, 20)?;
    let ciphertext_bytes = read_u64(bytes, 28)?;
    let header_is_valid = format_major == FORMAT_MAJOR
        && format_minor == FORMAT_MINOR
        && bytes[8] == KDF_ARGON2ID
        && bytes[9..12] == [0_u8; 3]
        && (MIN_MEMORY_KIB..=MAX_MEMORY_KIB).contains(&memory_kib)
        && (MIN_ITERATIONS..=MAX_ITERATIONS).contains(&iterations)
        && lanes == WRITER_LANES
        && usize::from(bytes[24]) == SALT_LEN
        && usize::from(bytes[25]) == NONCE_LEN
        && bytes[26..28] == [0_u8; 2];
    if !header_is_valid {
        return Err(BackupError::InvalidBackup);
    }
    let ciphertext_len =
        usize::try_from(ciphertext_bytes).map_err(|_| BackupError::InvalidBackup)?;
    if !(TAG_LEN..=MAX_PAYLOAD_BYTES + TAG_LEN).contains(&ciphertext_len) {
        return Err(BackupError::InvalidBackup);
    }
    let expected_len = HEADER_LEN
        .checked_add(ciphertext_len)
        .ok_or(BackupError::InvalidBackup)?;
    if bytes.len() != expected_len {
        return Err(BackupError::InvalidBackup);
    }
    Ok(ParsedHeader {
        info: BackupHeaderInfo {
            format_major,
            format_minor,
            memory_kib,
            iterations,
            lanes,
            ciphertext_bytes,
        },
        salt: &bytes[36..52],
        nonce: &bytes[52..HEADER_LEN],
        ciphertext: &bytes[HEADER_LEN..],
    })
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, BackupError> {
    let value: [u8; 2] = bytes
        .get(offset..offset + 2)
        .ok_or(BackupError::InvalidBackup)?
        .try_into()
        .map_err(|_| BackupError::InvalidBackup)?;
    Ok(u16::from_be_bytes(value))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, BackupError> {
    let value: [u8; 4] = bytes
        .get(offset..offset + 4)
        .ok_or(BackupError::InvalidBackup)?
        .try_into()
        .map_err(|_| BackupError::InvalidBackup)?;
    Ok(u32::from_be_bytes(value))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, BackupError> {
    let value: [u8; 8] = bytes
        .get(offset..offset + 8)
        .ok_or(BackupError::InvalidBackup)?
        .try_into()
        .map_err(|_| BackupError::InvalidBackup)?;
    Ok(u64::from_be_bytes(value))
}

fn validate_payload(payload: &PortableBackupV1) -> Result<(), BackupError> {
    if payload.manifest.format_major != FORMAT_MAJOR
        || payload.manifest.format_minor != FORMAT_MINOR
        || payload.manifest.database_schema != 1
        || payload.manifest.document_schema != 1
    {
        return Err(BackupError::InvalidBackup);
    }
    validate_export_metadata(&payload.manifest.app_version, &payload.manifest.created_at)
        .map_err(|_| BackupError::InvalidBackup)?;
    validate_profile(&payload.profile).map_err(|_| BackupError::InvalidBackup)?;
    if payload.manifest.inventory
        != inventory_for(&payload.profile).map_err(|_| BackupError::InvalidBackup)?
    {
        return Err(BackupError::InvalidBackup);
    }
    let profile_json =
        serde_json::to_vec(&payload.profile).map_err(|_| BackupError::InvalidBackup)?;
    let expected_hash = hex::encode(Sha256::digest(&profile_json));
    if payload.manifest.profile_sha256 != expected_hash {
        return Err(BackupError::InvalidBackup);
    }
    Ok(())
}

fn validate_export_metadata(app_version: &str, created_at: &str) -> Result<(), BackupError> {
    let app_version_valid = !app_version.is_empty()
        && app_version.len() <= 64
        && app_version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+' | b'_'));
    if !app_version_valid || created_at.len() > 64 || created_at.parse::<Timestamp>().is_err() {
        return Err(BackupError::InvalidContent);
    }
    Ok(())
}

fn validate_profile(profile: &PortableProfileV1) -> Result<(), BackupError> {
    if profile.published_resumes.len() > MAX_PUBLISHED_RESUMES
        || profile.settings.len() > MAX_SETTINGS
    {
        return Err(BackupError::InvalidContent);
    }
    if let Some(draft) = &profile.master_draft {
        validate_resume_revision(draft)?;
    }
    let mut previous_revision = 0_i64;
    for published in &profile.published_resumes {
        if published.published_revision < 1 || published.draft_revision < 1 {
            return Err(BackupError::InvalidContent);
        }
        published
            .document
            .validate(DocumentLimits::default())
            .map_err(|_| BackupError::InvalidContent)?;
        if published.published_revision <= previous_revision {
            return Err(BackupError::InvalidContent);
        }
        previous_revision = published.published_revision;
    }
    for (key, setting) in &profile.settings {
        if setting.revision < 1 {
            return Err(BackupError::InvalidContent);
        }
        validate_setting(key, &setting.value)?;
    }
    let serialized = serde_json::to_vec(profile).map_err(|_| BackupError::InvalidContent)?;
    if serialized.len() > MAX_PAYLOAD_BYTES {
        return Err(BackupError::InvalidContent);
    }
    Ok(())
}

fn validate_resume_revision(resume: &PortableResumeRevisionV1) -> Result<(), BackupError> {
    if resume.revision < 1 {
        return Err(BackupError::InvalidContent);
    }
    resume
        .document
        .validate(DocumentLimits::default())
        .map_err(|_| BackupError::InvalidContent)
}

fn validate_setting(key: &str, value: &Value) -> Result<(), BackupError> {
    let normalized = key.to_ascii_lowercase();
    let valid_key = !key.is_empty()
        && key.len() <= 64
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        && !["secret", "password", "token", "credential", "api_key"]
            .iter()
            .any(|forbidden| normalized.contains(forbidden));
    let serialized = serde_json::to_vec(value).map_err(|_| BackupError::InvalidContent)?;
    if !valid_key || serialized.len() > MAX_SETTING_BYTES {
        return Err(BackupError::InvalidContent);
    }
    Ok(())
}

fn inventory_for(profile: &PortableProfileV1) -> Result<BackupInventoryV1, BackupError> {
    Ok(BackupInventoryV1 {
        master_drafts: u16::from(profile.master_draft.is_some()),
        published_resumes: u16::try_from(profile.published_resumes.len())
            .map_err(|_| BackupError::InvalidContent)?,
        settings: u16::try_from(profile.settings.len()).map_err(|_| BackupError::InvalidContent)?,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ort_domain::{EntityId, ResumeDocument};
    use serde_json::Value;

    use super::{
        BackupError, BackupExportRequestV1, BackupPassphrase, PortableProfileV1,
        PortablePublishedResumeV1, PortableResumeRevisionV1, PortableSettingV1, Sha256,
        create_backup, create_backup_with_entropy, inspect_backup, restore_backup,
    };
    use sha2::Digest;

    const MARKER: &str = "SYNTHETIC-PORTABLE-PRIVATE-MARKER-71c36a";

    #[test]
    fn portable_backup_round_trips_and_exposes_no_content() {
        let passphrase = BackupPassphrase::new("correct horse battery staple".to_owned())
            .expect("valid passphrase");
        let request = sample_request();
        let expected_profile = request.profile.clone();
        let backup = create_backup(&passphrase, request).expect("create backup");
        assert!(
            !backup
                .windows(MARKER.len())
                .any(|window| window == MARKER.as_bytes())
        );
        let header = inspect_backup(&backup).expect("inspect header");
        assert_eq!(header.format_major, 1);
        assert_eq!(header.memory_kib, 65_536);
        assert_eq!(header.iterations, 3);
        assert_eq!(header.lanes, 4);

        let restored = restore_backup(&backup, &passphrase).expect("restore backup");
        assert_eq!(restored.profile, expected_profile);
        assert_eq!(restored.manifest.inventory.master_drafts, 1);
    }

    #[test]
    fn wrong_passphrase_tamper_and_truncation_share_invalid_error() {
        let passphrase = BackupPassphrase::new("correct passphrase".to_owned()).expect("valid");
        let wrong = BackupPassphrase::new("incorrect passphrase".to_owned()).expect("valid");
        let backup = create_backup(&passphrase, sample_request()).expect("create backup");
        assert_eq!(
            restore_backup(&backup, &wrong),
            Err(BackupError::InvalidBackup)
        );

        let mut tampered = backup.clone();
        let last = tampered.last_mut().expect("ciphertext byte");
        *last ^= 0x80;
        assert_eq!(
            restore_backup(&tampered, &passphrase),
            Err(BackupError::InvalidBackup)
        );
        assert_eq!(
            restore_backup(&backup[..backup.len() - 1], &passphrase),
            Err(BackupError::InvalidBackup)
        );
    }

    #[test]
    fn out_of_policy_header_is_rejected_before_decryption() {
        let passphrase = BackupPassphrase::new("correct passphrase".to_owned()).expect("valid");
        let mut backup = create_backup(&passphrase, sample_request()).expect("create backup");
        backup[12..16].copy_from_slice(&1_u32.to_be_bytes());
        assert_eq!(inspect_backup(&backup), Err(BackupError::InvalidBackup));
        assert_eq!(
            restore_backup(&backup, &passphrase),
            Err(BackupError::InvalidBackup)
        );

        for length in 0..super::HEADER_LEN {
            assert_eq!(
                inspect_backup(&backup[..length]),
                Err(BackupError::InvalidBackup)
            );
        }
        let mut oversized = backup.clone();
        oversized[28..36].copy_from_slice(&u64::MAX.to_be_bytes());
        assert_eq!(inspect_backup(&oversized), Err(BackupError::InvalidBackup));
        let mut reserved = backup;
        reserved[9] = 1;
        assert_eq!(inspect_backup(&reserved), Err(BackupError::InvalidBackup));
    }

    #[test]
    fn passphrases_are_redacted_and_secret_settings_are_excluded() {
        let passphrase = BackupPassphrase::new("do not display this value".to_owned())
            .expect("valid passphrase");
        assert_eq!(format!("{passphrase:?}"), "BackupPassphrase([REDACTED])");
        let mut request = sample_request();
        request.profile.settings.insert(
            "provider.api_token".to_owned(),
            PortableSettingV1 {
                revision: 1,
                value: Value::String("synthetic-token".to_owned()),
            },
        );
        assert_eq!(
            create_backup(&passphrase, request),
            Err(BackupError::InvalidContent)
        );
    }

    #[test]
    fn fixed_entropy_produces_a_stable_format_vector() {
        let passphrase = BackupPassphrase::new("vector passphrase".to_owned()).expect("valid");
        let backup =
            create_backup_with_entropy(&passphrase, sample_request(), [0x11; 16], [0x22; 24])
                .expect("create vector");
        let digest = hex::encode(Sha256::digest(&backup));
        assert_eq!(
            digest,
            "bad075c8e1369c6aa67f4b41d422826e84cde14070e43724caa063cae26e90aa"
        );
        let restored = restore_backup(&backup, &passphrase).expect("restore vector");
        assert_eq!(
            restored.profile.master_draft.expect("draft").document.title,
            MARKER
        );
    }

    fn sample_request() -> BackupExportRequestV1 {
        let mut settings = BTreeMap::new();
        settings.insert(
            "appearance.theme".to_owned(),
            PortableSettingV1 {
                revision: 2,
                value: Value::String("system".to_owned()),
            },
        );
        let mut draft = ResumeDocument::empty(MARKER);
        draft.document_id =
            EntityId::parse("018f8b1b-50ad-7b4a-8f7d-38fd63e44086").expect("fixed UUIDv7");
        let mut published = ResumeDocument::empty("Published synthetic resume");
        published.document_id =
            EntityId::parse("018f8b1b-50ad-7b4a-8f7d-38fd63e44087").expect("fixed UUIDv7");
        BackupExportRequestV1 {
            app_version: "0.0.0-dev".to_owned(),
            created_at: "2026-09-01T12:00:00Z".to_owned(),
            profile: PortableProfileV1 {
                master_draft: Some(PortableResumeRevisionV1 {
                    revision: 3,
                    document: draft,
                }),
                published_resumes: vec![PortablePublishedResumeV1 {
                    published_revision: 1,
                    draft_revision: 2,
                    document: published,
                }],
                settings,
            },
        }
    }
}
