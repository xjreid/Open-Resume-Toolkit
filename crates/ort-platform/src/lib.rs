//! Narrow operating-system adapter boundary.

pub const COMPONENT_NAME: &str = "ort-platform";

mod export;
mod import_staging;
mod input;
pub use export::{ExportDestination, ExportFileType, ExportWriteError, ExportWriteReceipt};
pub use import_staging::{
    IMPORT_STAGE_MAX_AGE, IMPORTS_DIRECTORY, ImportCleanupReport, ImportStageError,
    ImportStagingRoot, MAX_IMPORT_STAGE_ENTRIES, StagedImport,
};
pub use input::{
    NativeDocumentFormat, NativeDocumentSource, NativeInputError, read_native_backup,
    read_native_document,
};
