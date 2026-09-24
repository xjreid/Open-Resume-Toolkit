//! Non-secret key roster in the encrypted profile; key bytes remain in the OS vault.
//! The existing excluded-from-backups setting is reused for safe legacy migration.
use ort_domain::CommandResponse;
use ort_storage::{EncryptedStore, StorageError};
use ort_vault::{
    OsProviderCredentialVault, ProviderCredentialReference, ProviderCredentialVault, ProviderSecret,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{State, WebviewWindow};
use uuid::Uuid;

use crate::{DesktopState, ai_request::AiRequestGate, storage_unavailable, window_not_authorized};
const SETTING: &str = "ai.connection.v1";
const STORAGE: &str = "STORAGE_UNAVAILABLE";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedAiKey {
    pub credential_id: Uuid,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub provider: String,
    pub preset: String,
    pub paused: bool,
    pub removed: bool,
    pub cleanup_required: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiKeyRegistry {
    pub keys: Vec<SavedAiKey>,
    pub primary_credential_id: Option<Uuid>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AiConnectionState {
    pub mode: String,
    pub provider: Option<String>,
    pub preset: Option<String>,
    pub credential_id: Option<Uuid>,
}
impl AiKeyRegistry {
    #[cfg(test)]
    fn from_value(value: serde_json::Value) -> Result<Self, StorageError> {
        Self::from_value_with_migration(value).map(|(registry, _)| registry)
    }

    fn from_value_with_migration(
        mut value: serde_json::Value,
    ) -> Result<(Self, bool), StorageError> {
        let mut migrated = false;
        let registry = if value.get("keys").is_some() {
            let object = value.as_object_mut().ok_or(StorageError::InvalidData)?;
            migrated |= object.remove("nextIdentificationNumber").is_some();
            let keys = object
                .get_mut("keys")
                .and_then(serde_json::Value::as_array_mut)
                .ok_or(StorageError::InvalidData)?;
            for key in keys {
                let key = key.as_object_mut().ok_or(StorageError::InvalidData)?;
                migrated |= key.remove("identificationNumber").is_some();
                if !key.contains_key("createdAt") {
                    let id = key
                        .get("credentialId")
                        .and_then(serde_json::Value::as_str)
                        .and_then(|value| Uuid::parse_str(value).ok())
                        .ok_or(StorageError::InvalidData)?;
                    key.insert(
                        "createdAt".into(),
                        serde_json::Value::String(created_at_for(id)),
                    );
                    migrated = true;
                }
            }
            serde_json::from_value(value).map_err(|_| StorageError::InvalidData)?
        } else {
            migrated = true;
            let legacy: AiConnectionState =
                serde_json::from_value(value).map_err(|_| StorageError::InvalidData)?;
            if !matches!(legacy.mode.as_str(), "direct_api" | "no_ai") {
                return Err(StorageError::InvalidData);
            }
            match (legacy.credential_id, legacy.provider, legacy.preset) {
                (Some(id), Some(provider), Some(preset)) => Self {
                    keys: vec![SavedAiKey {
                        credential_id: id,
                        created_at: created_at_for(id),
                        name: None,
                        provider,
                        preset,
                        paused: legacy.mode != "direct_api",
                        removed: false,
                        cleanup_required: false,
                    }],
                    primary_credential_id: (legacy.mode == "direct_api").then_some(id),
                },
                (None, None, None) => Self::default(),
                _ => return Err(StorageError::InvalidData),
            }
        };
        registry.validate()?;
        Ok((registry, migrated))
    }
    fn validate(&self) -> Result<(), StorageError> {
        let mut ids = std::collections::HashSet::new();
        for key in &self.keys {
            if !matches!(key.provider.as_str(), "openai" | "anthropic" | "gemini")
                || !matches!(key.preset.as_str(), "economy" | "balanced" | "quality")
                || !ids.insert(key.credential_id)
                || key.created_at.parse::<jiff::Timestamp>().is_err()
                || key.name.as_ref().is_some_and(|name| !valid_key_name(name))
                || ((key.removed || key.cleanup_required) && !key.paused)
            {
                return Err(StorageError::InvalidData);
            }
        }
        if let Some(id) = self.primary_credential_id
            && !self.keys.iter().any(|key| {
                key.credential_id == id && !key.paused && !key.removed && !key.cleanup_required
            })
        {
            return Err(StorageError::InvalidData);
        }
        Ok(())
    }
    fn connection(&self, target: Option<Uuid>) -> Result<AiConnectionState, StorageError> {
        let id = target.or(self.primary_credential_id);
        let Some(id) = id else {
            return Ok(AiConnectionState {
                mode: "no_ai".into(),
                provider: None,
                preset: None,
                credential_id: None,
            });
        };
        let key = self
            .keys
            .iter()
            .find(|key| key.credential_id == id && !key.removed)
            .ok_or(StorageError::NotFound)?;
        Ok(AiConnectionState {
            mode: if key.paused || key.cleanup_required {
                "no_ai"
            } else {
                "direct_api"
            }
            .into(),
            provider: Some(key.provider.clone()),
            preset: Some(key.preset.clone()),
            credential_id: Some(id),
        })
    }
    fn pause(&mut self, id: Uuid, paused: bool) -> Result<(), &'static str> {
        let key = self
            .keys
            .iter_mut()
            .find(|key| key.credential_id == id && !key.removed)
            .ok_or("AI_KEY_NOT_FOUND")?;
        if key.cleanup_required && !paused {
            return Err("AI_CREDENTIAL_CLEANUP_REQUIRED");
        }
        key.paused = paused;
        if paused && self.primary_credential_id == Some(id) {
            self.primary_credential_id = None;
        }
        Ok(())
    }
    fn select(&mut self, id: Uuid) -> Result<(), &'static str> {
        if !self.keys.iter().any(|key| {
            key.credential_id == id && !key.paused && !key.removed && !key.cleanup_required
        }) {
            return Err("AI_KEY_PAUSED");
        }
        self.primary_credential_id = Some(id);
        Ok(())
    }
}
pub(crate) fn load_registry(
    store: &EncryptedStore,
) -> Result<(AiKeyRegistry, Option<i64>), StorageError> {
    match store.load_setting(SETTING)? {
        Some(saved) => {
            let (registry, migrated) = AiKeyRegistry::from_value_with_migration(saved.value)?;
            if migrated {
                let revision = store
                    .save_setting(SETTING, Some(saved.revision), &json!(registry))?
                    .revision;
                Ok((registry, Some(revision)))
            } else {
                Ok((registry, Some(saved.revision)))
            }
        }
        None => Ok((AiKeyRegistry::default(), None)),
    }
}

fn created_at_for(id: Uuid) -> String {
    id.get_timestamp()
        .and_then(|timestamp| {
            let (seconds, nanos) = timestamp.to_unix();
            jiff::Timestamp::new(i64::try_from(seconds).ok()?, i32::try_from(nanos).ok()?).ok()
        })
        .unwrap_or_else(jiff::Timestamp::now)
        .to_string()
}
fn save_registry(
    store: &EncryptedStore,
    registry: &AiKeyRegistry,
    revision: Option<i64>,
) -> Result<i64, &'static str> {
    registry.validate().map_err(|_| STORAGE)?;
    store
        .save_setting(SETTING, revision, &json!(registry))
        .map(|saved| saved.revision)
        .map_err(|_| STORAGE)
}
pub(crate) fn request_connection(
    store: &EncryptedStore,
    target: Option<Uuid>,
) -> Result<AiConnectionState, StorageError> {
    load_registry(store)?.0.connection(target)
}
// An explicit, user-confirmed credential test can exercise a paused key without
// enabling it for ordinary requests or changing the primary selection.
pub(crate) fn test_connection(
    store: &EncryptedStore,
    target: Uuid,
) -> Result<AiConnectionState, StorageError> {
    let registry = load_registry(store)?.0;
    let key = registry
        .keys
        .iter()
        .find(|key| key.credential_id == target && !key.removed)
        .ok_or(StorageError::NotFound)?;
    let mut connection = registry.connection(Some(target))?;
    if !key.cleanup_required {
        connection.mode = "direct_api".into();
    }
    Ok(connection)
}
fn reference(
    store: &EncryptedStore,
    key: &SavedAiKey,
) -> Result<ProviderCredentialReference, StorageError> {
    let (channel, install, profile) = store.vault_identity();
    ProviderCredentialReference::new(
        channel,
        &install.to_string(),
        &profile.to_string(),
        &key.provider,
        &key.credential_id.to_string(),
    )
    .map_err(|_| StorageError::InvalidData)
}
pub(crate) fn saved_credential_references(
    store: &EncryptedStore,
) -> Result<Vec<ProviderCredentialReference>, StorageError> {
    load_registry(store)?
        .0
        .keys
        .iter()
        .filter(|key| !key.removed)
        .map(|key| reference(store, key))
        .collect()
}
pub(crate) fn suspend_for_deletion(store: &EncryptedStore) -> Result<(), StorageError> {
    let (mut registry, revision) = load_registry(store)?;
    for key in &mut registry.keys {
        key.paused = true;
    }
    registry.primary_credential_id = None;
    save_registry(store, &registry, revision)
        .map(|_| ())
        .map_err(|_| StorageError::Unavailable)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddAiKeyRequest {
    provider: String,
    api_key: String,
    #[serde(default)]
    name: Option<String>,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiKeyAction {
    SelectPrimary,
    Pause,
    Unpause,
    Remove,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChangeAiKeyRequest {
    credential_id: Uuid,
    action: AiKeyAction,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteRemovedKeyDataRequest {
    credential_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteRemovedKeyDataResult {
    registry: AiKeyRegistry,
    cleared_operations: u64,
}

fn add_key<V: ProviderCredentialVault>(
    store: &EncryptedStore,
    vault: &V,
    request: AddAiKeyRequest,
) -> Result<AiKeyRegistry, &'static str> {
    let name = request
        .name
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty());
    if name.as_ref().is_some_and(|name| !valid_key_name(name)) {
        return Err("AI_KEY_NAME_INVALID");
    }
    let provider = match request.provider.as_str() {
        "openai" => ort_ai::Provider::OpenAi,
        "anthropic" => ort_ai::Provider::Anthropic,
        "gemini" => ort_ai::Provider::Gemini,
        _ => return Err("AI_CONFIGURATION_INVALID"),
    };
    let catalog = ort_ai::builtin_catalog(&jiff::Timestamp::now().to_string(), None)
        .map_err(|_| "AI_CATALOG_UNAVAILABLE")?;
    catalog
        .resolve(
            provider,
            ort_ai::Preset::Balanced,
            ort_ai::OperationType::CredentialTest,
        )
        .map_err(|_| "AI_PRESET_UNAVAILABLE")?;
    let secret = ProviderSecret::from_bytes(request.api_key.into_bytes())
        .map_err(|_| "AI_CREDENTIAL_INVALID")?;
    let (mut registry, revision) = load_registry(store).map_err(|_| STORAGE)?;
    let credential_id = Uuid::now_v7();
    let key = SavedAiKey {
        credential_id,
        created_at: created_at_for(credential_id),
        name,
        provider: request.provider,
        preset: "balanced".into(),
        paused: true,
        removed: false,
        cleanup_required: true,
    };
    let reference = reference(store, &key).map_err(|_| STORAGE)?;
    registry.keys.push(key);
    // Stage the address first: even failed/crashed vault writes remain discoverable and removable.
    let revision = save_registry(store, &registry, revision)?;
    vault
        .store_new(&reference, &secret)
        .map_err(|_| "AI_CREDENTIAL_CLEANUP_REQUIRED")?;
    let key = registry.keys.last_mut().ok_or(STORAGE)?;
    key.paused = false;
    key.cleanup_required = false;
    save_registry(store, &registry, Some(revision))
        .map_err(|_| "AI_CREDENTIAL_CLEANUP_REQUIRED")?;
    Ok(registry)
}
fn change_key<V: ProviderCredentialVault>(
    store: &EncryptedStore,
    vault: &V,
    request: ChangeAiKeyRequest,
) -> Result<AiKeyRegistry, &'static str> {
    let (mut registry, revision) = load_registry(store).map_err(|_| STORAGE)?;
    let id = request.credential_id;
    let key = registry
        .keys
        .iter()
        .find(|key| key.credential_id == id && !key.removed)
        .ok_or("AI_KEY_NOT_FOUND")?;
    let reference = reference(store, key).map_err(|_| STORAGE)?;
    match request.action {
        AiKeyAction::SelectPrimary => {
            vault
                .use_secret(&reference, |_| ())
                .map_err(|_| "AI_CREDENTIAL_MISSING")?;
            registry.select(id)?;
        }
        AiKeyAction::Pause => registry.pause(id, true)?,
        AiKeyAction::Unpause => {
            vault
                .use_secret(&reference, |_| ())
                .map_err(|_| "AI_CREDENTIAL_MISSING")?;
            registry.pause(id, false)?;
        }
        AiKeyAction::Remove => {
            registry.pause(id, true)?;
            registry
                .keys
                .iter_mut()
                .find(|key| key.credential_id == id)
                .ok_or(STORAGE)?
                .cleanup_required = true;
            // Fail closed before deletion. A failed cleanup is retryable and cannot remain primary.
            let revision = save_registry(store, &registry, revision)?;
            vault
                .delete(&reference)
                .map_err(|_| "AI_CREDENTIAL_CLEANUP_REQUIRED")?;
            store
                .remove_ai_cap_identity(id)
                .map_err(|_| "AI_CREDENTIAL_CLEANUP_REQUIRED")?;
            let key = registry
                .keys
                .iter_mut()
                .find(|key| key.credential_id == id)
                .ok_or(STORAGE)?;
            key.removed = true;
            key.cleanup_required = false;
            save_registry(store, &registry, Some(revision))
                .map_err(|_| "AI_CREDENTIAL_CLEANUP_REQUIRED")?;
            return Ok(registry);
        }
    }
    save_registry(store, &registry, revision)?;
    Ok(registry)
}

fn clear_primary(store: &EncryptedStore) -> Result<AiKeyRegistry, &'static str> {
    let (mut registry, revision) = load_registry(store).map_err(|_| STORAGE)?;
    if registry.primary_credential_id.take().is_some() {
        save_registry(store, &registry, revision)?;
    }
    Ok(registry)
}

fn delete_removed_key_data(
    store: &EncryptedStore,
    request: &DeleteRemovedKeyDataRequest,
) -> Result<DeleteRemovedKeyDataResult, &'static str> {
    if request.credential_ids.is_empty() || request.credential_ids.len() > 1_000 {
        return Err("AI_KEY_SELECTION_INVALID");
    }
    let selected = request
        .credential_ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    if selected.len() != request.credential_ids.len() {
        return Err("AI_KEY_SELECTION_INVALID");
    }
    let (mut registry, revision) = load_registry(store).map_err(|_| STORAGE)?;
    if selected.iter().any(|id| {
        !registry
            .keys
            .iter()
            .any(|key| key.credential_id == *id && key.removed && !key.cleanup_required)
    }) {
        return Err("AI_KEY_NOT_REMOVED");
    }
    let cleared_operations = store
        .clear_all_ai_activity_for_keys(&request.credential_ids)
        .map_err(|error| match error {
            StorageError::RevisionConflict => "AI_BUSY",
            _ => STORAGE,
        })?;
    registry
        .keys
        .retain(|key| !selected.contains(&key.credential_id));
    save_registry(store, &registry, revision)?;
    Ok(DeleteRemovedKeyDataResult {
        registry,
        cleared_operations,
    })
}
fn response(
    result: Result<Result<AiKeyRegistry, &'static str>, StorageError>,
) -> CommandResponse<AiKeyRegistry> {
    match result {
        Ok(Ok(value)) => CommandResponse::success(value),
        Ok(Err(code)) => CommandResponse::failure(code, "errors.aiKey", false),
        Err(_) => storage_unavailable(),
    }
}
fn valid_key_name(name: &str) -> bool {
    !name.is_empty()
        && name == name.trim()
        && name.chars().count() <= 80
        && !name.chars().any(char::is_control)
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameAiKeyRequest {
    credential_id: Uuid,
    name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetAiKeyPresetRequest {
    credential_id: Uuid,
    preset: String,
}
fn set_key_preset(
    store: &EncryptedStore,
    request: &SetAiKeyPresetRequest,
) -> Result<AiKeyRegistry, &'static str> {
    let (mut registry, revision) = load_registry(store).map_err(|_| STORAGE)?;
    let key = registry
        .keys
        .iter_mut()
        .find(|key| {
            key.credential_id == request.credential_id && !key.removed && !key.cleanup_required
        })
        .ok_or("AI_KEY_NOT_FOUND")?;
    let provider = match key.provider.as_str() {
        "openai" => ort_ai::Provider::OpenAi,
        "anthropic" => ort_ai::Provider::Anthropic,
        "gemini" => ort_ai::Provider::Gemini,
        _ => return Err("AI_PROVIDER_INVALID"),
    };
    let preset = match request.preset.as_str() {
        "economy" => ort_ai::Preset::Economy,
        "balanced" => ort_ai::Preset::Balanced,
        "quality" => ort_ai::Preset::Quality,
        _ => return Err("AI_PRESET_UNAVAILABLE"),
    };
    let catalog = ort_ai::builtin_catalog(&jiff::Timestamp::now().to_string(), None)
        .map_err(|_| "AI_CATALOG_UNAVAILABLE")?;
    catalog
        .resolve(provider, preset, ort_ai::OperationType::CredentialTest)
        .map_err(|_| "AI_PRESET_UNAVAILABLE")?;
    key.preset.clone_from(&request.preset);
    save_registry(store, &registry, revision)?;
    Ok(registry)
}
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn set_ai_key_preset(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    gate: State<'_, AiRequestGate>,
    request: SetAiKeyPresetRequest,
) -> CommandResponse<AiKeyRegistry> {
    if !matches!(window.label(), "main" | "overlay") {
        return window_not_authorized();
    }
    let lease = if window.label() == "overlay" {
        gate.begin_overlay(Uuid::now_v7())
    } else {
        gate.begin(Uuid::now_v7())
    };
    let Some(_lease) = lease else {
        return CommandResponse::failure("AI_BUSY", "errors.aiBusy", true);
    };
    response(state.with_store(|store| Ok(set_key_preset(store, &request))))
}

fn rename_key(
    store: &EncryptedStore,
    request: &RenameAiKeyRequest,
) -> Result<AiKeyRegistry, &'static str> {
    let name = request.name.trim();
    if !name.is_empty() && !valid_key_name(name) {
        return Err("AI_KEY_NAME_INVALID");
    }
    let (mut registry, revision) = load_registry(store).map_err(|_| STORAGE)?;
    let key = registry
        .keys
        .iter_mut()
        .find(|key| key.credential_id == request.credential_id && !key.removed)
        .ok_or("AI_KEY_NOT_FOUND")?;
    key.name = (!name.is_empty()).then(|| name.to_owned());
    save_registry(store, &registry, revision)?;
    Ok(registry)
}
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn rename_ai_key(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    gate: State<'_, AiRequestGate>,
    request: RenameAiKeyRequest,
) -> CommandResponse<AiKeyRegistry> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Some(_lease) = gate.begin(Uuid::now_v7()) else {
        return CommandResponse::failure("AI_BUSY", "errors.aiBusy", true);
    };
    response(state.with_store(|store| Ok(rename_key(store, &request))))
}
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn load_ai_connection(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
) -> CommandResponse<AiKeyRegistry> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    match state.with_store(|store| {
        let (registry, _) = load_registry(store)?;
        for key in registry
            .keys
            .iter()
            .filter(|key| !key.removed && !key.cleanup_required)
        {
            crate::ai_settings::unified_cap(store, key.credential_id)?;
        }
        Ok(registry)
    }) {
        Ok(registry) => CommandResponse::success(registry),
        Err(_) => storage_unavailable(),
    }
}
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn add_ai_key(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    gate: State<'_, AiRequestGate>,
    request: AddAiKeyRequest,
) -> CommandResponse<AiKeyRegistry> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Some(_lease) = gate.begin(Uuid::now_v7()) else {
        return CommandResponse::failure("AI_BUSY", "errors.aiBusy", true);
    };
    response(
        state.with_store(|store| Ok(add_key(store, &OsProviderCredentialVault::new(), request))),
    )
}
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn change_ai_key(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    gate: State<'_, AiRequestGate>,
    request: ChangeAiKeyRequest,
) -> CommandResponse<AiKeyRegistry> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Some(_lease) = gate.begin(Uuid::now_v7()) else {
        return CommandResponse::failure("AI_BUSY", "errors.aiBusy", true);
    };
    response(state.with_store(|store| {
        Ok(change_key(
            store,
            &OsProviderCredentialVault::new(),
            request,
        ))
    }))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn clear_ai_primary(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    gate: State<'_, AiRequestGate>,
) -> CommandResponse<AiKeyRegistry> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Some(_lease) = gate.begin(Uuid::now_v7()) else {
        return CommandResponse::failure("AI_BUSY", "errors.aiBusy", true);
    };
    response(state.with_store(|store| Ok(clear_primary(store))))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn delete_removed_ai_key_data(
    window: WebviewWindow,
    state: State<'_, DesktopState>,
    gate: State<'_, AiRequestGate>,
    request: DeleteRemovedKeyDataRequest,
) -> CommandResponse<DeleteRemovedKeyDataResult> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Some(_lease) = gate.begin(Uuid::now_v7()) else {
        return CommandResponse::failure("AI_BUSY", "errors.aiBusy", true);
    };
    match state.with_store(|store| Ok(delete_removed_key_data(store, &request))) {
        Ok(Ok(value)) => CommandResponse::success(value),
        Ok(Err("AI_BUSY")) => CommandResponse::failure("AI_BUSY", "errors.aiBusy", true),
        Ok(Err(STORAGE)) | Err(_) => storage_unavailable(),
        Ok(Err(code)) => CommandResponse::failure(code, "errors.aiKey", false),
    }
}

#[cfg(test)]
#[path = "ai_keys_tests.rs"]
mod tests;
