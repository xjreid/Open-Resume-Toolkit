//! Native export cache, dialog and drag lifetimes. Sources and workflow policy live in ort-application.
#![allow(clippy::needless_pass_by_value)]
use crate::application_materials::error;
use crate::{DesktopState, storage_failure, window_not_authorized};
use base64::{Engine, engine::general_purpose::STANDARD};
use ort_application::application_workspace::load;
use ort_application::material_document::document_for;
use ort_documents::render_docx_with_style;
use ort_domain::CommandResponse;
use ort_platform::{ExportDestination, ExportFileType};
use ort_storage::StorageError;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
#[derive(Default)]
pub struct DragFiles {
    pub(crate) session: Arc<Mutex<DragSession>>,
}

#[derive(Default)]
pub(crate) struct DragSession {
    pub(crate) directory: Option<tempfile::TempDir>,
    pub(crate) active_drags: usize,
    pub(crate) clear_pending: bool,
}

#[cfg(target_os = "macos")]
pub(crate) struct DragFileLease {
    pub(crate) session: Arc<Mutex<DragSession>>,
}

#[cfg(target_os = "macos")]
impl Drop for DragFileLease {
    fn drop(&mut self) {
        let mut session = self
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        session.active_drags = session.active_drags.saturating_sub(1);
        if session.active_drags == 0 && session.clear_pending {
            session.directory = None;
            session.clear_pending = false;
        }
    }
}

#[derive(Default)]
pub struct ApplicationExportState {
    pub(crate) prepared: Mutex<Vec<PreparedApplicationExports>>,
}

#[derive(Clone)]
pub(crate) struct PreparedApplicationExports {
    pub(crate) profile_id: uuid::Uuid,
    pub(crate) revision: i64,
    pub(crate) kind: MaterialKind,
    pub(crate) pdf: Vec<u8>,
    pub(crate) receipt: ort_domain::PdfRenderReceipt,
    pub(crate) docx: Vec<u8>,
}

impl ApplicationExportState {
    pub(crate) fn clear(&self) {
        self.prepared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }

    pub(crate) fn replace(&self, exports: PreparedApplicationExports) -> bool {
        let mut prepared = self
            .prepared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if prepared.iter().any(|current| {
            current.profile_id == exports.profile_id
                && current.kind == exports.kind
                && current.revision > exports.revision
        }) {
            return false;
        }
        prepared.retain(|current| current.kind != exports.kind);
        prepared.push(exports);
        true
    }

    pub(crate) fn promote_unchanged(
        &self,
        profile_id: uuid::Uuid,
        previous_revision: i64,
        revision: i64,
        kind: MaterialKind,
    ) {
        let mut prepared = self
            .prepared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(mut next) = prepared
            .iter()
            .find(|item| {
                item.profile_id == profile_id
                    && item.revision == previous_revision
                    && item.kind == kind
            })
            .cloned()
        else {
            return;
        };
        next.revision = revision;
        prepared.retain(|item| item.kind != kind);
        prepared.push(next);
    }

    #[cfg(test)]
    pub(crate) fn is_prepared(
        &self,
        profile_id: uuid::Uuid,
        revision: i64,
        kind: MaterialKind,
    ) -> bool {
        self.prepared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .any(|item| {
                item.profile_id == profile_id && item.revision == revision && item.kind == kind
            })
    }

    pub(crate) fn preview(
        &self,
        profile_id: uuid::Uuid,
        revision: i64,
        kind: MaterialKind,
    ) -> Option<ort_domain::PdfPreviewResponse> {
        let prepared = self.prepared.lock().ok()?;
        let export = prepared.iter().find(|item| {
            item.profile_id == profile_id && item.revision == revision && item.kind == kind
        })?;
        Some(ort_domain::PdfPreviewResponse {
            render_id: uuid::Uuid::now_v7().to_string(),
            source: ort_domain::ExportSource::SavedDraft,
            revision,
            generated_at_unix_ms: u64::try_from(jiff::Timestamp::now().as_millisecond()).ok()?,
            receipt: export.receipt.clone(),
            pdf_base64: STANDARD.encode(&export.pdf),
        })
    }

    pub(crate) fn bytes(
        &self,
        profile_id: uuid::Uuid,
        revision: i64,
        kind: MaterialKind,
        format: ApplicationExportFormat,
    ) -> Option<Vec<u8>> {
        self.prepared
            .lock()
            .ok()?
            .iter()
            .find(|prepared| {
                prepared.profile_id == profile_id
                    && prepared.revision == revision
                    && prepared.kind == kind
            })
            .map(|prepared| match format {
                ApplicationExportFormat::Pdf => prepared.pdf.clone(),
                ApplicationExportFormat::Docx => prepared.docx.clone(),
            })
    }
}

impl DragFiles {
    #[cfg(target_os = "macos")]
    pub(crate) fn sweep_stale() {
        use std::os::unix::fs::MetadataExt;
        use std::time::Duration;

        let root = std::env::temp_dir();
        // This is a cleanup hint, never a reason to fail desktop startup.
        // An age threshold avoids touching another live ORT process.
        let Ok(owner_probe) = tempfile::Builder::new()
            .prefix("ort-drag-probe-")
            .tempdir_in(&root)
        else {
            return;
        };
        let Ok(owner) = owner_probe.path().metadata().map(|value| value.uid()) else {
            return;
        };
        let Ok(children) = std::fs::read_dir(&root) else {
            return;
        };
        for child in children.flatten() {
            let path = child.path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !name.starts_with("ort-drag-") || name.starts_with("ort-drag-probe-") {
                continue;
            }
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if !meta.file_type().is_dir()
                || meta.uid() != owner
                || meta.mode() & 0o777 != 0o700
                || meta
                    .modified()
                    .ok()
                    .and_then(|time| time.elapsed().ok())
                    .is_none_or(|age| age < Duration::from_hours(24))
            {
                continue;
            }
            let _ = std::fs::remove_dir_all(path);
        }
    }

    pub(crate) fn clear(&self) {
        let mut session = self
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if session.active_drags == 0 {
            session.directory = None;
            session.clear_pending = false;
        } else {
            session.clear_pending = true;
        }
    }

    #[cfg(target_os = "macos")]
    pub(crate) fn materialize(
        &self,
        bytes: &[u8],
        name: &str,
    ) -> std::io::Result<(std::path::PathBuf, DragFileLease)> {
        use std::io::Write;
        use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

        let mut guard = self
            .session
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.directory.is_none() {
            guard.directory = Some(tempfile::Builder::new().prefix("ort-drag-").tempdir()?);
        }
        let root = guard
            .directory
            .as_ref()
            .expect("drag session was created")
            .path();
        let folder = root.join(uuid::Uuid::now_v7().to_string());
        std::fs::DirBuilder::new().mode(0o700).create(&folder)?;
        let path = folder.join(name);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        guard.active_drags += 1;
        Ok((
            path,
            DragFileLease {
                session: Arc::clone(&self.session),
            },
        ))
    }
}

pub use ort_domain::MaterialKind;

#[derive(Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ApplicationExportFormat {
    Pdf,
    Docx,
}

impl ApplicationExportFormat {
    fn file_type(self) -> ExportFileType {
        match self {
            Self::Pdf => ExportFileType::Pdf,
            Self::Docx => ExportFileType::Docx,
        }
    }

    fn filename(self, kind: MaterialKind) -> &'static str {
        match (kind, self) {
            (MaterialKind::Resume, Self::Pdf) => "tailored-resume.pdf",
            (MaterialKind::Resume, Self::Docx) => "tailored-resume.docx",
            (MaterialKind::CoverLetter, Self::Pdf) => "cover-letter.pdf",
            (MaterialKind::CoverLetter, Self::Docx) => "cover-letter.docx",
        }
    }

    fn dialog(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Pdf => (
                "Export unencrypted application PDF — choose a new filename",
                "PDF document",
                "pdf",
            ),
            Self::Docx => (
                "Export unencrypted application DOCX — choose a new filename",
                "Word document",
                "docx",
            ),
        }
    }
}

#[derive(Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreparedApplicationExport {
    pub revision: i64,
    pub pdf_ready: bool,
    pub page_count: usize,
    pub docx_ready: bool,
}

pub(crate) fn render_application_exports(
    state: &DesktopState,
    expected_revision: i64,
    kind: MaterialKind,
) -> Result<PreparedApplicationExports, StorageError> {
    let (profile_id, document, style) = state.with_store(|store| {
        let current = load(store)?.ok_or(StorageError::NotFound)?;
        if current.revision != expected_revision {
            return Err(StorageError::RevisionConflict);
        }
        Ok((
            store.manifest().profile_id,
            document_for(
                &current.workspace.resume,
                current.workspace.cover_letter.as_deref(),
                kind,
            )?,
            current.workspace.style,
        ))
    })?;
    let pdf = ort_render::render_pdf_with_style(&document, style)
        .map_err(|_| StorageError::InvalidData)?;
    let docx = render_docx_with_style(&document, style).map_err(|_| StorageError::InvalidData)?;
    Ok(PreparedApplicationExports {
        profile_id,
        revision: expected_revision,
        kind,
        pdf: pdf.bytes,
        receipt: pdf.receipt,
        docx,
    })
}

#[tauri::command]
pub fn prepare_application_exports(
    window: WebviewWindow,
    expected_revision: i64,
    kind: MaterialKind,
) -> CommandResponse<PreparedApplicationExport> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    let profile_id = match window
        .state::<DesktopState>()
        .with_store(|store| Ok(store.manifest().profile_id))
    {
        Ok(profile) => profile,
        Err(problem) => return storage_failure(&problem),
    };
    let cached_pages = window
        .state::<ApplicationExportState>()
        .prepared
        .lock()
        .ok()
        .and_then(|items| {
            items
                .iter()
                .find(|item| {
                    item.profile_id == profile_id
                        && item.revision == expected_revision
                        && item.kind == kind
                })
                .map(|item| item.receipt.page_count)
        });
    if let Some(page_count) = cached_pages {
        let current = window.state::<DesktopState>().with_store(|store| {
            ort_application::application_workspace::ensure_profile(store, profile_id)?;
            Ok(load(store)?.is_some_and(|saved| saved.revision == expected_revision))
        });
        match current {
            Ok(true) => {
                return CommandResponse::success(PreparedApplicationExport {
                    revision: expected_revision,
                    pdf_ready: true,
                    page_count,
                    docx_ready: true,
                });
            }
            Ok(false) => return storage_failure(&StorageError::RevisionConflict),
            Err(problem) => return storage_failure(&problem),
        }
    }
    let prepared = match render_application_exports(
        &window.state::<DesktopState>(),
        expected_revision,
        kind,
    ) {
        Ok(value) => value,
        Err(StorageError::InvalidData) => return error("EXPORT_PREPARE_FAILED"),
        Err(problem) => return storage_failure(&problem),
    };
    let page_count = prepared.receipt.page_count;
    let still_current = window.state::<DesktopState>().with_store(|store| {
        Ok(
            load(store)?.is_some_and(|saved| saved.revision == expected_revision)
                && store.manifest().profile_id == prepared.profile_id
                && window.state::<ApplicationExportState>().replace(prepared),
        )
    });
    match still_current {
        Ok(true) => {}
        Ok(_) => return storage_failure(&StorageError::RevisionConflict),
        Err(problem) => return storage_failure(&problem),
    }
    CommandResponse::success(PreparedApplicationExport {
        revision: expected_revision,
        pdf_ready: true,
        page_count,
        docx_ready: true,
    })
}

pub(crate) fn prepared_application_export_bytes(
    window: &WebviewWindow,
    expected_revision: i64,
    kind: MaterialKind,
    format: ApplicationExportFormat,
) -> Result<Vec<u8>, CommandResponse<bool>> {
    window
        .state::<DesktopState>()
        .with_store(|store| {
            if load(store)?.is_none_or(|saved| saved.revision != expected_revision) {
                return Err(StorageError::RevisionConflict);
            }
            window
                .state::<ApplicationExportState>()
                .bytes(store.manifest().profile_id, expected_revision, kind, format)
                .ok_or(StorageError::NotFound)
        })
        .map_err(|problem| storage_failure(&problem))
}

#[tauri::command]
pub async fn download_application_export(
    window: WebviewWindow,
    expected_revision: i64,
    kind: MaterialKind,
    format: ApplicationExportFormat,
) -> CommandResponse<bool> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    let bytes = match prepared_application_export_bytes(&window, expected_revision, kind, format) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(lease) = window.state::<crate::text_export::ExportState>().begin() else {
        return error("EXPORT_BUSY");
    };
    let (title, filter, extension) = format.dialog();
    let filename = format.filename(kind);
    match tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        let path = window
            .dialog()
            .file()
            .set_parent(&window)
            .set_title(title)
            .set_file_name(filename)
            .add_filter(filter, &[extension])
            .blocking_save_file();
        let Some(path) = path else {
            return error("EXPORT_CANCELLED");
        };
        let Some(path) = path.as_path() else {
            return error("EXPORT_INVALID_DESTINATION");
        };
        match ExportDestination::for_native_dialog(path, format.file_type())
            .and_then(|destination| destination.write(&bytes))
        {
            Ok(_) => CommandResponse::success(true),
            Err(_) => error("EXPORT_FAILED"),
        }
    })
    .await
    {
        Ok(result) => result,
        Err(_) => error("EXPORT_FAILED"),
    }
}

#[tauri::command]
pub async fn drag_application_export(
    window: WebviewWindow,
    expected_revision: i64,
    kind: MaterialKind,
    format: ApplicationExportFormat,
) -> CommandResponse<bool> {
    if window.label() != "overlay" {
        return window_not_authorized();
    }
    let bytes = match prepared_application_export_bytes(&window, expected_revision, kind, format) {
        Ok(value) => value,
        Err(response) => return response,
    };
    #[cfg(not(target_os = "macos"))]
    {
        let _ = bytes;
        error("DRAG_UNAVAILABLE")
    }
    #[cfg(target_os = "macos")]
    {
        let drag_files = window.state::<DragFiles>();
        let Ok((path, drag_lease)) = drag_files.materialize(&bytes, format.filename(kind)) else {
            return error("DRAG_UNAVAILABLE");
        };
        let native_window = window.clone();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        let app = window.app_handle().clone();
        if app
            .run_on_main_thread(move || {
                let drag_lease = Mutex::new(Some(drag_lease));
                let outcome = drag::start_drag(
                    &native_window,
                    drag::DragItem::Files(vec![path]),
                    drag::Image::Raw(include_bytes!("../icons/32x32.png").to_vec()),
                    move |_result, _position| {
                        drop(
                            drag_lease
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner)
                                .take(),
                        );
                    },
                    drag::Options::default(),
                );
                let _ = sender.send(outcome.is_ok());
            })
            .is_err()
        {
            return error("DRAG_UNAVAILABLE");
        }
        match tauri::async_runtime::spawn_blocking(move || receiver.recv()).await {
            Ok(Ok(true)) => CommandResponse::success(true),
            _ => error("DRAG_UNAVAILABLE"),
        }
    }
}

#[derive(Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MaterialPdf {
    pub base64: String,
    pub filename: String,
}

/// The popup displays the exact cached bytes used by Download, bound to the
/// saved workspace revision. It cannot submit PDF bytes or a destination path.
#[tauri::command]
pub fn preview_application_pdf(
    window: WebviewWindow,
    expected_revision: i64,
    kind: MaterialKind,
) -> CommandResponse<ort_domain::PdfPreviewResponse> {
    if !matches!(window.label(), "overlay" | "application-popup") {
        return window_not_authorized();
    }
    match prepared_application_pdf(
        &window.state::<DesktopState>(),
        &window.state::<ApplicationExportState>(),
        expected_revision,
        kind,
    ) {
        Ok(Some(preview)) => CommandResponse::success(preview),
        Ok(None) => error("EXPORT_NOT_PREPARED"),
        Err(problem) => storage_failure(&problem),
    }
}

pub(crate) fn prepared_application_pdf(
    state: &DesktopState,
    exports: &ApplicationExportState,
    expected_revision: i64,
    kind: MaterialKind,
) -> Result<Option<ort_domain::PdfPreviewResponse>, StorageError> {
    state.with_store(|store| {
        let current = load(store)?.ok_or(StorageError::NotFound)?;
        if current.revision != expected_revision {
            return Err(StorageError::RevisionConflict);
        }
        Ok(exports.preview(store.manifest().profile_id, expected_revision, kind))
    })
}
