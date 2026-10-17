//! Source hyperlink destinations lost when PDF pages become images. Rectangles
//! use the rendered page's top-left origin, normalized to 0..=1000.
use super::{contact_url, input};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const MAX_PDF_LINKS: usize = 200;
pub const MAX_PDF_LINK_BYTES: usize = 48_000;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PdfHyperlink {
    pub page: u16,
    pub rect: [u16; 4],
    pub url: String,
}

/// Validates the bounded, source-only sidecar received from PDF.js. It never
/// grants permission to open a URL or execute a PDF action.
#[must_use]
pub fn valid_pdf_hyperlinks(links: &[PdfHyperlink], page_count: usize) -> bool {
    links.len() <= MAX_PDF_LINKS
        && serde_json::to_vec(links).is_ok_and(|bytes| bytes.len() <= MAX_PDF_LINK_BYTES)
        && links.iter().all(|link| {
            link.page > 0
                && usize::from(link.page) <= page_count
                && link.rect[0] < link.rect[2]
                && link.rect[1] < link.rect[3]
                && link.rect.iter().all(|&coordinate| coordinate <= 1000)
                && link.url.chars().count() <= 2000
                && contact_url(&link.url, 0).is_ok_and(|url| url == link.url)
        })
}

#[must_use]
pub fn input_with_pdf_hyperlinks(links: &[PdfHyperlink]) -> Value {
    let mut input = input();
    input["pdfHyperlinks"] = json!({
        "coordinates":"[left, top, right, bottom], 0..1000 on the rendered page, origin top-left; page numbers start at 1",
        "links":links
    });
    input
}
