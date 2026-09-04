//! One bounded, memory-only preview. The webview cannot submit PDFs or paths.
use super::{
    DesktopState, storage_failure,
    text_export::{ExportState, exact_revision},
    window_not_authorized,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use ort_backup::{
    BackupError, BackupPassphrase, PortableProfileV1, PortableRenderManifestV1, restore_backup,
};
use ort_domain::{
    CommandResponse, ExportSource, MAX_PDF_RENDER_HISTORY, OpenPortablePdfHistoryRequest,
    PDF_PREVIEW_TTL_SECONDS, PdfExportResponse, PdfPreviewResponse, PdfReleaseResponse,
    PdfRenderHistoryRequest, PdfRenderHistoryResponse, PdfRenderManifest, PdfReplayRequest,
    PdfReplayResponse, PdfTicketRequest, PortablePdfArchiveReleaseResponse,
    PortablePdfArchiveRequest, PortablePdfHistoryResponse, PortablePdfReplayRequest,
    RenderPdfRequest,
};
use ort_platform::{
    ExportDestination, ExportFileType, ExportWriteError, NativeInputError, read_native_backup,
};
use ort_render::{PdfArtifact, PdfRenderError};
use ort_storage::{StorageError, StoredRenderManifest, VersionedResume};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FilePath};

struct Preview {
    id: String,
    source: ExportSource,
    revision: i64,
    created: Instant,
    generated_at_unix_ms: u64,
    artifact: PdfArtifact,
    origin: PreviewOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewOrigin {
    ActiveProfile,
    PortableArchive,
}

#[derive(Default)]
pub(crate) struct PdfState(Mutex<Option<Arc<Preview>>>);

#[derive(Clone)]
struct ArchivedRenderSource {
    manifest: PdfRenderManifest,
    saved: VersionedResume,
}

struct PortableArchive {
    id: String,
    created: Instant,
    expires_at_unix_ms: u64,
    total_manifests: u16,
    unavailable_sources: u16,
    incompatible_receipts: u16,
    entries: Vec<ArchivedRenderSource>,
}

#[derive(Default)]
pub(crate) struct PortablePdfState(Mutex<Option<Arc<PortableArchive>>>);

impl PortablePdfState {
    fn get(&self, id: &str, now: Instant) -> Option<Arc<PortableArchive>> {
        let mut slot = self.0.lock().ok()?;
        if slot.as_ref().is_some_and(|archive| {
            now.saturating_duration_since(archive.created)
                >= Duration::from_secs(PDF_PREVIEW_TTL_SECONDS)
        }) {
            *slot = None;
        }
        slot.as_ref().filter(|archive| archive.id == id).cloned()
    }

    fn replace(&self, archive: Arc<PortableArchive>) -> bool {
        let Ok(mut slot) = self.0.lock() else {
            return false;
        };
        *slot = Some(archive);
        true
    }

    fn clear(&self) -> bool {
        let Ok(mut slot) = self.0.lock() else {
            return false;
        };
        *slot = None;
        true
    }

    fn release(&self, id: &str) -> bool {
        let Ok(mut slot) = self.0.lock() else {
            return false;
        };
        if slot.as_ref().is_some_and(|archive| archive.id == id) {
            *slot = None;
            true
        } else {
            false
        }
    }
}

impl PdfState {
    fn get(&self, id: &str, now: Instant) -> Option<Arc<Preview>> {
        let mut slot = self.0.lock().ok()?;
        if slot.as_ref().is_some_and(|p| {
            now.saturating_duration_since(p.created) >= Duration::from_secs(PDF_PREVIEW_TTL_SECONDS)
        }) {
            *slot = None;
        }
        slot.as_ref().filter(|p| p.id == id).cloned()
    }

    fn release(&self, id: &str) -> bool {
        let Ok(mut slot) = self.0.lock() else {
            return false;
        };
        if slot.as_ref().is_some_and(|p| p.id == id) {
            *slot = None;
            true
        } else {
            false
        }
    }

    pub(crate) fn clear(&self) -> bool {
        let Ok(mut slot) = self.0.lock() else {
            return false;
        };
        *slot = None;
        true
    }
}

fn load_current_exact(
    state: &DesktopState,
    source: ExportSource,
    revision: i64,
) -> Result<VersionedResume, StorageError> {
    state.with_store(|store| {
        let value = match source {
            ExportSource::SavedDraft => store.load_draft(),
            ExportSource::PublishedSnapshot => store.load_latest_published(),
        }?;
        exact_revision(value, revision)
    })
}

fn load_retained_exact(
    state: &DesktopState,
    source: ExportSource,
    revision: i64,
) -> Result<VersionedResume, StorageError> {
    state.with_store(|store| match source {
        ExportSource::SavedDraft => exact_revision(store.load_draft()?, revision),
        ExportSource::PublishedSnapshot => store
            .load_published_revision(revision)?
            .ok_or(StorageError::NotFound),
    })
}

fn load_replay_source(
    state: &DesktopState,
    manifest_id: uuid::Uuid,
) -> Result<(StoredRenderManifest, VersionedResume), StorageError> {
    state.with_store(|store| {
        let manifest = store
            .load_render_manifest(manifest_id)?
            .ok_or(StorageError::NotFound)?;
        let saved = match manifest.source {
            ExportSource::SavedDraft => {
                exact_revision(store.load_draft()?, manifest.source_revision).map_err(|error| {
                    match error {
                        StorageError::RevisionConflict => StorageError::NotFound,
                        other => other,
                    }
                })?
            }
            ExportSource::PublishedSnapshot => store
                .load_published_revision(manifest.source_revision)?
                .ok_or(StorageError::NotFound)?,
        };
        Ok((manifest, saved))
    })
}

fn portable_source(
    profile: &PortableProfileV1,
    manifest: &PortableRenderManifestV1,
) -> Option<VersionedResume> {
    match manifest.source {
        ExportSource::SavedDraft => profile
            .master_draft
            .as_ref()
            .filter(|draft| draft.revision == manifest.source_revision)
            .map(|draft| VersionedResume {
                revision: draft.revision,
                document: draft.document.clone(),
            }),
        ExportSource::PublishedSnapshot => profile
            .published_resumes
            .iter()
            .find(|published| published.published_revision == manifest.source_revision)
            .map(|published| VersionedResume {
                revision: published.published_revision,
                document: published.document.clone(),
            }),
    }
}

fn portable_manifest_response(value: &PortableRenderManifestV1) -> PdfRenderManifest {
    PdfRenderManifest {
        manifest_id: value.manifest_id.clone(),
        source: value.source,
        source_revision: value.source_revision,
        generated_at_unix_ms: value.generated_at_unix_ms,
        last_generated_at_unix_ms: value.last_generated_at_unix_ms,
        render_count: value.render_count,
        receipt: value.receipt.clone(),
    }
}

fn build_portable_archive(
    profile: &PortableProfileV1,
    created: Instant,
    opened_at_unix_ms: u64,
) -> Option<Arc<PortableArchive>> {
    let total_manifests = u16::try_from(profile.render_manifests.len()).ok()?;
    let mut unavailable_sources = 0_u16;
    let mut incompatible_receipts = 0_u16;
    let mut entries = Vec::new();
    for manifest in &profile.render_manifests {
        let Some(saved) = portable_source(profile, manifest) else {
            unavailable_sources = unavailable_sources.checked_add(1)?;
            continue;
        };
        if manifest.receipt.renderer_version != ort_render::RENDERER_VERSION
            || manifest.receipt.template_id != ort_render::TEMPLATE_ID
            || manifest.receipt.font_bundle_id != ort_render::FONT_BUNDLE_ID
        {
            incompatible_receipts = incompatible_receipts.checked_add(1)?;
            continue;
        }
        if entries.len() < usize::from(MAX_PDF_RENDER_HISTORY) {
            entries.push(ArchivedRenderSource {
                manifest: portable_manifest_response(manifest),
                saved,
            });
        }
    }
    let expires_at_unix_ms = opened_at_unix_ms
        .checked_add(PDF_PREVIEW_TTL_SECONDS.checked_mul(1_000)?)?
        .min(9_007_199_254_740_991);
    Some(Arc::new(PortableArchive {
        id: uuid::Uuid::now_v7().to_string(),
        created,
        expires_at_unix_ms,
        total_manifests,
        unavailable_sources,
        incompatible_receipts,
        entries,
    }))
}

fn authenticate_portable_archive(
    bytes: &[u8],
    passphrase: &BackupPassphrase,
    created: Instant,
    opened_at_unix_ms: u64,
) -> Result<Arc<PortableArchive>, BackupError> {
    let backup = restore_backup(bytes, passphrase)?;
    build_portable_archive(&backup.profile, created, opened_at_unix_ms)
        .ok_or(BackupError::InvalidBackup)
}

fn unix_time_millis() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|value| u64::try_from(value.as_millis()).ok())
        .filter(|value| (1..=9_007_199_254_740_991).contains(value))
}

#[tauri::command]
pub(crate) async fn open_portable_pdf_render_history(
    window: WebviewWindow,
    request: OpenPortablePdfHistoryRequest,
) -> CommandResponse<PortablePdfHistoryResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let app = window.app_handle().clone();
    let Some(lease) = app.state::<ExportState>().begin() else {
        return portable_history_failure("BACKUP_BUSY");
    };
    if !app.state::<PortablePdfState>().clear() {
        return portable_history_failure("PDF_UNAVAILABLE");
    }
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let Ok(passphrase) = BackupPassphrase::new(request.payload.passphrase) else {
            return portable_history_failure("INVALID_BACKUP_PASSPHRASE");
        };
        let selection = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Open encrypted backup render history — read only")
            .add_filter("Open Resume Toolkit backup", &["ort-backup"])
            .blocking_pick_file();
        let Some(selection) = selection else {
            return CommandResponse::success(PortablePdfHistoryResponse::Cancelled);
        };
        let FilePath::Path(path) = selection else {
            return portable_history_failure("BACKUP_INVALID_OR_PASSPHRASE");
        };
        let bytes = match read_native_backup(&path) {
            Ok(value) => value,
            Err(NativeInputError::Unavailable) => {
                return portable_history_failure("BACKUP_READ_UNAVAILABLE");
            }
            Err(NativeInputError::InvalidSelection | NativeInputError::InvalidContent) => {
                return portable_history_failure("BACKUP_INVALID_OR_PASSPHRASE");
            }
        };
        let Some(opened_at_unix_ms) = unix_time_millis() else {
            return portable_history_failure("PDF_UNAVAILABLE");
        };
        let Ok(archive) =
            authenticate_portable_archive(&bytes, &passphrase, Instant::now(), opened_at_unix_ms)
        else {
            return portable_history_failure("BACKUP_INVALID_OR_PASSPHRASE");
        };
        let response = PortablePdfHistoryResponse::Opened {
            archive_id: archive.id.clone(),
            expires_at_unix_ms: archive.expires_at_unix_ms,
            total_manifests: archive.total_manifests,
            unavailable_sources: archive.unavailable_sources,
            incompatible_receipts: archive.incompatible_receipts,
            manifests: archive
                .entries
                .iter()
                .map(|entry| entry.manifest.clone())
                .collect(),
        };
        if !app.state::<PortablePdfState>().replace(archive) {
            return portable_history_failure("PDF_UNAVAILABLE");
        }
        CommandResponse::success(response)
    })
    .await
    {
        Ok(response) => response,
        Err(_) => portable_history_failure("PDF_UNAVAILABLE"),
    }
}

#[tauri::command]
pub(crate) async fn render_resume_pdf(
    window: WebviewWindow,
    request: RenderPdfRequest,
) -> CommandResponse<PdfPreviewResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let app = window.app_handle().clone();
    let Some(lease) = app.state::<ExportState>().begin() else {
        return failure("EXPORT_BUSY");
    };
    let state = app.state::<DesktopState>();
    let source = request.payload.source;
    let saved = match load_current_exact(&state, source, request.payload.expected_revision) {
        Ok(saved) => saved,
        Err(error) => return storage_failure(&error),
    };
    // Drop the previous native bytes before compiling another bounded preview.
    {
        let cache = app.state::<PdfState>();
        let Ok(mut slot) = cache.0.lock() else {
            return failure("PDF_UNAVAILABLE");
        };
        *slot = None;
    }
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let artifact = match ort_render::render_pdf(&saved.document) {
            Ok(value) => value,
            Err(error) => {
                return failure(match error {
                    PdfRenderError::UnsupportedGlyph => "PDF_UNSUPPORTED_GLYPH",
                    PdfRenderError::LayoutLimit => "PDF_LAYOUT_LIMIT",
                    PdfRenderError::OutputTooLarge => "PDF_BYTE_LIMIT",
                    PdfRenderError::InvalidContent => "PDF_INVALID_CONTENT",
                    PdfRenderError::Unavailable => "PDF_UNAVAILABLE",
                });
            }
        };
        let Some(generated_at_unix_ms) = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|v| u64::try_from(v.as_millis()).ok())
            .filter(|value| (1..=9_007_199_254_740_991).contains(value))
        else {
            return failure("PDF_UNAVAILABLE");
        };
        let preview = Arc::new(Preview {
            id: uuid::Uuid::now_v7().to_string(),
            source,
            revision: saved.revision,
            created: Instant::now(),
            generated_at_unix_ms,
            artifact,
            origin: PreviewOrigin::ActiveProfile,
        });
        let state = app.state::<DesktopState>();
        if let Err(error) = state.with_store(|store| {
            store.record_render_manifest(
                source,
                saved.revision,
                generated_at_unix_ms,
                &preview.artifact.receipt,
            )
        }) {
            return storage_failure(&error);
        }
        let response = PdfPreviewResponse {
            render_id: preview.id.clone(),
            source,
            revision: saved.revision,
            generated_at_unix_ms: preview.generated_at_unix_ms,
            receipt: preview.artifact.receipt.clone(),
            pdf_base64: STANDARD.encode(&preview.artifact.bytes),
        };
        let cache = app.state::<PdfState>();
        let Ok(mut slot) = cache.0.lock() else {
            return failure("PDF_UNAVAILABLE");
        };
        *slot = Some(preview);
        CommandResponse::success(response)
    })
    .await
    {
        Ok(response) => response,
        Err(_) => failure("PDF_UNAVAILABLE"),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn load_pdf_render_history(
    window: WebviewWindow,
    request: PdfRenderHistoryRequest,
) -> CommandResponse<PdfRenderHistoryResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let state = window.state::<DesktopState>();
    match state.with_store(|store| store.load_recent_render_manifests(MAX_PDF_RENDER_HISTORY)) {
        Ok(manifests) => CommandResponse::success(PdfRenderHistoryResponse {
            manifests: manifests
                .into_iter()
                .map(render_manifest_response)
                .collect(),
        }),
        Err(error) => storage_failure(&error),
    }
}

fn render_manifest_response(value: StoredRenderManifest) -> PdfRenderManifest {
    PdfRenderManifest {
        manifest_id: value.manifest_id.to_string(),
        source: value.source,
        source_revision: value.source_revision,
        generated_at_unix_ms: value.generated_at_unix_ms,
        last_generated_at_unix_ms: value.last_generated_at_unix_ms,
        render_count: value.render_count,
        receipt: value.receipt,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplayRenderError {
    Render(PdfRenderError),
    Incompatible,
}

fn render_matching_receipt(
    document: &ort_domain::ResumeDocument,
    expected: &ort_domain::PdfRenderReceipt,
) -> Result<PdfArtifact, ReplayRenderError> {
    let artifact = ort_render::render_pdf(document).map_err(ReplayRenderError::Render)?;
    if artifact.receipt == *expected {
        Ok(artifact)
    } else {
        Err(ReplayRenderError::Incompatible)
    }
}

#[tauri::command]
pub(crate) async fn replay_resume_pdf(
    window: WebviewWindow,
    request: PdfReplayRequest,
) -> CommandResponse<PdfReplayResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let app = window.app_handle().clone();
    let Some(lease) = app.state::<ExportState>().begin() else {
        return failure("EXPORT_BUSY");
    };
    let Ok(manifest_id) = uuid::Uuid::parse_str(&request.payload.manifest_id) else {
        return failure("PDF_REPLAY_UNAVAILABLE");
    };
    let replay = load_replay_source(&app.state::<DesktopState>(), manifest_id);
    let (manifest, saved) = match replay {
        Ok(value) => value,
        Err(StorageError::NotFound) => return failure("PDF_REPLAY_SOURCE_UNAVAILABLE"),
        Err(error) => return storage_failure(&error),
    };
    if !app.state::<PdfState>().clear() {
        return failure("PDF_UNAVAILABLE");
    }
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let artifact = match render_matching_receipt(&saved.document, &manifest.receipt) {
            Ok(value) => value,
            Err(ReplayRenderError::Incompatible) => return failure("PDF_REPLAY_INCOMPATIBLE"),
            Err(ReplayRenderError::Render(error)) => {
                return failure(match error {
                    PdfRenderError::UnsupportedGlyph => "PDF_UNSUPPORTED_GLYPH",
                    PdfRenderError::LayoutLimit => "PDF_LAYOUT_LIMIT",
                    PdfRenderError::OutputTooLarge => "PDF_BYTE_LIMIT",
                    PdfRenderError::InvalidContent => "PDF_INVALID_CONTENT",
                    PdfRenderError::Unavailable => "PDF_UNAVAILABLE",
                });
            }
        };
        let Some(generated_at_unix_ms) = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|value| u64::try_from(value.as_millis()).ok())
            .filter(|value| (1..=9_007_199_254_740_991).contains(value))
        else {
            return failure("PDF_UNAVAILABLE");
        };
        let preview = Arc::new(Preview {
            id: uuid::Uuid::now_v7().to_string(),
            source: manifest.source,
            revision: saved.revision,
            created: Instant::now(),
            generated_at_unix_ms,
            artifact,
            origin: PreviewOrigin::ActiveProfile,
        });
        if let Err(error) = app.state::<DesktopState>().with_store(|store| {
            store.record_render_manifest(
                preview.source,
                preview.revision,
                generated_at_unix_ms,
                &preview.artifact.receipt,
            )
        }) {
            return storage_failure(&error);
        }
        let preview_response = PdfPreviewResponse {
            render_id: preview.id.clone(),
            source: preview.source,
            revision: preview.revision,
            generated_at_unix_ms,
            receipt: preview.artifact.receipt.clone(),
            pdf_base64: STANDARD.encode(&preview.artifact.bytes),
        };
        let Ok(accessible_text) = ort_documents::render_plain_text(&saved.document) else {
            return failure("PDF_INVALID_CONTENT");
        };
        let cache = app.state::<PdfState>();
        let Ok(mut slot) = cache.0.lock() else {
            return failure("PDF_UNAVAILABLE");
        };
        *slot = Some(preview);
        CommandResponse::success(PdfReplayResponse {
            preview: preview_response,
            accessible_text,
        })
    })
    .await
    {
        Ok(response) => response,
        Err(_) => failure("PDF_UNAVAILABLE"),
    }
}

#[tauri::command]
pub(crate) async fn replay_portable_resume_pdf(
    window: WebviewWindow,
    request: PortablePdfReplayRequest,
) -> CommandResponse<PdfReplayResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let app = window.app_handle().clone();
    let Some(lease) = app.state::<ExportState>().begin() else {
        return failure("EXPORT_BUSY");
    };
    let Some(archive) = app
        .state::<PortablePdfState>()
        .get(&request.payload.archive_id, Instant::now())
    else {
        return failure("PORTABLE_PDF_ARCHIVE_EXPIRED");
    };
    let Some(entry) = archive
        .entries
        .iter()
        .find(|entry| entry.manifest.manifest_id == request.payload.manifest_id)
        .cloned()
    else {
        return failure("PDF_REPLAY_SOURCE_UNAVAILABLE");
    };
    if !app.state::<PdfState>().clear() {
        return failure("PDF_UNAVAILABLE");
    }
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let artifact = match render_matching_receipt(&entry.saved.document, &entry.manifest.receipt)
        {
            Ok(value) => value,
            Err(ReplayRenderError::Incompatible) => return failure("PDF_REPLAY_INCOMPATIBLE"),
            Err(ReplayRenderError::Render(error)) => {
                return failure(match error {
                    PdfRenderError::UnsupportedGlyph => "PDF_UNSUPPORTED_GLYPH",
                    PdfRenderError::LayoutLimit => "PDF_LAYOUT_LIMIT",
                    PdfRenderError::OutputTooLarge => "PDF_BYTE_LIMIT",
                    PdfRenderError::InvalidContent => "PDF_INVALID_CONTENT",
                    PdfRenderError::Unavailable => "PDF_UNAVAILABLE",
                });
            }
        };
        let Some(generated_at_unix_ms) = unix_time_millis() else {
            return failure("PDF_UNAVAILABLE");
        };
        let preview = Arc::new(Preview {
            id: uuid::Uuid::now_v7().to_string(),
            source: entry.manifest.source,
            revision: entry.saved.revision,
            created: Instant::now(),
            generated_at_unix_ms,
            artifact,
            origin: PreviewOrigin::PortableArchive,
        });
        let preview_response = PdfPreviewResponse {
            render_id: preview.id.clone(),
            source: preview.source,
            revision: preview.revision,
            generated_at_unix_ms,
            receipt: preview.artifact.receipt.clone(),
            pdf_base64: STANDARD.encode(&preview.artifact.bytes),
        };
        let Ok(accessible_text) = ort_documents::render_plain_text(&entry.saved.document) else {
            return failure("PDF_INVALID_CONTENT");
        };
        let cache = app.state::<PdfState>();
        let Ok(mut slot) = cache.0.lock() else {
            return failure("PDF_UNAVAILABLE");
        };
        *slot = Some(preview);
        CommandResponse::success(PdfReplayResponse {
            preview: preview_response,
            accessible_text,
        })
    })
    .await
    {
        Ok(response) => response,
        Err(_) => failure("PDF_UNAVAILABLE"),
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn release_portable_pdf_archive(
    window: WebviewWindow,
    request: PortablePdfArchiveRequest,
) -> CommandResponse<PortablePdfArchiveReleaseResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    CommandResponse::success(PortablePdfArchiveReleaseResponse {
        released: window
            .state::<PortablePdfState>()
            .release(&request.payload.archive_id),
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)] // Tauri command arguments are owned.
pub(crate) fn release_resume_pdf(
    window: WebviewWindow,
    request: PdfTicketRequest,
) -> CommandResponse<PdfReleaseResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    CommandResponse::success(PdfReleaseResponse {
        released: window
            .state::<PdfState>()
            .release(&request.payload.render_id),
    })
}

#[tauri::command]
pub(crate) async fn export_resume_pdf(
    window: WebviewWindow,
    request: PdfTicketRequest,
) -> CommandResponse<PdfExportResponse> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    if let Err(error) = request.validate() {
        return CommandResponse::Failure { ok: false, error };
    }
    let app = window.app_handle().clone();
    let Some(lease) = app.state::<ExportState>().begin() else {
        return failure("EXPORT_BUSY");
    };
    let Some(preview) = app
        .state::<PdfState>()
        .get(&request.payload.render_id, Instant::now())
    else {
        return failure("PDF_PREVIEW_EXPIRED");
    };
    if preview.origin == PreviewOrigin::ActiveProfile
        && let Err(error) = load_retained_exact(
            &app.state::<DesktopState>(),
            preview.source,
            preview.revision,
        )
    {
        return storage_failure(&error);
    }
    // The exact preview is captured before the dialog. Later saves cannot alter it.
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let selection = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Export unencrypted PDF — choose a new filename")
            .set_file_name("resume.pdf")
            .add_filter("PDF document", &["pdf"])
            .blocking_save_file();
        let Some(selection) = selection else {
            return CommandResponse::success(PdfExportResponse::Cancelled);
        };
        let FilePath::Path(path) = selection else {
            return failure("EXPORT_INVALID_DESTINATION");
        };
        match ExportDestination::for_native_dialog(&path, ExportFileType::Pdf)
            .and_then(|target| target.write(&preview.artifact.bytes))
        {
            Ok(receipt) => CommandResponse::success(PdfExportResponse::Exported {
                render_id: preview.id.clone(),
                pdf_sha256: preview.artifact.receipt.pdf_sha256.clone(),
                byte_count: preview.artifact.bytes.len(),
                cleanup_pending: receipt.cleanup_pending,
                durability_unconfirmed: receipt.durability_unconfirmed,
            }),
            Err(error) => failure(match error {
                ExportWriteError::AlreadyExists => "EXPORT_ALREADY_EXISTS",
                ExportWriteError::InvalidDestination => "EXPORT_INVALID_DESTINATION",
                ExportWriteError::InvalidContent => "EXPORT_INVALID_CONTENT",
                ExportWriteError::Unavailable => "EXPORT_UNAVAILABLE",
            }),
        }
    })
    .await
    {
        Ok(response) => response,
        Err(_) => failure("EXPORT_OUTCOME_UNKNOWN"),
    }
}

fn failure<T: serde::Serialize>(code: &str) -> CommandResponse<T> {
    CommandResponse::failure(code, "errors.pdf", false)
}

fn portable_history_failure(code: &str) -> CommandResponse<PortablePdfHistoryResponse> {
    CommandResponse::failure(code, "errors.portablePdfHistory", false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DesktopStorage;
    use ort_vault::testing::MemoryDatabaseKeyVault;
    use tempfile::TempDir;
    fn preview() -> Arc<Preview> {
        let mut document = ort_domain::ResumeDocument::empty("Private title");
        document.contact.full_name = "Synthetic".into();
        Arc::new(Preview {
            id: uuid::Uuid::now_v7().to_string(),
            source: ExportSource::SavedDraft,
            revision: 1,
            created: Instant::now(),
            generated_at_unix_ms: 1,
            artifact: ort_render::render_pdf(&document).unwrap(),
            origin: PreviewOrigin::ActiveProfile,
        })
    }
    #[test]
    fn ticket_expiry_release_and_replacement_are_identity_bound() {
        let first = preview();
        let state = PdfState(Mutex::new(Some(first.clone())));
        assert!(Arc::ptr_eq(
            &state.get(&first.id, first.created).unwrap(),
            &first
        ));
        assert!(state.get("unknown", first.created).is_none());
        assert!(!state.release("unknown"));
        let second = preview();
        *state.0.lock().unwrap() = Some(second.clone());
        assert!(!state.release(&first.id));
        assert!(state.get(&first.id, first.created).is_none());
        assert!(
            state
                .get(
                    &second.id,
                    second.created + Duration::from_secs(PDF_PREVIEW_TTL_SECONDS)
                )
                .is_none()
        );
        assert!(state.0.lock().unwrap().is_none());
        *state.0.lock().unwrap() = Some(first.clone());
        assert!(state.release(&first.id));
        assert!(!state.release(&first.id));
        *state.0.lock().unwrap() = Some(first);
        assert!(state.clear());
        assert!(state.0.lock().unwrap().is_none());
    }

    #[test]
    fn replay_exposes_bytes_only_when_the_full_receipt_matches() {
        let mut document = ort_domain::ResumeDocument::empty("Replay source");
        document.contact.full_name = "Synthetic Replay".into();
        let original = ort_render::render_pdf(&document).unwrap();
        let replay = render_matching_receipt(&document, &original.receipt).unwrap();
        assert_eq!(replay.bytes, original.bytes);

        let mut changed = document;
        changed.contact.full_name = "Changed Synthetic Replay".into();
        assert_eq!(
            render_matching_receipt(&changed, &original.receipt).err(),
            Some(ReplayRenderError::Incompatible)
        );
    }

    #[test]
    fn replay_loads_an_exact_older_publication_but_not_an_old_draft() {
        let temporary = TempDir::new().unwrap();
        let vault = MemoryDatabaseKeyVault::new();
        let store =
            ort_storage::EncryptedStore::open_or_initialize(temporary.path(), "test", &vault)
                .unwrap();
        let mut document = ort_domain::ResumeDocument::empty("First publication");
        document.contact.full_name = "Synthetic First".into();
        let first_draft = store.create_draft(&document).unwrap();
        let first_published = store.publish_draft(first_draft.revision).unwrap();
        let artifact = ort_render::render_pdf(&first_published.document).unwrap();
        let published_manifest = store
            .record_render_manifest(
                ExportSource::PublishedSnapshot,
                first_published.revision,
                1_000,
                &artifact.receipt,
            )
            .unwrap();
        let draft_manifest = store
            .record_render_manifest(
                ExportSource::SavedDraft,
                first_draft.revision,
                1_001,
                &artifact.receipt,
            )
            .unwrap();
        document.contact.full_name = "Synthetic Second".into();
        let second_draft = store.save_draft(first_draft.revision, &document).unwrap();
        store.publish_draft(second_draft.revision).unwrap();
        let state = DesktopState {
            storage: Mutex::new(DesktopStorage::Ready(store)),
        };

        let (loaded_manifest, loaded_source) =
            load_replay_source(&state, published_manifest.manifest_id).unwrap();
        assert_eq!(loaded_manifest, published_manifest);
        assert_eq!(loaded_source, first_published);
        assert_eq!(
            load_replay_source(&state, draft_manifest.manifest_id),
            Err(StorageError::NotFound)
        );
    }

    fn portable_manifest(
        source: ExportSource,
        revision: i64,
        document: &ort_domain::ResumeDocument,
        generated_at_unix_ms: u64,
    ) -> PortableRenderManifestV1 {
        PortableRenderManifestV1 {
            manifest_id: uuid::Uuid::now_v7().to_string(),
            source,
            source_revision: revision,
            generated_at_unix_ms,
            last_generated_at_unix_ms: generated_at_unix_ms,
            render_count: 1,
            receipt: ort_render::render_pdf(document).unwrap().receipt,
        }
    }

    #[test]
    fn portable_archive_retains_only_newest_exact_sources_and_expires() {
        let mut first = ort_domain::ResumeDocument::empty("Archived first");
        first.contact.full_name = "Synthetic Archived First".into();
        let mut second = first.clone();
        second.contact.full_name = "Synthetic Archived Second".into();
        let old_draft = portable_manifest(ExportSource::SavedDraft, 1, &first, 1_000);
        let publication = portable_manifest(ExportSource::PublishedSnapshot, 1, &first, 2_000);
        let current_draft = portable_manifest(ExportSource::SavedDraft, 2, &second, 3_000);
        let mut old_renderer = publication.clone();
        old_renderer.manifest_id = uuid::Uuid::now_v7().to_string();
        old_renderer.receipt.renderer_version = "typst-0.14.0/ort-1".into();
        let profile = PortableProfileV1 {
            master_draft: Some(ort_backup::PortableResumeRevisionV1 {
                revision: 2,
                document: second,
            }),
            published_resumes: vec![ort_backup::PortablePublishedResumeV1 {
                published_revision: 1,
                draft_revision: 1,
                document: first,
            }],
            settings: std::collections::BTreeMap::new(),
            render_manifests: vec![
                current_draft.clone(),
                publication.clone(),
                old_renderer,
                old_draft,
            ],
        };
        let created = Instant::now();
        let archive = build_portable_archive(&profile, created, 10_000).unwrap();
        assert_eq!(archive.total_manifests, 4);
        assert_eq!(archive.unavailable_sources, 1);
        assert_eq!(archive.incompatible_receipts, 1);
        assert_eq!(archive.entries.len(), 2);
        assert_eq!(
            archive.entries[0].manifest.manifest_id,
            current_draft.manifest_id
        );
        assert_eq!(
            archive.entries[1].manifest.manifest_id,
            publication.manifest_id
        );
        assert_eq!(
            archive.expires_at_unix_ms,
            10_000 + PDF_PREVIEW_TTL_SECONDS * 1_000
        );

        let state = PortablePdfState(Mutex::new(Some(archive.clone())));
        assert!(Arc::ptr_eq(
            &state.get(&archive.id, created).unwrap(),
            &archive
        ));
        assert!(state.get("unknown", created).is_none());
        assert!(!state.release("unknown"));
        assert!(
            state
                .get(
                    &archive.id,
                    created + Duration::from_secs(PDF_PREVIEW_TTL_SECONDS)
                )
                .is_none()
        );
        assert!(state.0.lock().unwrap().is_none());

        assert!(state.replace(archive.clone()));
        assert!(state.release(&archive.id));
        assert!(!state.release(&archive.id));
        assert!(state.replace(archive));
        assert!(state.clear());
        assert!(state.0.lock().unwrap().is_none());
    }

    #[test]
    fn portable_archive_authenticates_before_exposing_replay_sources() {
        let mut document = ort_domain::ResumeDocument::empty("Authenticated archive");
        document.contact.full_name = "Synthetic Authenticated Archive".into();
        let manifest = portable_manifest(ExportSource::SavedDraft, 1, &document, 1_000);
        let profile = PortableProfileV1 {
            master_draft: Some(ort_backup::PortableResumeRevisionV1 {
                revision: 1,
                document,
            }),
            published_resumes: Vec::new(),
            settings: std::collections::BTreeMap::new(),
            render_manifests: vec![manifest.clone()],
        };
        let passphrase = BackupPassphrase::new("synthetic archive phrase".into()).unwrap();
        let bytes = ort_backup::create_backup(
            &passphrase,
            ort_backup::BackupExportRequestV1 {
                app_version: "0.0.0-dev".into(),
                created_at: "2026-09-04T04:00:00Z".into(),
                profile,
            },
        )
        .unwrap();
        let created = Instant::now();
        let archive = authenticate_portable_archive(&bytes, &passphrase, created, 10_000).unwrap();
        assert_eq!(archive.entries.len(), 1);
        assert_eq!(
            archive.entries[0].manifest.manifest_id,
            manifest.manifest_id
        );

        let wrong = BackupPassphrase::new("wrong synthetic archive phrase".into()).unwrap();
        assert!(matches!(
            authenticate_portable_archive(&bytes, &wrong, created, 10_000),
            Err(BackupError::InvalidBackup)
        ));
        let mut damaged = bytes;
        let last = damaged.last_mut().unwrap();
        *last ^= 1;
        assert!(matches!(
            authenticate_portable_archive(&damaged, &passphrase, created, 10_000),
            Err(BackupError::InvalidBackup)
        ));
    }

    #[test]
    fn portable_archive_exposes_at_most_the_newest_twenty_receipts() {
        let mut document = ort_domain::ResumeDocument::empty("Bounded archive");
        document.contact.full_name = "Synthetic Bounded Archive".into();
        let receipt = ort_render::render_pdf(&document).unwrap().receipt;
        let mut published_resumes = Vec::new();
        let mut render_manifests = Vec::new();
        for revision in 1_i64..=21 {
            published_resumes.push(ort_backup::PortablePublishedResumeV1 {
                published_revision: revision,
                draft_revision: revision,
                document: document.clone(),
            });
            let timestamp = u64::try_from(22 - revision).unwrap() * 1_000;
            render_manifests.push(PortableRenderManifestV1 {
                manifest_id: uuid::Uuid::now_v7().to_string(),
                source: ExportSource::PublishedSnapshot,
                source_revision: revision,
                generated_at_unix_ms: timestamp,
                last_generated_at_unix_ms: timestamp,
                render_count: 1,
                receipt: receipt.clone(),
            });
        }
        let archive = build_portable_archive(
            &PortableProfileV1 {
                master_draft: None,
                published_resumes,
                settings: std::collections::BTreeMap::new(),
                render_manifests,
            },
            Instant::now(),
            10_000,
        )
        .unwrap();
        assert_eq!(archive.total_manifests, 21);
        assert_eq!(archive.entries.len(), usize::from(MAX_PDF_RENDER_HISTORY));
        assert_eq!(archive.entries[0].saved.revision, 1);
        assert_eq!(archive.entries[19].saved.revision, 20);
    }
}
