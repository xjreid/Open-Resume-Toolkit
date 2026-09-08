//! Hostile-document parsers that are linked only into the disposable worker.
//!
//! Calling these functions does not establish a sandbox. The executable entry
//! point remains inert until the platform containment and launch gates pass.

mod docx;
#[cfg(feature = "native-pdf")]
mod pdf;

pub use docx::{DocxParseError, extract_docx};
#[cfg(feature = "native-pdf")]
pub use pdf::{PdfParseError, extract_pdf};
