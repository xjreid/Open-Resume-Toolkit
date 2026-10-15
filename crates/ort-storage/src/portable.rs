//! Portable snapshot collection, kept separate from filesystem lifecycle recovery.
use crate::{
    AI_CONNECTION_SETTING_KEY, BTreeMap, BackupExportRequestV1, BackupPassphrase, Connection,
    EncryptedStore, MAX_RENDER_MANIFESTS, OptionalExtension, PortableAiAttemptV1,
    PortableAiOperationV1, PortableAiPriceV1, PortableProfileV1, PortablePublishedResumeV1,
    PortableRenderManifestV1, PortableResumeRevisionV1, PortableSettingV1, PortableTrackerEntryV1,
    Serialize, StorageError, Transaction, TransactionBehavior, ai_ledger, create_backup,
    map_backup_write_error, now_string, params, parse_document, parse_render_manifest,
    render_manifest_row, validate_setting_key,
};
use ort_backup::{MAX_PAYLOAD_BYTES, serialized_size};

#[derive(Default)]
struct CollectionBudget {
    used: usize,
}
impl CollectionBudget {
    fn record<T: Serialize>(&mut self, value: T) -> Result<T, StorageError> {
        let size = serialized_size(&value, MAX_PAYLOAD_BYTES.saturating_sub(self.used))
            .map_err(|_| StorageError::InvalidData)?;
        self.used = self
            .used
            .checked_add(size + 1)
            .filter(|used| *used <= MAX_PAYLOAD_BYTES)
            .ok_or(StorageError::InvalidData)?;
        Ok(value)
    }
}

// Conservative lower bounds: only bytes certainly present in portable output.
// All probes and row collection share this deferred read transaction/snapshot.
fn preflight(db: &Connection, profile: &str) -> Result<(), StorageError> {
    let mut bytes = 0_i64;
    let mut records = 0_i64;
    let maximum_bytes = i64::try_from(MAX_PAYLOAD_BYTES).map_err(|_| StorageError::InvalidData)?;
    for (table, expression, filter, maximum) in [
        ("resume_drafts", "length(document_json)", "", 1),
        ("published_resumes", "length(document_json)", "", 100_000),
        (
            "settings",
            "length(value_json) + length(CAST(setting_key AS BLOB))",
            "AND setting_key NOT IN ('ai.connection.v1','ai.chatgpt_plan.v1')",
            128,
        ),
        ("tracker_entries", "length(entry_json)", "", 100_000),
        (
            "render_manifests",
            "length(manifest_id) + length(document_sha256) + length(pdf_sha256) + length(renderer_version) + length(template_id) + length(template_sha256) + length(font_bundle_id) + length(font_bundle_sha256)",
            "",
            100,
        ),
        (
            "ai_operations",
            "length(operation_id) + length(operation_type) + length(status)",
            "",
            100_000,
        ),
        (
            "ai_attempts",
            "length(attempt_id) + length(operation_id) + length(provider) + length(credential_id) + length(requested_model) + coalesce(length(effective_model),0) + length(preset_version) + length(catalog_id) + length(catalog_effective_from) + length(pricing_components_json) + coalesce(length(usage_json),0) + coalesce(length(error_details_json),0)",
            "",
            100_000,
        ),
        (
            "ai_lifetime_totals",
            "length(credential_id) + length(currency)",
            "",
            ort_backup::MAX_AI_ACCOUNTING_RECORDS,
        ),
        (
            "ai_guardrail_policies",
            "length(credential_id) + length(currency) + length(time_zone)",
            "",
            ort_backup::MAX_AI_ACCOUNTING_RECORDS,
        ),
        (
            "ai_imported_guardrails",
            "length(policy_json)",
            "",
            ort_backup::MAX_AI_ACCOUNTING_RECORDS,
        ),
    ] {
        let sql = format!(
            "SELECT count(*), coalesce(sum({expression}),0) FROM {table} WHERE profile_id = ?1 {filter}"
        );
        let (count, size): (i64, i64) = db
            .query_row(&sql, [profile], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|_| StorageError::Unavailable)?;
        if count < 0
            || size < 0
            || count > i64::try_from(maximum).map_err(|_| StorageError::InvalidData)?
        {
            return Err(StorageError::InvalidData);
        }
        bytes = bytes
            .checked_add(size)
            .filter(|size| *size <= maximum_bytes)
            .ok_or(StorageError::InvalidData)?;
        records += count;
        if records > 100_000 {
            return Err(StorageError::InvalidData);
        }
    }
    Ok(())
}

impl EncryptedStore {
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
        create_backup(passphrase, self.portable_backup_request(app_version)?)
            .map_err(|error| map_backup_write_error(&error))
    }

    /// Reads a bounded, consistent snapshot for encryption outside the storage lock.
    /// # Errors
    /// Rejects oversized or invalid profiles before unbounded record materialization.
    pub fn portable_backup_request(
        &self,
        app_version: &str,
    ) -> Result<BackupExportRequestV1, StorageError> {
        Ok(BackupExportRequestV1 {
            app_version: app_version.to_owned(),
            created_at: now_string(),
            profile: self.read_portable_profile()?,
        })
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn read_portable_profile(&self) -> Result<PortableProfileV1, StorageError> {
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|_| StorageError::Unavailable)?;
        preflight(&transaction, &self.manifest.profile_id.to_string())?;
        let mut budget = CollectionBudget::default();
        let master_draft = transaction
            .query_row(
                "SELECT revision, document_json FROM resume_drafts WHERE profile_id = ?1",
                [self.manifest.profile_id.to_string()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .optional()
            .map_err(|_| StorageError::Unavailable)?
            .map(|(revision, json)| {
                budget.record(PortableResumeRevisionV1 {
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
                    budget.record(PortablePublishedResumeV1 {
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
                     WHERE profile_id = ?1 AND setting_key NOT IN ('ai.connection.v1', 'ai.chatgpt_plan.v1') ORDER BY setting_key",
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
                    budget.record((key, PortableSettingV1 { revision, value }))
                })
                .collect::<Result<BTreeMap<_, _>, StorageError>>()?
        };
        let mut settings = settings;
        settings.remove(AI_CONNECTION_SETTING_KEY);
        settings.remove("ai.chatgpt_plan.v1");
        let tracker_entries = {
            let mut statement = transaction.prepare(
                "SELECT entry_id, revision, entry_json FROM tracker_entries WHERE profile_id = ?1 ORDER BY entry_id",
            ).map_err(|_| StorageError::Unavailable)?;
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
                    let (id, revision, bytes) = row.map_err(|_| StorageError::Unavailable)?;
                    let value =
                        serde_json::from_slice(&bytes).map_err(|_| StorageError::InvalidData)?;
                    budget.record(PortableTrackerEntryV1 {
                        id,
                        revision,
                        value,
                    })
                })
                .collect::<Result<Vec<_>, StorageError>>()?
        };
        let render_manifests = read_portable_render_manifests(
            &transaction,
            &self.manifest.profile_id.to_string(),
            &mut budget,
        )?;
        let ai_operations = {
            let mut statement = transaction
                .prepare(
                    "SELECT operation_id, operation_type, started_at_unix_ms, ended_at_unix_ms,
                    status, cancelled FROM ai_operations WHERE profile_id = ?1
                    ORDER BY started_at_unix_ms, operation_id",
                )
                .map_err(|_| StorageError::Unavailable)?;
            statement
                .query_map([self.manifest.profile_id.to_string()], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, i64>(5)?,
                    ))
                })
                .map_err(|_| StorageError::Unavailable)?
                .map(|row| {
                    let (operation_id, operation_type, started, ended, status, cancelled) =
                        row.map_err(|_| StorageError::Unavailable)?;
                    budget.record(PortableAiOperationV1 {
                        operation_id,
                        operation_type,
                        started_at_unix_ms: u64::try_from(started)
                            .map_err(|_| StorageError::InvalidData)?,
                        ended_at_unix_ms: ended
                            .map(u64::try_from)
                            .transpose()
                            .map_err(|_| StorageError::InvalidData)?,
                        status,
                        cancelled: cancelled == 1,
                    })
                })
                .collect::<Result<Vec<_>, StorageError>>()?
        };
        let ai_attempts = read_portable_ai_attempts(
            &transaction,
            &self.manifest.profile_id.to_string(),
            &mut budget,
        )?;
        let ai_lifetime_totals =
            ai_ledger::read_totals(&transaction, &self.manifest.profile_id.to_string())?;
        let ai_guardrail_policies =
            ai_ledger::read_policies(&transaction, &self.manifest.profile_id.to_string())?;
        for value in &ai_lifetime_totals {
            budget.record(value)?;
        }
        for value in &ai_guardrail_policies {
            budget.record(value)?;
        }
        transaction
            .commit()
            .map_err(|_| StorageError::Unavailable)?;
        Ok(PortableProfileV1 {
            master_draft,
            published_resumes,
            settings,
            tracker_entries,
            render_manifests,
            ai_operations,
            ai_attempts,
            ai_lifetime_totals,
            ai_guardrail_policies,
        })
    }
}

#[allow(clippy::type_complexity, clippy::too_many_lines)]
fn read_portable_ai_attempts(
    transaction: &Transaction<'_>,
    profile_id: &str,
    budget: &mut CollectionBudget,
) -> Result<Vec<PortableAiAttemptV1>, StorageError> {
    let mut statement = transaction
        .prepare(
            "SELECT attempt_id, operation_id, provider, credential_id, requested_model,
            effective_model, preset_version, catalog_id, catalog_effective_from,
            pricing_components_json, started_at_unix_ms, ended_at_unix_ms,
            status, retry_of, usage_json, usage_complete, estimated_input_tokens,
            reserved_cost_micros, settled_cost_micros, currency, estimate_completeness,
            error_category, error_details_json, connection_source, reasoning, monetary_cost_tracking, reported_retries FROM ai_attempts WHERE profile_id = ?1
            ORDER BY started_at_unix_ms, attempt_id",
        )
        .map_err(|_| StorageError::Unavailable)?;
    statement
        .query_map([profile_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Vec<u8>>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, Option<i64>>(11)?,
                row.get::<_, String>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<Vec<u8>>>(14)?,
                row.get::<_, i64>(15)?,
                row.get::<_, i64>(16)?,
                row.get::<_, i64>(17)?,
                row.get::<_, Option<i64>>(18)?,
                row.get::<_, String>(19)?,
                row.get::<_, String>(20)?,
                row.get::<_, Option<String>>(21)?,
                row.get::<_, Option<Vec<u8>>>(22)?,
                row.get::<_, String>(23)?,
                row.get::<_, Option<String>>(24)?,
                row.get::<_, String>(25)?,
                row.get::<_, u32>(26)?,
            ))
        })
        .map_err(|_| StorageError::Unavailable)?
        .map(|row| {
            let (
                attempt_id,
                operation_id,
                provider,
                credential_id,
                requested_model,
                effective_model,
                preset_version,
                catalog_id,
                catalog_effective_from,
                pricing_components,
                started,
                ended,
                status,
                retry_of,
                usage,
                usage_complete,
                estimated_input_tokens,
                reserved_cost_micros,
                settled_cost_micros,
                currency,
                estimate_completeness,
                error_category,
                error_details,
                connection_source,
                reasoning,
                monetary_cost_tracking,
                reported_retries,
            ) = row.map_err(|_| StorageError::Unavailable)?;
            let pricing_components =
                serde_json::from_slice::<Vec<ort_ai::Price>>(&pricing_components)
                    .map_err(|_| StorageError::InvalidData)?
                    .into_iter()
                    .map(|price| PortableAiPriceV1 {
                        category: match price.category {
                            ort_ai::PriceCategory::Input => "input",
                            ort_ai::PriceCategory::CachedInput => "cached_input",
                            ort_ai::PriceCategory::CacheWrite => "cache_write",
                            ort_ai::PriceCategory::Output => "output",
                            ort_ai::PriceCategory::Reasoning => "reasoning",
                        }
                        .into(),
                        micros_per_million: price.micros_per_million,
                    })
                    .collect();
            budget.record(PortableAiAttemptV1 {
                connection_source,
                reasoning,
                monetary_cost_tracking,
                reported_retries,
                attempt_id,
                operation_id,
                provider,
                credential_id,
                requested_model,
                effective_model,
                preset_version,
                catalog_id,
                catalog_effective_from,
                pricing_components,
                started_at_unix_ms: u64::try_from(started)
                    .map_err(|_| StorageError::InvalidData)?,
                ended_at_unix_ms: ended
                    .map(u64::try_from)
                    .transpose()
                    .map_err(|_| StorageError::InvalidData)?,
                status,
                retry_of,
                usage: usage
                    .map(|bytes| serde_json::from_slice(&bytes))
                    .transpose()
                    .map_err(|_| StorageError::InvalidData)?,
                usage_complete: usage_complete == 1,
                estimated_input_tokens: u64::try_from(estimated_input_tokens)
                    .map_err(|_| StorageError::InvalidData)?,
                reserved_cost_micros: u64::try_from(reserved_cost_micros)
                    .map_err(|_| StorageError::InvalidData)?,
                settled_cost_micros: settled_cost_micros
                    .map(u64::try_from)
                    .transpose()
                    .map_err(|_| StorageError::InvalidData)?,
                currency,
                estimate_completeness,
                error_category,
                error_details: error_details
                    .map(|bytes| serde_json::from_slice(&bytes))
                    .transpose()
                    .map_err(|_| StorageError::InvalidData)?,
            })
        })
        .collect()
}

fn read_portable_render_manifests(
    transaction: &Transaction<'_>,
    profile_id: &str,
    budget: &mut CollectionBudget,
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
            budget.record(PortableRenderManifestV1 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ort_vault::testing::MemoryDatabaseKeyVault;

    #[test]
    fn aggregate_probe_rejects_history_before_any_record_is_materialized() {
        let temp = tempfile::tempdir().unwrap();
        let store =
            EncryptedStore::open_or_initialize(temp.path(), "test", &MemoryDatabaseKeyVault::new())
                .unwrap();
        let connection = store.connection.lock().unwrap();
        // SQLite creates the oversized synthetic blob internally. The aggregate
        // probe reads its stored length; no blob or JSON tree crosses into Rust.
        connection.execute("INSERT INTO settings(profile_id,setting_key,revision,value_json,updated_at) VALUES (?1,'synthetic',1,zeroblob(?2),?3)", params![store.manifest.profile_id.to_string(), i64::try_from(MAX_PAYLOAD_BYTES + 1).unwrap(), now_string()]).unwrap();
        assert_eq!(
            preflight(&connection, &store.manifest.profile_id.to_string()),
            Err(StorageError::InvalidData)
        );
        drop(connection);
        let phrase = BackupPassphrase::new("synthetic resource probe".into()).unwrap();
        assert_eq!(
            store.create_portable_backup(&phrase, "0.0.0-dev"),
            Err(StorageError::InvalidData)
        );
    }

    #[test]
    fn collection_budget_stops_before_retaining_a_record_over_budget() {
        let mut budget = CollectionBudget {
            used: MAX_PAYLOAD_BYTES - 6,
        };
        assert_eq!(budget.record("abc").unwrap(), "abc"); // Five JSON bytes plus separator.
        assert_eq!(budget.record("x"), Err(StorageError::InvalidData));
    }
}
