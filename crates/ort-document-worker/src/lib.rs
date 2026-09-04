//! Hostile-document parsers that are linked only into the disposable worker.
//!
//! Calling these functions does not establish a sandbox. The executable entry
//! point remains inert until the platform containment and launch gates pass.

mod docx;
mod pdf;

pub use docx::{DocxParseError, extract_docx};
pub use pdf::{PdfParseError, extract_pdf};
