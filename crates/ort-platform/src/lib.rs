//! Narrow operating-system adapter boundary.

pub const COMPONENT_NAME: &str = "ort-platform";

mod export;
pub use export::{ExportDestination, ExportWriteError, ExportWriteReceipt};
