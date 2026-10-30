//! PDF.js prepares source hyperlink metadata for both routes and page images
//! for Codex. No resume text extraction. Replies belong to their main window.
use crate::{DesktopState, window_not_authorized};
use base64::{Engine, engine::general_purpose::STANDARD};
use ort_domain::CommandResponse;
use serde::Deserialize;
use std::sync::Mutex;
use tauri::{Emitter, Manager, WebviewWindow};
use tokio::sync::oneshot;
use uuid::Uuid;

const MAX_PAGES: usize = 10;
const MAX_PAGE_BYTES: usize = 3 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 20 * 1024 * 1024;

pub(crate) struct PdfPages {
    pages: Vec<Vec<u8>>,
    links: Vec<ort_ai::resume_import::PdfHyperlink>,
}
impl PdfPages {
    #[cfg(test)]
    pub(crate) fn fixture(count: usize) -> Self {
        Self::decode(
            RenderReply {
                id: Uuid::now_v7(),
                pages: vec![tests::png(); count],
                links: vec![],
                page_count: count,
                error: None,
            },
            true,
        )
        .expect("synthetic PNG pages")
    }
    #[cfg(all(test, unix))]
    pub(crate) fn fixture_with_links(
        count: usize,
        links: Vec<ort_ai::resume_import::PdfHyperlink>,
    ) -> Self {
        assert!(ort_ai::resume_import::valid_pdf_hyperlinks(&links, count));
        let mut fixture = Self::fixture(count);
        fixture.links = links;
        fixture
    }
    pub(crate) fn pages(&self) -> &[Vec<u8>] {
        &self.pages
    }
    pub(crate) fn links(&self) -> &[ort_ai::resume_import::PdfHyperlink] {
        &self.links
    }
    fn decode(reply: RenderReply, images: bool) -> Result<Self, &'static str> {
        if let Some(error) = reply.error {
            return Err(match error.as_str() {
                "AI_PDF_PAGE_LIMIT" => "AI_PDF_PAGE_LIMIT",
                "AI_PDF_IMAGES_TOO_LARGE" => "AI_PDF_IMAGES_TOO_LARGE",
                "AI_PDF_RENDER_TIMEOUT" => "AI_PDF_RENDER_TIMEOUT",
                "AI_PDF_LINK_LIMIT" => "AI_PDF_LINK_LIMIT",
                _ => "AI_PDF_RENDER_FAILED",
            });
        }
        if reply.page_count == 0
            || reply.page_count > (if images { MAX_PAGES } else { 1000 })
            || (images && reply.pages.len() != reply.page_count)
            || (!images && !reply.pages.is_empty())
        {
            return Err("AI_PDF_PAGE_LIMIT");
        }
        if !ort_ai::resume_import::valid_pdf_hyperlinks(&reply.links, reply.page_count) {
            return Err("AI_PDF_LINK_LIMIT");
        }
        let mut total = 0;
        let mut pages = Vec::new();
        for encoded in reply.pages {
            if encoded.len() > MAX_PAGE_BYTES * 4 / 3 + 4 {
                return Err("AI_PDF_IMAGES_TOO_LARGE");
            }
            let bytes = STANDARD
                .decode(encoded)
                .map_err(|_| "AI_PDF_RENDER_FAILED")?;
            total += bytes.len();
            if bytes.len() > MAX_PAGE_BYTES || total > MAX_TOTAL_BYTES {
                return Err("AI_PDF_IMAGES_TOO_LARGE");
            }
            // Fixed PNG format, bounded canvas dimensions. Decoding the actual
            // pixels belongs to the supervised runtime, never native IPC.
            if bytes.len() < 33
                || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
                || bytes[8..16] != [0, 0, 0, 13, b'I', b'H', b'D', b'R']
            {
                return Err("AI_PDF_RENDER_FAILED");
            }
            let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
            let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
            if width == 0 || height == 0 || width > 1800 || height > 1800 {
                return Err("AI_PDF_RENDER_FAILED");
            }
            pages.push(bytes);
        }
        Ok(Self {
            pages,
            links: reply.links,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RenderReply {
    id: Uuid,
    pages: Vec<String>,
    links: Vec<ort_ai::resume_import::PdfHyperlink>,
    page_count: usize,
    error: Option<String>,
}
type RenderResult = Result<PdfPages, &'static str>;
#[derive(Default)]
pub(crate) struct RenderState(Mutex<Option<(Uuid, bool, oneshot::Sender<RenderResult>)>>);
struct Lease<'a>(&'a RenderState, Uuid);
impl Drop for Lease<'_> {
    fn drop(&mut self) {
        if let Ok(mut pending) = self.0.0.lock()
            && pending.as_ref().is_some_and(|(id, _, _)| *id == self.1)
        {
            pending.take();
        }
    }
}
impl RenderState {
    fn begin(
        &self,
        images: bool,
    ) -> Result<(Lease<'_>, oneshot::Receiver<RenderResult>), &'static str> {
        let mut pending = self.0.lock().map_err(|_| "AI_PDF_RENDER_FAILED")?;
        if pending.is_some() {
            return Err("LOCAL_DATA_OPERATION_BUSY");
        }
        let id = Uuid::now_v7();
        let (send, receive) = oneshot::channel();
        *pending = Some((id, images, send));
        Ok((Lease(self, id), receive))
    }
    fn finish(&self, reply: RenderReply) -> bool {
        let Ok(mut pending) = self.0.lock() else {
            return false;
        };
        if pending.as_ref().is_none_or(|(id, _, _)| *id != reply.id) {
            return false;
        }
        let (_, images, send) = pending.take().expect("matched render request");
        drop(pending);
        send.send(PdfPages::decode(reply, images)).is_ok()
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn complete_ai_import_render(
    window: WebviewWindow,
    reply: RenderReply,
) -> CommandResponse<bool> {
    if window.label() != "main" {
        return window_not_authorized();
    }
    CommandResponse::success(
        window
            .state::<DesktopState>()
            .reviews
            .pdf_render
            .finish(reply),
    )
}

pub(super) async fn render(
    window: &WebviewWindow,
    pdf: &[u8],
    cancel: &std::sync::atomic::AtomicBool,
    images: bool,
) -> RenderResult {
    let state = window.state::<DesktopState>();
    let (lease, receive) = state.reviews.pdf_render.begin(images)?;
    window
        .emit_to(
            "main",
            "ort:ai-import-render-pdf",
            serde_json::json!({
                "id":lease.1, "base64":STANDARD.encode(pdf), "images":images
            }),
        )
        .map_err(|_| "AI_PDF_RENDER_FAILED")?;
    tokio::select! {
        result = receive => result.map_err(|_| "AI_PDF_RENDER_FAILED")?,
        () = super::ai::cancelled(cancel) => Err("AI_CANCELLED"),
        () = tokio::time::sleep(std::time::Duration::from_secs(30)) => Err("AI_PDF_RENDER_TIMEOUT"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(crate) fn png() -> String {
        // Real, synthetic 1x1 PNG. Runtime image loading is separately qualified.
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAIAAACQd1PeAAAADElEQVR4nGP4//8/AAX+Av4N70a4AAAAAElFTkSuQmCC".into()
    }
    #[test]
    fn render_replies_are_owned_one_shot_and_discarded_after_cancel() {
        let state = RenderState::default();
        let (lease, mut result) = state.begin(true).unwrap();
        assert!(state.begin(true).is_err());
        assert!(!state.finish(RenderReply {
            id: Uuid::now_v7(),
            pages: vec![png()],
            page_count: 1,
            links: vec![],
            error: None
        }));
        assert!(state.finish(RenderReply {
            id: lease.1,
            pages: vec![png(), png()],
            page_count: 2,
            links: vec![],
            error: None
        }));
        assert_eq!(result.try_recv().unwrap().unwrap().pages().len(), 2);
        assert!(!state.finish(RenderReply {
            id: lease.1,
            pages: vec![png()],
            page_count: 1,
            links: vec![],
            error: None
        }));
        drop(lease);
        let (lease, mut result) = state.begin(true).unwrap();
        let id = lease.1;
        drop(lease);
        assert!(!state.finish(RenderReply {
            id,
            pages: vec![png()],
            page_count: 1,
            links: vec![],
            error: None
        }));
        assert!(result.try_recv().is_err());
    }
    #[test]
    fn rejects_empty_oversized_non_image_and_unknown_error_content() {
        for pages in [
            vec![],
            vec![png(); 11],
            vec!["private content".into()],
            vec!["A".repeat(MAX_PAGE_BYTES * 2)],
        ] {
            assert!(
                PdfPages::decode(
                    RenderReply {
                        id: Uuid::now_v7(),
                        page_count: pages.len(),
                        links: vec![],
                        pages,
                        error: None
                    },
                    true
                )
                .is_err()
            );
        }
        assert!(matches!(
            PdfPages::decode(
                RenderReply {
                    id: Uuid::now_v7(),
                    pages: vec![],
                    page_count: 0,
                    links: vec![],
                    error: Some("private error".into())
                },
                true
            ),
            Err("AI_PDF_RENDER_FAILED")
        ));
    }

    #[test]
    fn api_metadata_is_image_free_and_owned_by_the_requested_mode() {
        let reply = || RenderReply {
            id: Uuid::now_v7(),
            pages: vec![],
            page_count: 11,
            links: vec![ort_ai::resume_import::PdfHyperlink {
                page: 11,
                rect: [10, 20, 30, 40],
                url: "https://github.com/example/project".into(),
            }],
            error: None,
        };
        let prepared = PdfPages::decode(reply(), false).unwrap();
        assert!(prepared.pages().is_empty());
        assert_eq!(prepared.links()[0].page, 11);
        assert!(matches!(
            PdfPages::decode(reply(), true),
            Err("AI_PDF_PAGE_LIMIT")
        ));
        let mut bad = reply();
        bad.links[0].page = 12;
        assert!(matches!(
            PdfPages::decode(bad, false),
            Err("AI_PDF_LINK_LIMIT")
        ));
        let mut bad = reply();
        bad.pages.push(png());
        assert!(matches!(
            PdfPages::decode(bad, false),
            Err("AI_PDF_PAGE_LIMIT")
        ));
        let mut bad = reply();
        bad.links[0].url = "file:///PRIVATE_DATA".into();
        assert!(matches!(
            PdfPages::decode(bad, false),
            Err("AI_PDF_LINK_LIMIT")
        ));
    }
}
