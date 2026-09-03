//! Encrypted, local-first persistence.
//!
//! The database key lives only in the operating-system credential vault. A
//! non-secret manifest locates that key; the SQLite database is always opened
//! through `SQLCipher` before any schema access is attempted.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use jiff::Timestamp;
use ort_backup::{
    BackupError, BackupExportRequestV1, BackupPassphrase, PortableProfileV1,
    PortablePublishedResumeV1, PortableRenderManifestV1, PortableResumeRevisionV1,
    PortableSettingV1, create_backup, restore_backup,
};
use ort_domain::{
    DocumentLimits, ExportSource, MAX_PDF_BYTES, MAX_PDF_PAGES, PdfRenderReceipt, ResumeDocument,
};
use ort_vault::{DatabaseKey, DatabaseKeyVault, VaultError, VaultReference};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, backup, params,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zeroize::Zeroize;

const DATABASE_FILENAME: &str = "profile.db";
const MANIFEST_FILENAME: &str = "profile.json";
const PREVIOUS_MANIFEST_FILENAME: &str = "profile.json.previous";
const MANIFEST_UPDATE_FILENAME: &str = ".profile-update.tmp";
const DATABASE_FORMAT_VERSION: u16 = 1;
const SCHEMA_VERSION: i64 = 2;
const MAX_RENDER_MANIFESTS: i64 = 100;
const MAX_JAVASCRIPT_DATE_MS: u64 = 8_640_000_000_000_000;
const MAX_MANIFEST_BYTES: u64 = 16 * 1_024;
const MAX_SETTING_BYTES: usize = 64 * 1_024;
const MIGRATION_V1_SQL: &str = "CREATE TABLE schema_migrations (
         version INTEGER PRIMARY KEY,
         checksum_sha256 TEXT NOT NULL,
         minimum_app_version TEXT NOT NULL,
         estimated_disk_bytes INTEGER NOT NULL CHECK (estimated_disk_bytes >= 0),
         requires_safety_copy INTEGER NOT NULL CHECK (requires_safety_copy IN (0, 1)),
         applied_at TEXT NOT NULL
     ) STRICT;
     CREATE TABLE app_metadata (
         metadata_key TEXT PRIMARY KEY,
         metadata_value TEXT NOT NULL
     ) STRICT;
     CREATE TABLE profiles (
         profile_id TEXT PRIMARY KEY,
         revision INTEGER NOT NULL CHECK (revision >= 1),
         created_at TEXT NOT NULL,
         updated_at TEXT NOT NULL
     ) STRICT;
     CREATE TABLE resume_drafts (
         profile_id TEXT PRIMARY KEY REFERENCES profiles(profile_id) ON DELETE CASCADE,
         revision INTEGER NOT NULL CHECK (revision >= 1),
         schema_version INTEGER NOT NULL CHECK (schema_version >= 1),
         document_json BLOB NOT NULL,
         created_at TEXT NOT NULL,
         updated_at TEXT NOT NULL
     ) STRICT;
     CREATE TABLE published_resumes (
         profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
         published_revision INTEGER NOT NULL CHECK (published_revision >= 1),
         draft_revision INTEGER NOT NULL CHECK (draft_revision >= 1),
         schema_version INTEGER NOT NULL CHECK (schema_version >= 1),
         document_json BLOB NOT NULL,
         published_at TEXT NOT NULL,
         PRIMARY KEY (profile_id, published_revision)
     ) STRICT;
     CREATE TABLE settings (
         profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
         setting_key TEXT NOT NULL,
         revision INTEGER NOT NULL CHECK (revision >= 1),
         value_json BLOB NOT NULL,
         updated_at TEXT NOT NULL,
         PRIMARY KEY (profile_id, setting_key)
     ) STRICT;
     CREATE TABLE diagnostic_events (
         event_id TEXT PRIMARY KEY,
         profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
         event_code TEXT NOT NULL,
         severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'error')),
         safe_context_json BLOB NOT NULL,
         created_at TEXT NOT NULL
     ) STRICT;";
const MIGRATION_V2_SQL: &str = "CREATE TABLE render_manifests (
         manifest_id TEXT PRIMARY KEY,
         profile_id TEXT NOT NULL REFERENCES profiles(profile_id) ON DELETE CASCADE,
         source TEXT NOT NULL CHECK (source IN ('saved_draft', 'published_snapshot')),
         source_revision INTEGER NOT NULL CHECK (source_revision >= 1),
         generated_at_unix_ms INTEGER NOT NULL CHECK (generated_at_unix_ms >= 1),
         last_generated_at_unix_ms INTEGER NOT NULL CHECK (last_generated_at_unix_ms >= generated_at_unix_ms),
         render_count INTEGER NOT NULL CHECK (render_count >= 1),
         document_sha256 TEXT NOT NULL,
         document_schema_version INTEGER NOT NULL CHECK (document_schema_version >= 1),
         pdf_sha256 TEXT NOT NULL,
         renderer_version TEXT NOT NULL,
         template_id TEXT NOT NULL,
         template_sha256 TEXT NOT NULL,
         font_bundle_id TEXT NOT NULL,
         font_bundle_sha256 TEXT NOT NULL,
         page_count INTEGER NOT NULL CHECK (page_count >= 1),
         byte_count INTEGER NOT NULL CHECK (byte_count >= 1),
         UNIQUE (profile_id, source, source_revision, pdf_sha256)
     ) STRICT;
     CREATE INDEX render_manifests_recent
         ON render_manifests (profile_id, last_generated_at_unix_ms DESC, manifest_id DESC);";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StorageError {
    #[error("the storage location is unsafe")]
    UnsafeLocation,
    #[error("storage initialization was interrupted")]
    IncompleteInitialization,
    #[error("the profile manifest is invalid")]
    InvalidManifest,
    #[error("the database key is unavailable")]
    VaultKeyUnavailable,
    #[error("the database key does not unlock this profile")]
    DatabaseKeyMismatch,
    #[error("the encrypted database provider is unavailable")]
    CipherUnavailable,
    #[error("the encrypted database failed its integrity check")]
    IntegrityFailure,
    #[error("the database was created by a newer application version")]
    NewerSchema,
    #[error("the requested record was not found")]
    NotFound,
    #[error("the record changed after it was loaded")]
    RevisionConflict,
    #[error("the supplied data is invalid")]
    InvalidData,
    #[error("local storage is unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProfileManifest {
    pub manifest_version: u16,
    pub database_format_version: u16,
    pub schema_version: i64,
    pub install_id: Uuid,
    pub profile_id: Uuid,
    pub channel: String,
    pub created_at: String,
}

impl ProfileManifest {
    /// Returns the non-secret operating-system vault address for this profile.
    ///
    /// # Errors
    /// Returns an error when the persisted identifiers or channel are malformed.
    pub fn vault_reference(&self) -> Result<VaultReference, StorageError> {
        VaultReference::new(
            &self.channel,
            &self.install_id.to_string(),
            &self.profile_id.to_string(),
        )
        .map_err(|_| StorageError::InvalidManifest)
    }

    fn validate(&self, expected_channel: &str) -> Result<(), StorageError> {
        if self.manifest_version != 1
            || self.database_format_version != DATABASE_FORMAT_VERSION
            || self.schema_version > SCHEMA_VERSION
            || self.channel != expected_channel
            || self.install_id.get_version_num() != 7
            || self.profile_id.get_version_num() != 7
        {
            return Err(StorageError::InvalidManifest);
        }
        validate_channel(expected_channel)?;
        self.vault_reference()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionedResume {
    pub revision: i64,
    pub document: ResumeDocument,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VersionedSetting {
    pub revision: i64,
    pub value: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

impl DiagnosticSeverity {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    fn parse(value: &str) -> Result<Self, StorageError> {
        match value {
            "info" => Ok(Self::Info),
            "warning" => Ok(Self::Warning),
            "error" => Ok(Self::Error),
            _ => Err(StorageError::InvalidData),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticEvent {
    pub event_id: Uuid,
    pub event_code: String,
    pub severity: DiagnosticSeverity,
    pub safe_context: Map<String, Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRenderManifest {
    pub manifest_id: Uuid,
    pub source: ExportSource,
    pub source_revision: i64,
    pub generated_at_unix_ms: u64,
    pub last_generated_at_unix_ms: u64,
    pub render_count: u32,
    pub receipt: PdfRenderReceipt,
}

pub struct EncryptedStore {
    connection: Mutex<Connection>,
    manifest: ProfileManifest,
    database_path: PathBuf,
}

impl EncryptedStore {
    /// Opens an existing encrypted profile or creates one atomically enough to
    /// ensure that a missing vault key is never silently replaced.
    ///
    /// # Errors
    /// Returns a safe storage error without embedding paths, SQL, or secrets.
    pub fn open_or_initialize(
        root: &Path,
        channel: &str,
        vault: &dyn DatabaseKeyVault,
    ) -> Result<Self, StorageError> {
        validate_channel(channel)?;
        reject_symlink(root)?;

        let manifest_path = root.join(MANIFEST_FILENAME);
        let database_path = root.join(DATABASE_FILENAME);
        recover_manifest_update(root, &manifest_path, &database_path)?;

        if manifest_path.exists() {
            reject_symlink(&manifest_path)?;
            reject_symlink(&database_path)?;
            if !database_path.is_file() {
                return Err(StorageError::IncompleteInitialization);
            }
            let mut manifest = read_manifest(&manifest_path, channel)?;
            let reference = manifest.vault_reference()?;
            let key = vault
                .load(&reference)
                .map_err(|error| map_vault_load_error(&error))?;
            let connection = open_encrypted_connection(&database_path, &key, false)?;
            migrate_schema(&connection)?;
            verify_schema(&connection)?;
            if manifest.schema_version == SCHEMA_VERSION {
                remove_previous_manifest(root)?;
            } else {
                manifest.schema_version = SCHEMA_VERSION;
                replace_manifest_atomically(root, &manifest_path, &manifest)?;
            }
            set_private_database_permissions(&database_path)?;
            return Ok(Self {
                connection: Mutex::new(connection),
                manifest,
                database_path,
            });
        }

        if database_path.exists() {
            return Err(StorageError::IncompleteInitialization);
        }

        create_private_directory(root)?;
        let manifest = ProfileManifest {
            manifest_version: 1,
            database_format_version: DATABASE_FORMAT_VERSION,
            schema_version: SCHEMA_VERSION,
            install_id: Uuid::now_v7(),
            profile_id: Uuid::now_v7(),
            channel: channel.to_owned(),
            created_at: now_string(),
        };
        let reference = manifest.vault_reference()?;
        let key = DatabaseKey::generate().map_err(|error| map_vault_creation_error(&error))?;
        vault
            .store_new(&reference, &key)
            .map_err(|error| map_vault_creation_error(&error))?;

        let initialized = (|| {
            let connection = open_encrypted_connection(&database_path, &key, true)?;
            initialize_schema(&connection, &manifest)?;
            migrate_schema(&connection)?;
            verify_schema(&connection)?;
            set_private_database_permissions(&database_path)?;
            write_manifest_atomically(root, &manifest_path, &manifest)?;
            Ok(connection)
        })();

        match initialized {
            Ok(connection) => Ok(Self {
                connection: Mutex::new(connection),
                manifest,
                database_path,
            }),
            Err(error) => {
                let _ = remove_exact_database_files(&database_path);
                let _ = vault.delete(&reference);
                Err(error)
            }
        }
    }

    #[must_use]
    pub const fn manifest(&self) -> &ProfileManifest {
        &self.manifest
    }

    #[must_use]
    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    /// Loads the current editable resume.
    ///
    /// # Errors
    /// Returns an error for unavailable storage or invalid persisted data.
    pub fn load_draft(&self) -> Result<Option<VersionedResume>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        connection
            .query_row(
                "SELECT revision, document_json FROM resume_drafts WHERE profile_id = ?1",
                [self.manifest.profile_id.to_string()],
                |row| {
                    let revision: i64 = row.get(0)?;
                    let json: Vec<u8> = row.get(1)?;
                    Ok((revision, json))
                },
            )
            .optional()
            .map_err(|_| StorageError::Unavailable)?
            .map(|(revision, json)| parse_versioned_resume(revision, &json))
            .transpose()
    }

    /// Creates the first editable resume at revision 1.
    ///
    /// # Errors
    /// Returns a validation or revision conflict error when it already exists.
    pub fn create_draft(&self, document: &ResumeDocument) -> Result<VersionedResume, StorageError> {
        let json = serialize_document(document)?;
        let now = now_string();
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let result = connection.execute(
            "INSERT INTO resume_drafts \
             (profile_id, revision, schema_version, document_json, created_at, updated_at) \
             VALUES (?1, 1, ?2, ?3, ?4, ?4)",
            params![
                self.manifest.profile_id.to_string(),
                i64::from(document.schema_version),
                json,
                now
            ],
        );
        match result {
            Ok(1) => {
                set_private_database_permissions(&self.database_path)?;
                Ok(VersionedResume {
                    revision: 1,
                    document: document.clone(),
                })
            }
            Err(error) if is_constraint_error(&error) => Err(StorageError::RevisionConflict),
            Ok(_) | Err(_) => Err(StorageError::Unavailable),
        }
    }

    /// Saves a draft only when the expected revision is still current.
    ///
    /// # Errors
    /// Returns `RevisionConflict` if another write won the race.
    pub fn save_draft(
        &self,
        expected_revision: i64,
        document: &ResumeDocument,
    ) -> Result<VersionedResume, StorageError> {
        if expected_revision < 1 {
            return Err(StorageError::InvalidData);
        }
        let json = serialize_document(document)?;
        let next_revision = expected_revision
            .checked_add(1)
            .ok_or(StorageError::InvalidData)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let changed = connection
            .execute(
                "UPDATE resume_drafts SET revision = ?1, schema_version = ?2, \
                 document_json = ?3, updated_at = ?4 \
                 WHERE profile_id = ?5 AND revision = ?6",
                params![
                    next_revision,
                    i64::from(document.schema_version),
                    json,
                    now_string(),
                    self.manifest.profile_id.to_string(),
                    expected_revision
                ],
            )
            .map_err(|_| StorageError::Unavailable)?;
        if changed != 1 {
            return Err(StorageError::RevisionConflict);
        }
        set_private_database_permissions(&self.database_path)?;
        Ok(VersionedResume {
            revision: next_revision,
            document: document.clone(),
        })
    }

    /// Publishes an immutable snapshot of one exact draft revision.
    /// Repeating a publication of the same revision returns its existing snapshot.
    ///
    /// # Errors
    /// Returns `RevisionConflict` if the draft changed before publication.
    pub fn publish_draft(
        &self,
        expected_draft_revision: i64,
    ) -> Result<VersionedResume, StorageError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        let row = transaction
            .query_row(
                "SELECT revision, schema_version, document_json FROM resume_drafts \
                 WHERE profile_id = ?1",
                [self.manifest.profile_id.to_string()],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| StorageError::Unavailable)?
            .ok_or(StorageError::NotFound)?;
        if row.0 != expected_draft_revision {
            return Err(StorageError::RevisionConflict);
        }
        // Validate before any INSERT or COMMIT, including data loaded from disk.
        let document = parse_document(&row.2)?;
        if i64::from(document.schema_version) != row.1 {
            return Err(StorageError::InvalidData);
        }
        let existing = transaction
            .query_row(
                "SELECT published_revision, document_json FROM published_resumes \
                 WHERE profile_id = ?1 AND draft_revision = ?2 ORDER BY published_revision DESC LIMIT 1",
                params![self.manifest.profile_id.to_string(), expected_draft_revision],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional()
            .map_err(|_| StorageError::Unavailable)?;
        if let Some((revision, json)) = existing {
            let published = parse_versioned_resume(revision, &json)?;
            if published.document != document {
                return Err(StorageError::IntegrityFailure);
            }
            return Ok(published);
        }

        let published_revision: i64 = transaction
            .query_row(
                "SELECT COALESCE(MAX(published_revision), 0) + 1 FROM published_resumes \
                 WHERE profile_id = ?1",
                [self.manifest.profile_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|_| StorageError::Unavailable)?;
        transaction
            .execute(
                "INSERT INTO published_resumes \
                 (profile_id, published_revision, draft_revision, schema_version, document_json, published_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    self.manifest.profile_id.to_string(),
                    published_revision,
                    row.0,
                    row.1,
                    row.2,
                    now_string()
                ],
            )
            .map_err(|_| StorageError::Unavailable)?;
        transaction
            .commit()
            .map_err(|_| StorageError::Unavailable)?;
        set_private_database_permissions(&self.database_path)?;

        Ok(VersionedResume {
            revision: published_revision,
            document,
        })
    }

    /// Loads the newest published snapshot without modifying the draft.
    ///
    /// # Errors
    /// Returns an error for unavailable storage or invalid persisted data.
    pub fn load_latest_published(&self) -> Result<Option<VersionedResume>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        connection
            .query_row(
                "SELECT published_revision, document_json FROM published_resumes \
                 WHERE profile_id = ?1 ORDER BY published_revision DESC LIMIT 1",
                [self.manifest.profile_id.to_string()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional()
            .map_err(|_| StorageError::Unavailable)?
            .map(|(revision, json)| parse_versioned_resume(revision, &json))
            .transpose()
    }

    /// Creates or updates a bounded JSON setting with optimistic revision checks.
    ///
    /// # Errors
    /// Returns a validation or revision conflict error when appropriate.
    pub fn save_setting(
        &self,
        key: &str,
        expected_revision: Option<i64>,
        value: &Value,
    ) -> Result<VersionedSetting, StorageError> {
        validate_setting_key(key)?;
        let json = serde_json::to_vec(value).map_err(|_| StorageError::InvalidData)?;
        if json.len() > MAX_SETTING_BYTES {
            return Err(StorageError::InvalidData);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;

        let revision = match expected_revision {
            None => {
                let result = connection.execute(
                    "INSERT INTO settings (profile_id, setting_key, revision, value_json, updated_at) \
                     VALUES (?1, ?2, 1, ?3, ?4)",
                    params![
                        self.manifest.profile_id.to_string(),
                        key,
                        json,
                        now_string()
                    ],
                );
                match result {
                    Ok(1) => 1,
                    Err(error) if is_constraint_error(&error) => {
                        return Err(StorageError::RevisionConflict);
                    }
                    Ok(_) | Err(_) => return Err(StorageError::Unavailable),
                }
            }
            Some(current) if current >= 1 => {
                let next = current.checked_add(1).ok_or(StorageError::InvalidData)?;
                let changed = connection
                    .execute(
                        "UPDATE settings SET revision = ?1, value_json = ?2, updated_at = ?3 \
                         WHERE profile_id = ?4 AND setting_key = ?5 AND revision = ?6",
                        params![
                            next,
                            json,
                            now_string(),
                            self.manifest.profile_id.to_string(),
                            key,
                            current
                        ],
                    )
                    .map_err(|_| StorageError::Unavailable)?;
                if changed != 1 {
                    return Err(StorageError::RevisionConflict);
                }
                next
            }
            Some(_) => return Err(StorageError::InvalidData),
        };
        Ok(VersionedSetting {
            revision,
            value: value.clone(),
        })
    }

    /// Loads a JSON setting.
    ///
    /// # Errors
    /// Returns an error for malformed keys or unavailable storage.
    pub fn load_setting(&self, key: &str) -> Result<Option<VersionedSetting>, StorageError> {
        validate_setting_key(key)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let row = connection
            .query_row(
                "SELECT revision, value_json FROM settings WHERE profile_id = ?1 AND setting_key = ?2",
                params![self.manifest.profile_id.to_string(), key],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional()
            .map_err(|_| StorageError::Unavailable)?;
        row.map(|(revision, json)| {
            serde_json::from_slice(&json)
                .map(|value| VersionedSetting { revision, value })
                .map_err(|_| StorageError::InvalidData)
        })
        .transpose()
    }

    /// Records bounded operational metadata that is safe for diagnostics.
    ///
    /// Free-form prose, nested values, common secret-bearing key names, and
    /// non-token strings are rejected so this API cannot become a resume or
    /// credential logging path.
    ///
    /// # Errors
    /// Returns `InvalidData` if the event is not safe and bounded.
    pub fn record_diagnostic(
        &self,
        event_code: &str,
        severity: DiagnosticSeverity,
        safe_context: &Map<String, Value>,
    ) -> Result<DiagnosticEvent, StorageError> {
        validate_diagnostic_code(event_code)?;
        validate_safe_context(safe_context)?;
        let event = DiagnosticEvent {
            event_id: Uuid::now_v7(),
            event_code: event_code.to_owned(),
            severity,
            safe_context: safe_context.clone(),
            created_at: now_string(),
        };
        let context_json =
            serde_json::to_vec(&event.safe_context).map_err(|_| StorageError::InvalidData)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        connection
            .execute(
                "INSERT INTO diagnostic_events \
                 (event_id, profile_id, event_code, severity, safe_context_json, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    event.event_id.to_string(),
                    self.manifest.profile_id.to_string(),
                    event.event_code,
                    event.severity.as_str(),
                    context_json,
                    event.created_at
                ],
            )
            .map_err(|_| StorageError::Unavailable)?;
        Ok(event)
    }

    /// Loads at most 100 recent safe diagnostic events.
    ///
    /// # Errors
    /// Returns an error if the limit or a persisted event is invalid.
    pub fn load_recent_diagnostics(
        &self,
        limit: u16,
    ) -> Result<Vec<DiagnosticEvent>, StorageError> {
        if !(1..=100).contains(&limit) {
            return Err(StorageError::InvalidData);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let mut statement = connection
            .prepare(
                "SELECT event_id, event_code, severity, safe_context_json, created_at \
                 FROM diagnostic_events WHERE profile_id = ?1 \
                 ORDER BY created_at DESC, event_id DESC LIMIT ?2",
            )
            .map_err(|_| StorageError::Unavailable)?;
        let rows = statement
            .query_map(
                params![self.manifest.profile_id.to_string(), i64::from(limit)],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .map_err(|_| StorageError::Unavailable)?;

        rows.map(|row| {
            let (event_id, event_code, severity, context_json, created_at) =
                row.map_err(|_| StorageError::Unavailable)?;
            let event_id = Uuid::parse_str(&event_id).map_err(|_| StorageError::InvalidData)?;
            if event_id.get_version_num() != 7 {
                return Err(StorageError::InvalidData);
            }
            validate_diagnostic_code(&event_code)?;
            let safe_context: Map<String, Value> =
                serde_json::from_slice(&context_json).map_err(|_| StorageError::InvalidData)?;
            validate_safe_context(&safe_context)?;
            Ok(DiagnosticEvent {
                event_id,
                event_code,
                severity: DiagnosticSeverity::parse(&severity)?,
                safe_context,
                created_at,
            })
        })
        .collect()
    }

    /// Stores a content-free receipt for one exact locally rendered revision.
    /// Re-rendering identical bytes updates the existing manifest instead of
    /// growing an unbounded event log. Only the newest 100 identities remain.
    ///
    /// # Errors
    /// Returns an error for invalid receipt metadata or unavailable storage.
    pub fn record_render_manifest(
        &self,
        source: ExportSource,
        source_revision: i64,
        generated_at_unix_ms: u64,
        receipt: &PdfRenderReceipt,
    ) -> Result<StoredRenderManifest, StorageError> {
        validate_render_manifest(source_revision, generated_at_unix_ms, receipt)?;
        let generated_at =
            i64::try_from(generated_at_unix_ms).map_err(|_| StorageError::InvalidData)?;
        let page_count =
            i64::try_from(receipt.page_count).map_err(|_| StorageError::InvalidData)?;
        let byte_count =
            i64::try_from(receipt.byte_count).map_err(|_| StorageError::InvalidData)?;
        let source_code = export_source_code(source);
        let manifest_id = Uuid::now_v7();
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        transaction
            .execute(
                "INSERT INTO render_manifests \
                 (manifest_id, profile_id, source, source_revision, generated_at_unix_ms, \
                  last_generated_at_unix_ms, render_count, document_sha256, \
                  document_schema_version, pdf_sha256, renderer_version, template_id, \
                  template_sha256, font_bundle_id, font_bundle_sha256, page_count, byte_count) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5, 1, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15) \
                 ON CONFLICT(profile_id, source, source_revision, pdf_sha256) DO UPDATE SET \
                  last_generated_at_unix_ms = MAX(render_manifests.last_generated_at_unix_ms, excluded.last_generated_at_unix_ms), \
                  render_count = MIN(render_manifests.render_count + 1, 2147483647)",
                params![
                    manifest_id.to_string(),
                    self.manifest.profile_id.to_string(),
                    source_code,
                    source_revision,
                    generated_at,
                    receipt.document_sha256,
                    i64::from(receipt.document_schema_version),
                    receipt.pdf_sha256,
                    receipt.renderer_version,
                    receipt.template_id,
                    receipt.template_sha256,
                    receipt.font_bundle_id,
                    receipt.font_bundle_sha256,
                    page_count,
                    byte_count,
                ],
            )
            .map_err(|_| StorageError::Unavailable)?;
        transaction
            .execute(
                "DELETE FROM render_manifests WHERE manifest_id IN (\
                   SELECT manifest_id FROM render_manifests WHERE profile_id = ?1 \
                   ORDER BY last_generated_at_unix_ms DESC, manifest_id DESC \
                   LIMIT -1 OFFSET ?2\
                 )",
                params![self.manifest.profile_id.to_string(), MAX_RENDER_MANIFESTS],
            )
            .map_err(|_| StorageError::Unavailable)?;
        transaction
            .commit()
            .map_err(|_| StorageError::Unavailable)?;
        drop(connection);
        self.load_render_manifest_identity(source, source_revision, &receipt.pdf_sha256)?
            .ok_or(StorageError::Unavailable)
    }

    /// Loads the newest bounded render identities without PDF bytes or content.
    ///
    /// # Errors
    /// Returns an error for an invalid limit or malformed persisted metadata.
    pub fn load_recent_render_manifests(
        &self,
        limit: u16,
    ) -> Result<Vec<StoredRenderManifest>, StorageError> {
        if !(1..=50).contains(&limit) {
            return Err(StorageError::InvalidData);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let mut statement = connection
            .prepare(
                "SELECT manifest_id, source, source_revision, generated_at_unix_ms, \
                 last_generated_at_unix_ms, render_count, document_sha256, \
                 document_schema_version, pdf_sha256, renderer_version, template_id, \
                 template_sha256, font_bundle_id, font_bundle_sha256, page_count, byte_count \
                 FROM render_manifests WHERE profile_id = ?1 \
                 ORDER BY last_generated_at_unix_ms DESC, manifest_id DESC LIMIT ?2",
            )
            .map_err(|_| StorageError::Unavailable)?;
        let rows = statement
            .query_map(
                params![self.manifest.profile_id.to_string(), i64::from(limit)],
                render_manifest_row,
            )
            .map_err(|_| StorageError::Unavailable)?;
        rows.map(|row| {
            let row = row.map_err(|_| StorageError::InvalidData)?;
            parse_render_manifest(row)
        })
        .collect()
    }

    fn load_render_manifest_identity(
        &self,
        source: ExportSource,
        source_revision: i64,
        pdf_sha256: &str,
    ) -> Result<Option<StoredRenderManifest>, StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        connection
            .query_row(
                "SELECT manifest_id, source, source_revision, generated_at_unix_ms, \
                 last_generated_at_unix_ms, render_count, document_sha256, \
                 document_schema_version, pdf_sha256, renderer_version, template_id, \
                 template_sha256, font_bundle_id, font_bundle_sha256, page_count, byte_count \
                 FROM render_manifests WHERE profile_id = ?1 AND source = ?2 \
                 AND source_revision = ?3 AND pdf_sha256 = ?4",
                params![
                    self.manifest.profile_id.to_string(),
                    export_source_code(source),
                    source_revision,
                    pdf_sha256,
                ],
                render_manifest_row,
            )
            .optional()
            .map_err(|_| StorageError::Unavailable)?
            .map(parse_render_manifest)
            .transpose()
    }

    /// Creates a password-protected portable backup containing canonical
    /// records but no database key, vault reference, or diagnostics.
    ///
    /// # Errors
    /// Returns an error when records are invalid or encryption is unavailable.
    pub fn create_portable_backup(
        &self,
        passphrase: &BackupPassphrase,
        app_version: &str,
    ) -> Result<Vec<u8>, StorageError> {
        let profile = self.read_portable_profile()?;
        create_backup(
            passphrase,
            BackupExportRequestV1 {
                app_version: app_version.to_owned(),
                created_at: now_string(),
                profile,
            },
        )
        .map_err(|error| map_backup_write_error(&error))
    }

    /// Restores a portable backup into an otherwise empty, newly encrypted
    /// profile. The destination retains its independently generated local
    /// database key and vault identity.
    ///
    /// # Errors
    /// Returns `RevisionConflict` if the destination already contains portable
    /// records, or `InvalidData` for every untrusted backup failure.
    pub fn restore_portable_backup(
        &self,
        bytes: &[u8],
        passphrase: &BackupPassphrase,
    ) -> Result<(), StorageError> {
        let backup =
            restore_backup(bytes, passphrase).map_err(|error| map_backup_read_error(&error))?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StorageError::Unavailable)?;
        let existing: i64 = transaction
            .query_row(
                "SELECT \
                 (SELECT COUNT(*) FROM resume_drafts WHERE profile_id = ?1) + \
                 (SELECT COUNT(*) FROM published_resumes WHERE profile_id = ?1) + \
                 (SELECT COUNT(*) FROM settings WHERE profile_id = ?1) + \
                 (SELECT COUNT(*) FROM render_manifests WHERE profile_id = ?1)",
                [self.manifest.profile_id.to_string()],
                |row| row.get(0),
            )
            .map_err(|_| StorageError::Unavailable)?;
        if existing != 0 {
            return Err(StorageError::RevisionConflict);
        }

        let now = now_string();
        if let Some(draft) = &backup.profile.master_draft {
            let document_json = serialize_document(&draft.document)?;
            transaction
                .execute(
                    "INSERT INTO resume_drafts \
                     (profile_id, revision, schema_version, document_json, created_at, updated_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                    params![
                        self.manifest.profile_id.to_string(),
                        draft.revision,
                        i64::from(draft.document.schema_version),
                        document_json,
                        now
                    ],
                )
                .map_err(|_| StorageError::Unavailable)?;
        }
        for published in &backup.profile.published_resumes {
            let document_json = serialize_document(&published.document)?;
            transaction
                .execute(
                    "INSERT INTO published_resumes \
                     (profile_id, published_revision, draft_revision, schema_version, document_json, published_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        self.manifest.profile_id.to_string(),
                        published.published_revision,
                        published.draft_revision,
                        i64::from(published.document.schema_version),
                        document_json,
                        now
                    ],
                )
                .map_err(|_| StorageError::Unavailable)?;
        }
        for (key, setting) in &backup.profile.settings {
            validate_setting_key(key)?;
            let value_json =
                serde_json::to_vec(&setting.value).map_err(|_| StorageError::InvalidData)?;
            if value_json.len() > MAX_SETTING_BYTES {
                return Err(StorageError::InvalidData);
            }
            transaction
                .execute(
                    "INSERT INTO settings \
                     (profile_id, setting_key, revision, value_json, updated_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        self.manifest.profile_id.to_string(),
                        key,
                        setting.revision,
                        value_json,
                        now
                    ],
                )
                .map_err(|_| StorageError::Unavailable)?;
        }
        restore_render_manifests(
            &transaction,
            &self.manifest.profile_id.to_string(),
            &backup.profile.render_manifests,
        )?;
        transaction
            .commit()
            .map_err(|_| StorageError::Unavailable)?;
        drop(connection);
        self.verify_integrity()
    }

    fn read_portable_profile(&self) -> Result<PortableProfileV1, StorageError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|_| StorageError::Unavailable)?;
        let master_draft = transaction
            .query_row(
                "SELECT revision, document_json FROM resume_drafts WHERE profile_id = ?1",
                [self.manifest.profile_id.to_string()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional()
            .map_err(|_| StorageError::Unavailable)?
            .map(|(revision, json)| {
                Ok(PortableResumeRevisionV1 {
                    revision,
                    document: parse_document(&json)?,
                })
            })
            .transpose()?;

        let published_resumes = {
            let mut statement = transaction
                .prepare(
                    "SELECT published_revision, draft_revision, document_json \
                     FROM published_resumes WHERE profile_id = ?1 \
                     ORDER BY published_revision",
                )
                .map_err(|_| StorageError::Unavailable)?;
            statement
                .query_map([self.manifest.profile_id.to_string()], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                })
                .map_err(|_| StorageError::Unavailable)?
                .map(|row| {
                    let (published_revision, draft_revision, json) =
                        row.map_err(|_| StorageError::Unavailable)?;
                    Ok(PortablePublishedResumeV1 {
                        published_revision,
                        draft_revision,
                        document: parse_document(&json)?,
                    })
                })
                .collect::<Result<Vec<_>, StorageError>>()?
        };

        let settings = {
            let mut statement = transaction
                .prepare(
                    "SELECT setting_key, revision, value_json FROM settings \
                     WHERE profile_id = ?1 ORDER BY setting_key",
                )
                .map_err(|_| StorageError::Unavailable)?;
            statement
                .query_map([self.manifest.profile_id.to_string()], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                    ))
                })
                .map_err(|_| StorageError::Unavailable)?
                .map(|row| {
                    let (key, revision, json) = row.map_err(|_| StorageError::Unavailable)?;
                    validate_setting_key(&key)?;
                    let value =
                        serde_json::from_slice(&json).map_err(|_| StorageError::InvalidData)?;
                    Ok((key, PortableSettingV1 { revision, value }))
                })
                .collect::<Result<BTreeMap<_, _>, StorageError>>()?
        };
        let render_manifests =
            read_portable_render_manifests(&transaction, &self.manifest.profile_id.to_string())?;
        transaction
            .commit()
            .map_err(|_| StorageError::Unavailable)?;
        Ok(PortableProfileV1 {
            master_draft,
            published_resumes,
            settings,
            render_manifests,
        })
    }

    /// Runs both `SQLCipher` authentication and SQLite structural checks.
    ///
    /// # Errors
    /// Returns `IntegrityFailure` if any check reports damage.
    pub fn verify_integrity(&self) -> Result<(), StorageError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        verify_integrity(&connection)
    }

    /// Creates a same-device encrypted checkpoint in a new directory.
    ///
    /// The checkpoint retains the profile's existing OS-vault reference. A
    /// later password-protected export format is intentionally a separate
    /// boundary because it requires a user-managed recovery secret.
    ///
    /// # Errors
    /// Returns an error if the destination exists, is unsafe, or cannot be
    /// verified as an encrypted copy before its manifest is committed.
    pub fn create_encrypted_checkpoint(
        &self,
        destination_root: &Path,
        vault: &dyn DatabaseKeyVault,
    ) -> Result<(), StorageError> {
        reject_symlink(destination_root)?;
        if destination_root.exists() {
            return Err(StorageError::UnsafeLocation);
        }

        create_private_directory(destination_root)?;
        let destination_database = destination_root.join(DATABASE_FILENAME);
        let destination_manifest = destination_root.join(MANIFEST_FILENAME);
        let reference = self.manifest.vault_reference()?;
        let key = vault
            .load(&reference)
            .map_err(|error| map_vault_load_error(&error))?;

        let result = (|| {
            let source = self
                .connection
                .lock()
                .map_err(|_| StorageError::Unavailable)?;
            let mut destination = open_encrypted_connection(&destination_database, &key, true)?;
            {
                let copy = backup::Backup::new(&source, &mut destination)
                    .map_err(|_| StorageError::Unavailable)?;
                copy.run_to_completion(100, Duration::ZERO, None)
                    .map_err(|_| StorageError::Unavailable)?;
            }
            verify_schema(&destination)?;
            set_private_database_permissions(&destination_database)?;
            drop(destination);
            write_manifest_atomically(destination_root, &destination_manifest, &self.manifest)?;
            Ok(())
        })();

        if let Err(error) = result {
            let _ = remove_exact_database_files(&destination_database);
            let _ = fs::remove_file(&destination_manifest);
            let _ = fs::remove_dir(destination_root);
            return Err(error);
        }
        Ok(())
    }
}

fn restore_render_manifests(
    transaction: &Transaction<'_>,
    profile_id: &str,
    manifests: &[PortableRenderManifestV1],
) -> Result<(), StorageError> {
    for manifest in manifests {
        let manifest_id =
            Uuid::parse_str(&manifest.manifest_id).map_err(|_| StorageError::InvalidData)?;
        if manifest_id.get_version_num() != 7
            || manifest_id.to_string() != manifest.manifest_id
            || manifest.last_generated_at_unix_ms < manifest.generated_at_unix_ms
            || manifest.render_count == 0
        {
            return Err(StorageError::InvalidData);
        }
        validate_render_manifest(
            manifest.source_revision,
            manifest.generated_at_unix_ms,
            &manifest.receipt,
        )?;
        let generated_at =
            i64::try_from(manifest.generated_at_unix_ms).map_err(|_| StorageError::InvalidData)?;
        let last_generated_at = i64::try_from(manifest.last_generated_at_unix_ms)
            .map_err(|_| StorageError::InvalidData)?;
        let page_count =
            i64::try_from(manifest.receipt.page_count).map_err(|_| StorageError::InvalidData)?;
        let byte_count =
            i64::try_from(manifest.receipt.byte_count).map_err(|_| StorageError::InvalidData)?;
        transaction
            .execute(
                "INSERT INTO render_manifests \
                 (manifest_id, profile_id, source, source_revision, generated_at_unix_ms, \
                  last_generated_at_unix_ms, render_count, document_sha256, \
                  document_schema_version, pdf_sha256, renderer_version, template_id, \
                  template_sha256, font_bundle_id, font_bundle_sha256, page_count, byte_count) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    manifest.manifest_id,
                    profile_id,
                    export_source_code(manifest.source),
                    manifest.source_revision,
                    generated_at,
                    last_generated_at,
                    i64::from(manifest.render_count),
                    manifest.receipt.document_sha256,
                    i64::from(manifest.receipt.document_schema_version),
                    manifest.receipt.pdf_sha256,
                    manifest.receipt.renderer_version,
                    manifest.receipt.template_id,
                    manifest.receipt.template_sha256,
                    manifest.receipt.font_bundle_id,
                    manifest.receipt.font_bundle_sha256,
                    page_count,
                    byte_count,
                ],
            )
            .map_err(|_| StorageError::Unavailable)?;
    }
    Ok(())
}

fn read_portable_render_manifests(
    transaction: &Transaction<'_>,
    profile_id: &str,
) -> Result<Vec<PortableRenderManifestV1>, StorageError> {
    let mut statement = transaction
        .prepare(
            "SELECT manifest_id, source, source_revision, generated_at_unix_ms, \
             last_generated_at_unix_ms, render_count, document_sha256, \
             document_schema_version, pdf_sha256, renderer_version, template_id, \
             template_sha256, font_bundle_id, font_bundle_sha256, page_count, byte_count \
             FROM render_manifests WHERE profile_id = ?1 \
             ORDER BY last_generated_at_unix_ms DESC, manifest_id DESC \
             LIMIT ?2",
        )
        .map_err(|_| StorageError::Unavailable)?;
    statement
        .query_map(
            params![profile_id, MAX_RENDER_MANIFESTS + 1],
            render_manifest_row,
        )
        .map_err(|_| StorageError::Unavailable)?
        .map(|row| {
            let stored = parse_render_manifest(row.map_err(|_| StorageError::Unavailable)?)?;
            Ok(PortableRenderManifestV1 {
                manifest_id: stored.manifest_id.to_string(),
                source: stored.source,
                source_revision: stored.source_revision,
                generated_at_unix_ms: stored.generated_at_unix_ms,
                last_generated_at_unix_ms: stored.last_generated_at_unix_ms,
                render_count: stored.render_count,
                receipt: stored.receipt,
            })
        })
        .collect()
}

type RenderManifestRow = (
    String,
    String,
    i64,
    i64,
    i64,
    i64,
    String,
    i64,
    String,
    String,
    String,
    String,
    String,
    String,
    i64,
    i64,
);

fn render_manifest_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RenderManifestRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
        row.get(13)?,
        row.get(14)?,
        row.get(15)?,
    ))
}

fn parse_render_manifest(row: RenderManifestRow) -> Result<StoredRenderManifest, StorageError> {
    let (
        manifest_id,
        source,
        source_revision,
        generated_at,
        last_generated_at,
        render_count,
        document_sha256,
        document_schema_version,
        pdf_sha256,
        renderer_version,
        template_id,
        template_sha256,
        font_bundle_id,
        font_bundle_sha256,
        page_count,
        byte_count,
    ) = row;
    let manifest_id_text = manifest_id;
    let manifest_id = Uuid::parse_str(&manifest_id_text).map_err(|_| StorageError::InvalidData)?;
    if manifest_id.get_version_num() != 7 || manifest_id.to_string() != manifest_id_text {
        return Err(StorageError::InvalidData);
    }
    let source = parse_export_source(&source)?;
    let generated_at_unix_ms =
        u64::try_from(generated_at).map_err(|_| StorageError::InvalidData)?;
    let last_generated_at_unix_ms =
        u64::try_from(last_generated_at).map_err(|_| StorageError::InvalidData)?;
    let render_count = u32::try_from(render_count).map_err(|_| StorageError::InvalidData)?;
    let document_schema_version =
        u16::try_from(document_schema_version).map_err(|_| StorageError::InvalidData)?;
    let page_count = usize::try_from(page_count).map_err(|_| StorageError::InvalidData)?;
    let byte_count = usize::try_from(byte_count).map_err(|_| StorageError::InvalidData)?;
    let receipt = PdfRenderReceipt {
        document_sha256,
        document_schema_version,
        pdf_sha256,
        renderer_version,
        template_id,
        template_sha256,
        font_bundle_id,
        font_bundle_sha256,
        page_count,
        byte_count,
    };
    validate_render_manifest(source_revision, generated_at_unix_ms, &receipt)?;
    if last_generated_at_unix_ms < generated_at_unix_ms
        || last_generated_at_unix_ms > MAX_JAVASCRIPT_DATE_MS
        || render_count == 0
    {
        return Err(StorageError::InvalidData);
    }
    Ok(StoredRenderManifest {
        manifest_id,
        source,
        source_revision,
        generated_at_unix_ms,
        last_generated_at_unix_ms,
        render_count,
        receipt,
    })
}

const fn export_source_code(source: ExportSource) -> &'static str {
    match source {
        ExportSource::SavedDraft => "saved_draft",
        ExportSource::PublishedSnapshot => "published_snapshot",
    }
}

fn parse_export_source(value: &str) -> Result<ExportSource, StorageError> {
    match value {
        "saved_draft" => Ok(ExportSource::SavedDraft),
        "published_snapshot" => Ok(ExportSource::PublishedSnapshot),
        _ => Err(StorageError::InvalidData),
    }
}

fn validate_render_manifest(
    source_revision: i64,
    generated_at_unix_ms: u64,
    receipt: &PdfRenderReceipt,
) -> Result<(), StorageError> {
    let valid_hash = |value: &str| {
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    };
    let valid_id = |value: &str| {
        !value.is_empty()
            && value.len() <= 128
            && value.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/')
            })
    };
    if !(1..=9_007_199_254_740_991).contains(&source_revision)
        || !(1..=MAX_JAVASCRIPT_DATE_MS).contains(&generated_at_unix_ms)
        || receipt.document_schema_version == 0
        || !(1..=MAX_PDF_PAGES).contains(&receipt.page_count)
        || !(1..=MAX_PDF_BYTES).contains(&receipt.byte_count)
        || !valid_hash(&receipt.document_sha256)
        || !valid_hash(&receipt.pdf_sha256)
        || !valid_hash(&receipt.template_sha256)
        || !valid_hash(&receipt.font_bundle_sha256)
        || !valid_id(&receipt.renderer_version)
        || !valid_id(&receipt.template_id)
        || !valid_id(&receipt.font_bundle_id)
    {
        return Err(StorageError::InvalidData);
    }
    Ok(())
}

fn validate_channel(channel: &str) -> Result<(), StorageError> {
    let valid = !channel.is_empty()
        && channel.len() <= 32
        && channel
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(StorageError::InvalidData)
    }
}

fn validate_setting_key(key: &str) -> Result<(), StorageError> {
    let valid = !key.is_empty()
        && key.len() <= 64
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'));
    if valid {
        Ok(())
    } else {
        Err(StorageError::InvalidData)
    }
}

fn validate_diagnostic_code(code: &str) -> Result<(), StorageError> {
    let valid = !code.is_empty()
        && code.len() <= 64
        && code
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_');
    if valid {
        Ok(())
    } else {
        Err(StorageError::InvalidData)
    }
}

fn validate_safe_context(context: &Map<String, Value>) -> Result<(), StorageError> {
    if context.len() > 16 {
        return Err(StorageError::InvalidData);
    }
    for (key, value) in context {
        let normalized = key.to_ascii_lowercase();
        let safe_key = !key.is_empty()
            && key.len() <= 64
            && key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
            && ![
                "secret",
                "password",
                "token",
                "credential",
                "resume",
                "content",
            ]
            .iter()
            .any(|forbidden| normalized.contains(forbidden));
        if !safe_key || !is_safe_diagnostic_value(value) {
            return Err(StorageError::InvalidData);
        }
    }
    let json = serde_json::to_vec(context).map_err(|_| StorageError::InvalidData)?;
    if json.len() > 8 * 1_024 {
        return Err(StorageError::InvalidData);
    }
    Ok(())
}

fn is_safe_diagnostic_value(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(_) => true,
        Value::Number(number) => number.is_i64() || number.is_u64(),
        Value::String(text) => {
            !text.is_empty()
                && text.len() <= 128
                && text.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':')
                })
        }
        Value::Array(_) | Value::Object(_) => false,
    }
}

fn reject_symlink(path: &Path) -> Result<(), StorageError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(StorageError::UnsafeLocation),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(StorageError::Unavailable),
    }
}

fn create_private_directory(path: &Path) -> Result<(), StorageError> {
    fs::create_dir_all(path).map_err(|_| StorageError::Unavailable)?;
    if !path.is_dir() {
        return Err(StorageError::UnsafeLocation);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| StorageError::Unavailable)?;
    }
    Ok(())
}

fn set_private_file_permissions(path: &Path) -> Result<(), StorageError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|_| StorageError::Unavailable)?;
    }
    Ok(())
}

fn set_private_database_permissions(database_path: &Path) -> Result<(), StorageError> {
    set_private_file_permissions(database_path)?;
    let database_name = database_path.file_name().ok_or(StorageError::Unavailable)?;
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut sidecar_name = database_name.to_os_string();
        sidecar_name.push(suffix);
        let sidecar = database_path.with_file_name(sidecar_name);
        if sidecar.exists() {
            set_private_file_permissions(&sidecar)?;
        }
    }
    Ok(())
}

fn read_manifest(path: &Path, channel: &str) -> Result<ProfileManifest, StorageError> {
    let file = File::open(path).map_err(|_| StorageError::Unavailable)?;
    let metadata = file.metadata().map_err(|_| StorageError::Unavailable)?;
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err(StorageError::InvalidManifest);
    }
    let capacity = usize::try_from(metadata.len()).map_err(|_| StorageError::InvalidManifest)?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| StorageError::Unavailable)?;
    if u64::try_from(bytes.len()).map_err(|_| StorageError::InvalidManifest)? > MAX_MANIFEST_BYTES {
        return Err(StorageError::InvalidManifest);
    }
    let manifest: ProfileManifest =
        serde_json::from_slice(&bytes).map_err(|_| StorageError::InvalidManifest)?;
    manifest.validate(channel)?;
    Ok(manifest)
}

fn write_manifest_atomically(
    root: &Path,
    destination: &Path,
    manifest: &ProfileManifest,
) -> Result<(), StorageError> {
    let temporary = root.join(MANIFEST_UPDATE_FILENAME);
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|_| StorageError::Unavailable)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|_| StorageError::Unavailable)?;
    set_private_file_permissions(&temporary)?;
    if file
        .write_all(&bytes)
        .and_then(|()| file.sync_all())
        .is_err()
    {
        let _ = fs::remove_file(&temporary);
        return Err(StorageError::Unavailable);
    }
    if destination.exists() {
        let _ = fs::remove_file(&temporary);
        return Err(StorageError::IncompleteInitialization);
    }
    fs::rename(&temporary, destination).map_err(|_| StorageError::Unavailable)?;
    set_private_file_permissions(destination)?;
    #[cfg(unix)]
    File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| StorageError::Unavailable)?;
    Ok(())
}

fn replace_manifest_atomically(
    root: &Path,
    destination: &Path,
    manifest: &ProfileManifest,
) -> Result<(), StorageError> {
    if !destination.is_file() {
        return Err(StorageError::IncompleteInitialization);
    }
    remove_previous_manifest(root)?;
    let previous = root.join(PREVIOUS_MANIFEST_FILENAME);
    let temporary = root.join(MANIFEST_UPDATE_FILENAME);
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|_| StorageError::Unavailable)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|_| StorageError::Unavailable)?;
    set_private_file_permissions(&temporary)?;
    if file
        .write_all(&bytes)
        .and_then(|()| file.sync_all())
        .is_err()
    {
        let _ = fs::remove_file(&temporary);
        return Err(StorageError::Unavailable);
    }
    drop(file);
    fs::rename(destination, &previous).map_err(|_| StorageError::Unavailable)?;
    if fs::rename(&temporary, destination).is_err() {
        let _ = fs::rename(&previous, destination);
        let _ = fs::remove_file(&temporary);
        return Err(StorageError::Unavailable);
    }
    set_private_file_permissions(destination)?;
    sync_directory(root)?;
    fs::remove_file(&previous).map_err(|_| StorageError::Unavailable)?;
    sync_directory(root)
}

fn recover_manifest_update(
    root: &Path,
    destination: &Path,
    database: &Path,
) -> Result<(), StorageError> {
    let previous = root.join(PREVIOUS_MANIFEST_FILENAME);
    if previous.exists() {
        reject_symlink(&previous)?;
        if !previous.is_file() {
            return Err(StorageError::IncompleteInitialization);
        }
        if !destination.exists() {
            if !database.is_file() {
                return Err(StorageError::IncompleteInitialization);
            }
            fs::rename(&previous, destination).map_err(|_| StorageError::Unavailable)?;
            sync_directory(root)?;
        }
    }
    remove_exact_optional_file(&root.join(MANIFEST_UPDATE_FILENAME))
}

fn remove_previous_manifest(root: &Path) -> Result<(), StorageError> {
    let previous = root.join(PREVIOUS_MANIFEST_FILENAME);
    remove_exact_optional_file(&previous)?;
    sync_directory(root)
}

fn remove_exact_optional_file(path: &Path) -> Result<(), StorageError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(StorageError::UnsafeLocation)
        }
        Ok(_) => fs::remove_file(path).map_err(|_| StorageError::Unavailable),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(StorageError::Unavailable),
    }
}

fn sync_directory(root: &Path) -> Result<(), StorageError> {
    #[cfg(unix)]
    File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| StorageError::Unavailable)?;
    Ok(())
}

fn open_encrypted_connection(
    path: &Path,
    key: &DatabaseKey,
    create: bool,
) -> Result<Connection, StorageError> {
    let mut flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    if create {
        flags |= OpenFlags::SQLITE_OPEN_CREATE;
    }
    let connection =
        Connection::open_with_flags(path, flags).map_err(|_| StorageError::Unavailable)?;
    // Reject builds that lost the repository's SQLCIPHER_OMIT_LOG policy before
    // applying a key or enabling memory security. In the pinned native source,
    // this fixed target returns SQLITE_ERROR only when logging is compiled out.
    // Never pass a path here: other strings can create a native log file.
    let logging_status: String = connection
        .query_row("PRAGMA cipher_log = 'stderr'", [], |row| row.get(0))
        .map_err(|_| StorageError::CipherUnavailable)?;
    if logging_status != "1" {
        return Err(StorageError::CipherUnavailable);
    }
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|_| StorageError::Unavailable)?;

    apply_database_key(&connection, key)?;
    apply_cipher_settings(&connection)?;

    let cipher_version: String = connection
        .query_row("PRAGMA cipher_version", [], |row| row.get(0))
        .map_err(|_| StorageError::CipherUnavailable)?;
    if cipher_version.trim().is_empty() {
        return Err(StorageError::CipherUnavailable);
    }

    connection
        .query_row("SELECT count(*) FROM sqlite_master", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|_| StorageError::DatabaseKeyMismatch)?;
    apply_connection_settings(&connection)?;
    Ok(connection)
}

fn apply_database_key(connection: &Connection, key: &DatabaseKey) -> Result<(), StorageError> {
    key.expose_for(|bytes| {
        let mut literal = format!("x'{}'", hex::encode(bytes));
        let result = connection.pragma_update(None, "key", &literal);
        literal.zeroize();
        result.map_err(|_| StorageError::DatabaseKeyMismatch)
    })
}

fn apply_cipher_settings(connection: &Connection) -> Result<(), StorageError> {
    for (name, value) in [
        ("cipher_compatibility", "4"),
        ("cipher_page_size", "4096"),
        ("kdf_iter", "256000"),
        ("cipher_kdf_algorithm", "PBKDF2_HMAC_SHA512"),
        ("cipher_hmac_algorithm", "HMAC_SHA512"),
        ("cipher_use_hmac", "ON"),
        ("cipher_plaintext_header_size", "0"),
        ("cipher_memory_security", "ON"),
    ] {
        connection
            .pragma_update(None, name, value)
            .map_err(|_| StorageError::CipherUnavailable)?;
    }
    Ok(())
}

fn apply_connection_settings(connection: &Connection) -> Result<(), StorageError> {
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .and_then(|()| connection.pragma_update(None, "secure_delete", "ON"))
        .and_then(|()| connection.pragma_update(None, "trusted_schema", "OFF"))
        .and_then(|()| connection.pragma_update(None, "synchronous", "FULL"))
        .map_err(|_| StorageError::Unavailable)?;
    let mode: String = connection
        .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
        .map_err(|_| StorageError::Unavailable)?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(StorageError::Unavailable);
    }
    Ok(())
}

fn initialize_schema(
    connection: &Connection,
    manifest: &ProfileManifest,
) -> Result<(), StorageError> {
    connection
        .execute_batch("BEGIN IMMEDIATE")
        .map_err(|_| StorageError::Unavailable)?;
    let result = (|| {
        connection
            .execute_batch(MIGRATION_V1_SQL)
            .map_err(|_| StorageError::Unavailable)?;
        let now = now_string();
        connection
            .execute(
                "INSERT INTO schema_migrations \
                 (version, checksum_sha256, minimum_app_version, estimated_disk_bytes, \
                  requires_safety_copy, applied_at) \
                 VALUES (1, ?1, ?2, 0, 0, ?3)",
                params![migration_v1_checksum(), "0.0.0-dev", now],
            )
            .and_then(|_| {
                connection.execute(
                    "INSERT INTO app_metadata (metadata_key, metadata_value) \
                     VALUES ('database_format_version', ?1)",
                    [DATABASE_FORMAT_VERSION.to_string()],
                )
            })
            .and_then(|_| {
                connection.execute(
                    "INSERT INTO profiles (profile_id, revision, created_at, updated_at) \
                     VALUES (?1, 1, ?2, ?2)",
                    params![manifest.profile_id.to_string(), now],
                )
            })
            .map_err(|_| StorageError::Unavailable)?;
        connection
            .execute_batch("COMMIT")
            .map_err(|_| StorageError::Unavailable)
    })();
    if result.is_err() {
        let _ = connection.execute_batch("ROLLBACK");
    }
    result
}

fn migrate_schema(connection: &Connection) -> Result<(), StorageError> {
    let migrations = load_migration_receipts(connection)?;
    let latest = migrations
        .last()
        .map(|(version, _)| *version)
        .ok_or(StorageError::IntegrityFailure)?;
    if latest > SCHEMA_VERSION {
        return Err(StorageError::NewerSchema);
    }
    verify_migration_receipts(&migrations)?;
    if latest == SCHEMA_VERSION {
        return Ok(());
    }
    if latest != 1 {
        return Err(StorageError::IntegrityFailure);
    }

    connection
        .execute_batch("BEGIN IMMEDIATE")
        .map_err(|_| StorageError::Unavailable)?;
    let result = (|| {
        connection
            .execute_batch(MIGRATION_V2_SQL)
            .map_err(|_| StorageError::Unavailable)?;
        connection
            .execute(
                "INSERT INTO schema_migrations \
                 (version, checksum_sha256, minimum_app_version, estimated_disk_bytes, \
                  requires_safety_copy, applied_at) \
                 VALUES (2, ?1, ?2, 0, 0, ?3)",
                params![migration_v2_checksum(), "0.0.0-dev", now_string()],
            )
            .map_err(|_| StorageError::Unavailable)?;
        connection
            .execute_batch("COMMIT")
            .map_err(|_| StorageError::Unavailable)
    })();
    if result.is_err() {
        let _ = connection.execute_batch("ROLLBACK");
    }
    result
}

fn verify_schema(connection: &Connection) -> Result<(), StorageError> {
    let migrations = load_migration_receipts(connection)?;
    let latest = migrations
        .last()
        .map(|(version, _)| *version)
        .ok_or(StorageError::IntegrityFailure)?;
    if latest > SCHEMA_VERSION {
        return Err(StorageError::NewerSchema);
    }
    if latest != SCHEMA_VERSION {
        return Err(StorageError::IntegrityFailure);
    }
    verify_migration_receipts(&migrations)?;
    verify_integrity(connection)
}

fn load_migration_receipts(connection: &Connection) -> Result<Vec<(i64, String)>, StorageError> {
    let mut statement = connection
        .prepare("SELECT version, checksum_sha256 FROM schema_migrations ORDER BY version")
        .map_err(|_| StorageError::IntegrityFailure)?;
    statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|_| StorageError::IntegrityFailure)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StorageError::IntegrityFailure)
}

fn verify_migration_receipts(migrations: &[(i64, String)]) -> Result<(), StorageError> {
    let expected = [(1, migration_v1_checksum()), (2, migration_v2_checksum())];
    if migrations.len() > expected.len()
        || migrations.iter().zip(expected).any(
            |((version, checksum), (expected_version, expected_checksum))| {
                *version != expected_version || *checksum != expected_checksum
            },
        )
    {
        return Err(StorageError::IntegrityFailure);
    }
    Ok(())
}

fn migration_v1_checksum() -> String {
    hex::encode(Sha256::digest(MIGRATION_V1_SQL.as_bytes()))
}

fn migration_v2_checksum() -> String {
    hex::encode(Sha256::digest(MIGRATION_V2_SQL.as_bytes()))
}

fn verify_integrity(connection: &Connection) -> Result<(), StorageError> {
    let mut cipher_statement = connection
        .prepare("PRAGMA cipher_integrity_check")
        .map_err(|_| StorageError::CipherUnavailable)?;
    let cipher_errors: Vec<String> = cipher_statement
        .query_map([], |row| row.get(0))
        .map_err(|_| StorageError::IntegrityFailure)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StorageError::IntegrityFailure)?;
    if !cipher_errors.is_empty() {
        return Err(StorageError::IntegrityFailure);
    }
    let result: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|_| StorageError::IntegrityFailure)?;
    if result != "ok" {
        return Err(StorageError::IntegrityFailure);
    }
    Ok(())
}

fn serialize_document(document: &ResumeDocument) -> Result<Vec<u8>, StorageError> {
    document
        .validate(DocumentLimits::default())
        .map_err(|_| StorageError::InvalidData)?;
    serde_json::to_vec(document).map_err(|_| StorageError::InvalidData)
}

fn parse_document(json: &[u8]) -> Result<ResumeDocument, StorageError> {
    let document: ResumeDocument =
        serde_json::from_slice(json).map_err(|_| StorageError::InvalidData)?;
    document
        .validate(DocumentLimits::default())
        .map_err(|_| StorageError::InvalidData)?;
    Ok(document)
}

fn parse_versioned_resume(revision: i64, json: &[u8]) -> Result<VersionedResume, StorageError> {
    Ok(VersionedResume {
        revision,
        document: parse_document(json)?,
    })
}

fn now_string() -> String {
    Timestamp::now().to_string()
}

fn map_vault_load_error(error: &VaultError) -> StorageError {
    match error {
        VaultError::Missing | VaultError::Unavailable | VaultError::CorruptSecret => {
            StorageError::VaultKeyUnavailable
        }
        VaultError::InvalidReference
        | VaultError::RandomUnavailable
        | VaultError::AlreadyExists => StorageError::InvalidManifest,
    }
}

fn map_vault_creation_error(error: &VaultError) -> StorageError {
    match error {
        VaultError::AlreadyExists => StorageError::IncompleteInitialization,
        VaultError::InvalidReference => StorageError::InvalidManifest,
        VaultError::Missing
        | VaultError::Unavailable
        | VaultError::CorruptSecret
        | VaultError::RandomUnavailable => StorageError::VaultKeyUnavailable,
    }
}

fn map_backup_write_error(error: &BackupError) -> StorageError {
    match error {
        BackupError::InvalidPassphrase | BackupError::InvalidContent => StorageError::InvalidData,
        BackupError::InvalidBackup | BackupError::CryptoUnavailable => StorageError::Unavailable,
    }
}

fn map_backup_read_error(error: &BackupError) -> StorageError {
    match error {
        BackupError::InvalidBackup
        | BackupError::InvalidPassphrase
        | BackupError::InvalidContent => StorageError::InvalidData,
        BackupError::CryptoUnavailable => StorageError::Unavailable,
    }
}

fn is_constraint_error(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(code, _)
            if matches!(code.code, rusqlite::ErrorCode::ConstraintViolation)
    )
}

fn remove_exact_database_files(database_path: &Path) -> Result<(), StorageError> {
    let mut paths = vec![database_path.to_path_buf()];
    let database_name = database_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(StorageError::Unavailable)?;
    for suffix in ["-wal", "-shm", "-journal"] {
        paths.push(database_path.with_file_name(format!("{database_name}{suffix}")));
    }
    for path in paths {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(StorageError::Unavailable),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use ort_backup::BackupPassphrase;
    use ort_domain::{ExportSource, PdfRenderReceipt, ResumeDocument};
    use ort_vault::DatabaseKeyVault;
    use ort_vault::testing::MemoryDatabaseKeyVault;
    use serde_json::{Map, Value};
    use tempfile::TempDir;

    use super::{DiagnosticSeverity, EncryptedStore, StorageError};

    const PLAINTEXT_MARKER: &str = "SYNTHETIC-PRIVATE-RESUME-MARKER-9d87f8";

    fn synthetic_render_receipt(pdf_sha256: String) -> PdfRenderReceipt {
        PdfRenderReceipt {
            document_sha256: "a".repeat(64),
            document_schema_version: 1,
            pdf_sha256,
            renderer_version: "typst-0.15.1/ort-1".to_owned(),
            template_id: "plain_pdf_v1".to_owned(),
            template_sha256: "b".repeat(64),
            font_bundle_id: "libertinus-serif/typst-assets-0.15.1".to_owned(),
            font_bundle_sha256: "c".repeat(64),
            page_count: 1,
            byte_count: 1024,
        }
    }

    #[test]
    fn encrypted_profile_round_trips_without_plaintext_on_disk() {
        let temporary = TempDir::new().expect("temporary directory");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("initialize encrypted store");
        {
            // Removing native logging must not turn off memory protection or
            // keying. Assert the effective native values, not just our setters.
            let connection = store.connection.lock().expect("storage lock");
            for pragma in ["cipher_memory_security", "cipher_status"] {
                let enabled: String = connection
                    .pragma_query_value(None, pragma, |row| row.get(0))
                    .expect("effective cipher protection");
                assert_eq!(enabled, "1", "{pragma} must remain enabled");
            }
        }
        let document = ResumeDocument::empty(PLAINTEXT_MARKER);
        let created = store.create_draft(&document).expect("create draft");
        assert_eq!(created.revision, 1);
        store.verify_integrity().expect("integrity");
        let wal_path = store.database_path().with_file_name("profile.db-wal");
        assert!(wal_path.is_file(), "the write must exercise encrypted WAL");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(store.database_path())
                    .expect("database metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(&wal_path)
                    .expect("WAL metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        assert_no_marker_in_files(temporary.path());
        drop(store);

        let reopened = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("reopen encrypted store");
        let loaded = reopened
            .load_draft()
            .expect("load draft")
            .expect("draft exists");
        assert_eq!(loaded.document.title, PLAINTEXT_MARKER);
        drop(reopened);

        assert_no_marker_in_files(temporary.path());
    }

    #[test]
    fn missing_vault_key_never_replaces_or_mutates_the_database() {
        let temporary = TempDir::new().expect("temporary directory");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("initialize encrypted store");
        let reference = store.manifest().vault_reference().expect("vault reference");
        let database_path = store.database_path().to_path_buf();
        drop(store);
        let before = fs::read(&database_path).expect("read before");
        vault.delete(&reference).expect("delete test key");

        assert_eq!(
            EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
                .err()
                .expect("must fail"),
            StorageError::VaultKeyUnavailable
        );
        let after = fs::read(database_path).expect("read after");
        assert_eq!(before, after);
    }

    #[test]
    fn wrong_vault_key_cannot_open_or_mutate_the_database() {
        use ort_vault::DatabaseKey;

        let temporary = TempDir::new().expect("temporary directory");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("initialize encrypted store");
        let reference = store.manifest().vault_reference().expect("vault reference");
        let database_path = store.database_path().to_path_buf();
        store
            .create_draft(&ResumeDocument::empty(PLAINTEXT_MARKER))
            .expect("create draft");
        drop(store);
        let before = fs::read(&database_path).expect("read before");
        let wrong_key = DatabaseKey::generate().expect("generate wrong key");
        vault
            .replace_for_test(&reference, &wrong_key)
            .expect("replace test key");

        assert_eq!(
            EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
                .err()
                .expect("must fail"),
            StorageError::DatabaseKeyMismatch
        );
        let after = fs::read(database_path).expect("read after");
        assert_eq!(before, after);
    }

    #[test]
    fn committed_encrypted_wal_recovers_after_crash_snapshot() {
        let temporary = TempDir::new().expect("temporary directory");
        let profile_root = temporary.path().join("profile");
        let crash_root = temporary.path().join("crash-snapshot");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(&profile_root, "test", &vault)
            .expect("initialize encrypted store");
        store
            .create_draft(&ResumeDocument::empty(PLAINTEXT_MARKER))
            .expect("create draft");

        fs::create_dir(&crash_root).expect("create crash snapshot directory");
        for filename in ["profile.json", "profile.db", "profile.db-wal"] {
            let source = profile_root.join(filename);
            assert!(source.is_file(), "crash source must contain {filename}");
            fs::copy(source, crash_root.join(filename)).expect("copy crash snapshot file");
        }
        assert_no_marker_in_files(&crash_root);

        let recovered = EncryptedStore::open_or_initialize(&crash_root, "test", &vault)
            .expect("recover committed encrypted WAL");
        let draft = recovered
            .load_draft()
            .expect("load recovered draft")
            .expect("recovered draft exists");
        assert_eq!(draft.document.title, PLAINTEXT_MARKER);
        drop(recovered);
        drop(store);
        assert_no_marker_in_files(&crash_root);
    }

    #[test]
    fn ciphertext_corruption_fails_closed_without_rewriting_database() {
        let temporary = TempDir::new().expect("temporary directory");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("initialize encrypted store");
        store
            .create_draft(&ResumeDocument::empty(PLAINTEXT_MARKER))
            .expect("create draft");
        let database_path = store.database_path().to_path_buf();
        drop(store);

        let mut corrupted = fs::read(&database_path).expect("read encrypted database");
        assert!(
            corrupted.len() > 4_196,
            "database must contain multiple pages"
        );
        corrupted[4_196] ^= 0xA5;
        fs::write(&database_path, &corrupted).expect("write synthetic corruption");

        assert_eq!(
            EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
                .err()
                .expect("corruption must fail closed"),
            StorageError::IntegrityFailure
        );
        assert_eq!(
            fs::read(database_path).expect("read failed database"),
            corrupted
        );
    }

    #[test]
    fn newer_schema_is_refused_without_downgrade() {
        let temporary = TempDir::new().expect("temporary directory");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("initialize encrypted store");
        {
            let connection = store.connection.lock().expect("lock test connection");
            connection
                .execute(
                    "INSERT INTO schema_migrations \
                     (version, checksum_sha256, minimum_app_version, estimated_disk_bytes, \
                      requires_safety_copy, applied_at) \
                     VALUES (3, 'synthetic-newer', '9.0.0', 0, 0, ?1)",
                    [super::now_string()],
                )
                .expect("seed newer schema marker");
        }
        let database_path = store.database_path().to_path_buf();
        drop(store);
        let before = fs::read(&database_path).expect("read before");

        assert_eq!(
            EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
                .err()
                .expect("newer schema must be refused"),
            StorageError::NewerSchema
        );
        assert_eq!(fs::read(database_path).expect("read after refusal"), before);
    }

    #[test]
    fn migration_checksum_tampering_is_rejected() {
        let temporary = TempDir::new().expect("temporary directory");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("initialize encrypted store");
        {
            let connection = store.connection.lock().expect("lock test connection");
            connection
                .execute(
                    "UPDATE schema_migrations SET checksum_sha256 = 'tampered' WHERE version = 1",
                    [],
                )
                .expect("tamper migration checksum");
        }
        let database_path = store.database_path().to_path_buf();
        drop(store);
        let before = fs::read(&database_path).expect("read before");
        assert_eq!(
            EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
                .err()
                .expect("checksum mismatch must fail"),
            StorageError::IntegrityFailure
        );
        assert_eq!(fs::read(database_path).expect("read after refusal"), before);
    }

    #[test]
    fn schema_v1_profile_upgrades_additively_and_updates_its_manifest() {
        let temporary = TempDir::new().expect("temporary directory");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("initialize encrypted store");
        store
            .create_draft(&ResumeDocument::empty(PLAINTEXT_MARKER))
            .expect("create draft");
        {
            let connection = store.connection.lock().expect("lock test connection");
            connection
                .execute_batch(
                    "DROP TABLE render_manifests; DELETE FROM schema_migrations WHERE version = 2;",
                )
                .expect("simulate schema v1 database");
        }
        drop(store);
        let manifest_path = temporary.path().join("profile.json");
        let mut manifest: Value = serde_json::from_slice(
            &fs::read(&manifest_path).expect("read manifest for v1 simulation"),
        )
        .expect("parse manifest");
        manifest["schemaVersion"] = Value::from(1);
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).expect("serialize v1 manifest"),
        )
        .expect("write v1 manifest");

        let upgraded = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("upgrade schema v1 profile");
        assert_eq!(upgraded.manifest().schema_version, 2);
        assert_eq!(
            upgraded
                .load_draft()
                .expect("load upgraded draft")
                .expect("draft exists")
                .document
                .title,
            PLAINTEXT_MARKER
        );
        let connection = upgraded.connection.lock().expect("lock upgraded database");
        let versions: Vec<i64> = connection
            .prepare("SELECT version FROM schema_migrations ORDER BY version")
            .expect("prepare migration query")
            .query_map([], |row| row.get(0))
            .expect("query migration versions")
            .collect::<Result<_, _>>()
            .expect("collect migration versions");
        assert_eq!(versions, vec![1, 2]);
    }

    #[test]
    fn interrupted_manifest_replacement_recovers_the_previous_exact_file() {
        let temporary = TempDir::new().expect("temporary directory");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("initialize encrypted store");
        store
            .create_draft(&ResumeDocument::empty(PLAINTEXT_MARKER))
            .expect("create draft");
        drop(store);
        let manifest = temporary.path().join(super::MANIFEST_FILENAME);
        let previous = temporary.path().join(super::PREVIOUS_MANIFEST_FILENAME);
        fs::rename(&manifest, &previous).expect("simulate interrupted manifest handoff");
        fs::write(
            temporary.path().join(super::MANIFEST_UPDATE_FILENAME),
            b"partial replacement",
        )
        .expect("seed interrupted temporary manifest");

        let recovered = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("recover exact previous manifest");
        assert!(manifest.is_file());
        assert!(!previous.exists());
        assert!(
            !temporary
                .path()
                .join(super::MANIFEST_UPDATE_FILENAME)
                .exists()
        );
        assert_eq!(
            recovered
                .load_draft()
                .expect("load recovered draft")
                .expect("draft exists")
                .document
                .title,
            PLAINTEXT_MARKER
        );
    }

    #[test]
    fn render_manifests_are_encrypted_deduplicated_bounded_and_reopenable() {
        let temporary = TempDir::new().expect("temporary directory");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("initialize encrypted store");
        let receipt = synthetic_render_receipt("d".repeat(64));
        let first = store
            .record_render_manifest(ExportSource::SavedDraft, 1, 1_000, &receipt)
            .expect("record first render");
        let repeated = store
            .record_render_manifest(ExportSource::SavedDraft, 1, 2_000, &receipt)
            .expect("record repeated render");
        assert_eq!(repeated.manifest_id, first.manifest_id);
        assert_eq!(repeated.generated_at_unix_ms, 1_000);
        assert_eq!(repeated.last_generated_at_unix_ms, 2_000);
        assert_eq!(repeated.render_count, 2);

        for index in 1..=100_u64 {
            let receipt = synthetic_render_receipt(format!("{index:064x}"));
            store
                .record_render_manifest(
                    ExportSource::PublishedSnapshot,
                    i64::try_from(index).expect("revision"),
                    2_000 + index,
                    &receipt,
                )
                .expect("record bounded manifest");
        }
        let count: i64 = store
            .connection
            .lock()
            .expect("connection")
            .query_row("SELECT COUNT(*) FROM render_manifests", [], |row| {
                row.get(0)
            })
            .expect("count manifests");
        assert_eq!(count, super::MAX_RENDER_MANIFESTS);
        assert_eq!(
            store
                .load_recent_render_manifests(50)
                .expect("load manifests")
                .len(),
            50
        );
        assert_eq!(
            store.load_recent_render_manifests(51),
            Err(StorageError::InvalidData)
        );
        assert_no_marker_in_files(temporary.path());
        drop(store);

        let reopened = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("reopen encrypted store");
        let recent = reopened
            .load_recent_render_manifests(1)
            .expect("load newest manifest");
        assert_eq!(recent[0].source_revision, 100);
        assert_eq!(recent[0].receipt.pdf_sha256, format!("{:064x}", 100));
    }

    #[test]
    fn orphan_database_is_preserved_as_incomplete_initialization() {
        let temporary = TempDir::new().expect("temporary directory");
        let database_path = temporary.path().join("profile.db");
        let marker = b"synthetic incomplete initialization";
        fs::write(&database_path, marker).expect("seed orphan database");
        let vault = MemoryDatabaseKeyVault::new();
        assert_eq!(
            EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
                .err()
                .expect("orphan database must fail"),
            StorageError::IncompleteInitialization
        );
        assert_eq!(
            fs::read(database_path).expect("read preserved database"),
            marker
        );
    }

    #[test]
    fn diagnostics_accept_only_bounded_non_sensitive_metadata() {
        let temporary = TempDir::new().expect("temporary directory");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("initialize encrypted store");
        let mut context = Map::new();
        context.insert(
            "operation_id".to_owned(),
            Value::String("op-123".to_owned()),
        );
        context.insert("attempt".to_owned(), Value::from(2));
        let recorded = store
            .record_diagnostic("STORAGE_RETRY", DiagnosticSeverity::Warning, &context)
            .expect("record safe diagnostic");
        let loaded = store.load_recent_diagnostics(10).expect("load diagnostics");
        assert_eq!(loaded, vec![recorded]);

        let mut unsafe_context = Map::new();
        unsafe_context.insert(
            "api_token".to_owned(),
            Value::String("synthetic-secret".to_owned()),
        );
        assert_eq!(
            store.record_diagnostic("UNSAFE_EVENT", DiagnosticSeverity::Error, &unsafe_context),
            Err(StorageError::InvalidData)
        );
    }

    #[test]
    fn optimistic_revision_prevents_lost_updates() {
        let temporary = TempDir::new().expect("temporary directory");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("initialize encrypted store");
        let original = ResumeDocument::empty("Original");
        store.create_draft(&original).expect("create draft");

        let first = ResumeDocument::empty("First writer");
        store.save_draft(1, &first).expect("first writer wins");
        let second = ResumeDocument::empty("Second writer");
        assert_eq!(
            store.save_draft(1, &second),
            Err(StorageError::RevisionConflict)
        );
    }

    #[test]
    fn published_snapshot_does_not_change_with_later_draft_edits() {
        let temporary = TempDir::new().expect("temporary directory");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
            .expect("initialize encrypted store");
        store
            .create_draft(&ResumeDocument::empty("Published"))
            .expect("create draft");
        store.publish_draft(1).expect("publish");
        store
            .save_draft(1, &ResumeDocument::empty("Edited draft"))
            .expect("edit draft");
        let published = store
            .load_latest_published()
            .expect("load published")
            .expect("published exists");
        assert_eq!(published.document.title, "Published");
    }

    #[test]
    fn publishing_the_same_revision_is_idempotent() {
        let temporary = TempDir::new().expect("temporary directory");
        let vault = MemoryDatabaseKeyVault::new();
        let store =
            EncryptedStore::open_or_initialize(temporary.path(), "test", &vault).expect("store");
        store
            .create_draft(&ResumeDocument::empty("Synthetic"))
            .expect("draft");
        let first = store.publish_draft(1).expect("first publication");
        let repeated = store.publish_draft(1).expect("repeated publication");
        assert_eq!(first, repeated);
        let connection = store.connection.lock().expect("connection");
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM published_resumes", [], |row| {
                row.get(0)
            })
            .expect("count");
        assert_eq!(count, 1);
    }

    #[test]
    fn invalid_persisted_draft_cannot_create_a_published_snapshot() {
        let temporary = TempDir::new().expect("temporary directory");
        let vault = MemoryDatabaseKeyVault::new();
        let store =
            EncryptedStore::open_or_initialize(temporary.path(), "test", &vault).expect("store");
        store
            .create_draft(&ResumeDocument::empty("Synthetic"))
            .expect("draft");
        {
            let connection = store.connection.lock().expect("connection");
            connection
                .execute(
                    "UPDATE resume_drafts SET document_json = ?1",
                    [b"{}".as_slice()],
                )
                .expect("seed malformed draft");
        }
        assert!(store.publish_draft(1).is_err());
        assert_eq!(
            store.load_latest_published().expect("published query"),
            None
        );
    }

    #[test]
    fn encrypted_checkpoint_is_verified_and_reopenable() {
        let temporary = TempDir::new().expect("temporary directory");
        let profile_root = temporary.path().join("profile");
        let checkpoint_root = temporary.path().join("checkpoint");
        let vault = MemoryDatabaseKeyVault::new();
        let store = EncryptedStore::open_or_initialize(&profile_root, "test", &vault)
            .expect("initialize encrypted store");
        store
            .create_draft(&ResumeDocument::empty(PLAINTEXT_MARKER))
            .expect("create draft");
        store
            .create_encrypted_checkpoint(&checkpoint_root, &vault)
            .expect("create encrypted checkpoint");
        drop(store);

        let checkpoint = EncryptedStore::open_or_initialize(&checkpoint_root, "test", &vault)
            .expect("open checkpoint");
        let draft = checkpoint
            .load_draft()
            .expect("load checkpoint draft")
            .expect("draft exists");
        assert_eq!(draft.document.title, PLAINTEXT_MARKER);
        drop(checkpoint);

        let database = fs::read(checkpoint_root.join("profile.db")).expect("read checkpoint");
        assert!(!contains_subslice(&database, PLAINTEXT_MARKER.as_bytes()));
    }

    #[test]
    fn portable_backup_restores_into_a_fresh_keyed_profile() {
        let temporary = TempDir::new().expect("temporary directory");
        let source_root = temporary.path().join("source");
        let destination_root = temporary.path().join("destination");
        let vault = MemoryDatabaseKeyVault::new();
        let source = EncryptedStore::open_or_initialize(&source_root, "test", &vault)
            .expect("initialize source");
        source
            .create_draft(&ResumeDocument::empty(PLAINTEXT_MARKER))
            .expect("create draft");
        source.publish_draft(1).expect("publish draft");
        source
            .save_draft(1, &ResumeDocument::empty("Current synthetic draft"))
            .expect("advance draft");
        source
            .save_setting(
                "appearance.theme",
                None,
                &Value::String("system".to_owned()),
            )
            .expect("save setting");
        let render_manifest = source
            .record_render_manifest(
                ExportSource::PublishedSnapshot,
                1,
                1_725_192_000_000,
                &synthetic_render_receipt("e".repeat(64)),
            )
            .expect("record render manifest");
        let passphrase = BackupPassphrase::new("synthetic portable backup passphrase".to_owned())
            .expect("valid passphrase");
        let backup = source
            .create_portable_backup(&passphrase, "0.0.0-dev")
            .expect("create portable backup");
        assert!(!contains_subslice(&backup, PLAINTEXT_MARKER.as_bytes()));

        let destination = EncryptedStore::open_or_initialize(&destination_root, "test", &vault)
            .expect("initialize destination");
        assert_ne!(
            source
                .manifest()
                .vault_reference()
                .expect("source reference"),
            destination
                .manifest()
                .vault_reference()
                .expect("destination reference")
        );
        let wrong = BackupPassphrase::new("wrong synthetic passphrase".to_owned())
            .expect("valid passphrase");
        assert_eq!(
            destination.restore_portable_backup(&backup, &wrong),
            Err(StorageError::InvalidData)
        );
        assert!(
            destination
                .load_draft()
                .expect("load empty draft")
                .is_none()
        );

        destination
            .restore_portable_backup(&backup, &passphrase)
            .expect("restore portable backup");
        assert_eq!(
            destination
                .load_draft()
                .expect("load restored draft")
                .expect("draft exists")
                .document
                .title,
            "Current synthetic draft"
        );
        assert_eq!(
            destination
                .load_latest_published()
                .expect("load restored published")
                .expect("published exists")
                .document
                .title,
            PLAINTEXT_MARKER
        );
        assert_eq!(
            destination
                .load_setting("appearance.theme")
                .expect("load restored setting")
                .expect("setting exists")
                .value,
            Value::String("system".to_owned())
        );
        assert_eq!(
            destination
                .load_recent_render_manifests(20)
                .expect("load restored render history"),
            vec![render_manifest]
        );
        destination.verify_integrity().expect("restored integrity");
    }

    #[test]
    #[cfg(unix)]
    fn symlink_storage_root_is_rejected() {
        use std::os::unix::fs::symlink;

        let temporary = TempDir::new().expect("temporary directory");
        let real = temporary.path().join("real");
        fs::create_dir(&real).expect("create real directory");
        let linked = temporary.path().join("linked");
        symlink(&real, &linked).expect("create symlink");
        let vault = MemoryDatabaseKeyVault::new();
        assert_eq!(
            EncryptedStore::open_or_initialize(&linked, "test", &vault)
                .err()
                .expect("must reject symlink"),
            StorageError::UnsafeLocation
        );
    }

    fn contains_subslice(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    fn assert_no_marker_in_files(directory: &std::path::Path) {
        for entry in fs::read_dir(directory).expect("read profile directory") {
            let path = entry.expect("directory entry").path();
            if path.is_file() {
                let bytes = fs::read(path).expect("read profile file");
                assert!(!contains_subslice(&bytes, PLAINTEXT_MARKER.as_bytes()));
            }
        }
    }
}
