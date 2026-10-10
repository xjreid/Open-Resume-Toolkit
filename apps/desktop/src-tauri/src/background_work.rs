//! Bounded background jobs keep CPU rendering and large queries off the UI thread.
use ort_domain::CommandResponse;
use serde::Serialize;
use tokio::sync::Semaphore;

static JOBS: Semaphore = Semaphore::const_new(4);

pub(crate) async fn run<T, F>(job: F) -> CommandResponse<T>
where
    T: Serialize + Send + 'static,
    F: FnOnce() -> CommandResponse<T> + Send + 'static,
{
    let Ok(permit) = JOBS.try_acquire() else {
        return CommandResponse::failure("BACKGROUND_BUSY", "errors.backgroundBusy", true);
    };
    // The permit belongs to the blocking job, including when its caller closes.
    match tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        job()
    })
    .await
    {
        Ok(response) => response,
        Err(_) => {
            CommandResponse::failure("COMMAND_UNAVAILABLE", "errors.commandUnavailable", true)
        }
    }
}

struct CachedPdf {
    profile: uuid::Uuid,
    digest: [u8; 32],
    style: ort_domain::DocumentStyle,
    artifact: ort_render::PdfArtifact,
}
static PDF_CACHE: std::sync::Mutex<Vec<CachedPdf>> = std::sync::Mutex::new(Vec::new());
static CACHE_GENERATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub(crate) fn clear_render_cache() {
    CACHE_GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    if let Ok(mut cache) = PDF_CACHE.lock() {
        cache.clear();
    }
}
pub(crate) fn render_pdf(
    profile: uuid::Uuid,
    document: &ort_domain::ResumeDocument,
    style: ort_domain::DocumentStyle,
) -> Result<ort_render::PdfArtifact, ort_render::PdfRenderError> {
    use sha2::{Digest, Sha256};
    let digest: [u8; 32] = Sha256::digest(
        serde_json::to_vec(document).map_err(|_| ort_render::PdfRenderError::InvalidContent)?,
    )
    .into();
    let generation = CACHE_GENERATION.load(std::sync::atomic::Ordering::SeqCst);
    let mut cache = PDF_CACHE
        .lock()
        .map_err(|_| ort_render::PdfRenderError::Unavailable)?;
    if let Some(hit) = cache
        .iter()
        .find(|entry| entry.profile == profile && entry.digest == digest && entry.style == style)
    {
        return Ok(ort_render::PdfArtifact {
            bytes: hit.artifact.bytes.clone(),
            receipt: hit.artifact.receipt.clone(),
        });
    }
    let artifact = ort_render::render_pdf_with_style(document, style)?;
    if generation == CACHE_GENERATION.load(std::sync::atomic::Ordering::SeqCst) {
        // At most two bounded artifacts. Profile retirement invalidates in-flight insertions.
        if cache.len() >= 2 {
            cache.remove(0);
        }
        cache.push(CachedPdf {
            profile,
            digest,
            style,
            artifact: ort_render::PdfArtifact {
                bytes: artifact.bytes.clone(),
                receipt: artifact.receipt.clone(),
            },
        });
    }
    Ok(artifact)
}
pub(crate) fn preflight_pdf(
    profile: uuid::Uuid,
    workspace: &ort_domain::ApplicationWorkspace,
    kind: ort_domain::MaterialKind,
) -> Result<(), &'static str> {
    let document = ort_application::material_document::document_for(
        &workspace.resume,
        workspace.cover_letter.as_deref(),
        kind,
    )
    .map_err(|_| "PDF_UNAVAILABLE")?;
    document
        .validate(ort_domain::DocumentLimits::default())
        .map_err(|_| "RESUME_INVALID")?;
    render_pdf(profile, &document, workspace.style)
        .map(|_| ())
        .map_err(|_| "PDF_UNAVAILABLE")
}
