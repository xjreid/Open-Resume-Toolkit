//! Narrow operating-system adapter boundary.

pub const COMPONENT_NAME: &str = "ort-platform";

mod export;
mod input;
pub use export::{ExportDestination, ExportFileType, ExportWriteError, ExportWriteReceipt};
pub use input::{NativeInputError, read_native_backup};
