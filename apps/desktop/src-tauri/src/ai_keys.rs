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
    pub identification_number: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub provider: String,
    pub preset: String,
    pub paused: bool,
    pub removed: bool,
    pub cleanup_required: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AiKeyRegistry {
    pub keys: Vec<SavedAiKey>,
    pub primary_credential_id: Option<Uuid>,
    pub next_identification_number: u64,
}
impl Default for AiKeyRegistry {
    fn default() -> Self {
        Self {
            keys: vec![],
            primary_credential_id: None,
            next_identification_number: 1,
        }
    }
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
    fn from_value(value: serde_json::Value) -> Result<Self, StorageError> {
        let registry = if value.get("keys").is_some() {
            serde_json::from_value(value).map_err(|_| StorageError::InvalidData)?
        } else {
            let legacy: AiConnectionState =
                serde_json::from_value(value).map_err(|_| StorageError::InvalidData)?;
            if !matches!(legacy.mode.as_str(), "direct_api" | "no_ai") {
                return Err(StorageError::InvalidData);
            }
            match (legacy.credential_id, legacy.provider, legacy.preset) {
                (Some(id), Some(provider), Some(preset)) => Self {
                    keys: vec![SavedAiKey {
                        credential_id: id,
                        identification_number: 1,
                        name: None,
                        provider,
                        preset,
                        paused: legacy.mode != "direct_api",
                        removed: false,
                        cleanup_required: false,
                    }],
                    primary_credential_id: (legacy.mode == "direct_api").then_some(id),
                    next_identification_number: 2,
                },
                (None, None, None) => Self::default(),
                _ => return Err(StorageError::InvalidData),
            }
        };
        registry.validate()?;
        Ok(registry)
    }
    fn validate(&self) -> Result<(), StorageError> {
        let mut ids = std::collections::HashSet::new();
        let mut numbers = std::collections::HashSet::new();
        if self.next_identification_number == 0
            || self.next_identification_number > 9_007_199_254_740_991
        {
            return Err(StorageError::InvalidData);
        }
        for key in &self.keys {
            if !matches!(key.provider.as_str(), "openai" | "anthropic" | "gemini")
                || !matches!(key.preset.as_str(), "economy" | "balanced" | "quality")
                || key.identification_number == 0
                || key.identification_number >= self.next_identification_number
                || !ids.insert(key.credential_id)
                || !numbers.insert(key.identification_number)
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
        Some(saved) => Ok((
            AiKeyRegistry::from_value(saved.value)?,
            Some(saved.revision),
        )),
        None => Ok((AiKeyRegistry::default(), None)),
    }
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

fn add_key<V: ProviderCredentialVault>(
    store: &EncryptedStore,
    vault: &V,
    request: AddAiKeyRequest,
) -> Result<AiKeyRegistry, &'static str> {
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
    let number = registry.next_identification_number;
    registry.next_identification_number = number.checked_add(1).ok_or(STORAGE)?;
    let key = SavedAiKey {
        credential_id: Uuid::now_v7(),
        identification_number: number,
        name: None,
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
    if window.label() != "main" {
        return window_not_authorized();
    }
    let Some(_lease) = gate.begin(Uuid::now_v7()) else {
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

#[cfg(test)]
#[path = "ai_keys_tests.rs"]
mod tests;
