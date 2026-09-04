//! Preparation boundary for the disabled hostile-document import path.
//!
//! This composes native acquisition, bounded format-envelope inspection, and
//! private staging over the same owned bytes. It does not launch a parser,
//! expose a desktop command, or enable import.

use std::{fs::File, sync::atomic::AtomicBool};

use ort_documents::{
    IMPORT_ENABLED,
    import::{InputFormat, ValidatedExtraction},
    import_source::{SourceError, SourceInspection, inspect_source},
    worker_supervisor::{
        ContainedWorker, NativeAdapterError, OPERATION_NONCE_BYTES, SupervisionError, supervise,
    },
};
use ort_platform::{
    ImportStageError, ImportStagingRoot, NativeDocumentFormat, NativeDocumentSource, StagedImport,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DocumentImportError {
    #[error("document import remains disabled pending native containment proof")]
    Disabled,
    #[error("the selected document envelope is unsupported")]
    Source(#[from] SourceError),
    #[error("private document staging is unavailable")]
    Staging(#[from] ImportStageError),
    #[error("the native document worker could not be launched")]
    Native(#[from] NativeAdapterError),
    #[error("document-worker supervision rejected the extraction")]
    Supervision(#[from] SupervisionError),
}

/// Reviewed native adapter entry point. The adapter receives one operation
/// nonce, independently validated expected format, and one read-only staged
/// input handle. It must establish containment before returning the worker.
pub trait ContainedWorkerLauncher {
    type Worker: ContainedWorker;

    /// # Errors
    /// Returns a content-free native error without retaining the input handle
    /// when containment cannot be established.
    fn launch(
        &mut self,
        operation_nonce: [u8; OPERATION_NONCE_BYTES],
        format: InputFormat,
        input: File,
    ) -> Result<Self::Worker, NativeAdapterError>;
}

/// A preflighted source staged for transfer by handle into a future native
/// containment adapter. The selected user path and source bytes are not exposed.
pub struct PreparedImport {
    inspection: SourceInspection,
    stage: StagedImport,
}

impl PreparedImport {
    /// Inspects and stages the same owned native source without rereading its
    /// selected path. Structurally valid input remains hostile.
    ///
    /// # Errors
    /// Rejects source-envelope or private-staging failure without launching a
    /// parser or mutating profile state.
    pub fn prepare(
        staging: &ImportStagingRoot,
        source: NativeDocumentSource,
    ) -> Result<Self, DocumentImportError> {
        let expected = input_format(source.format);
        let inspection = inspect_source(&source.bytes, expected)?;
        let stage = staging.stage(source)?;
        if stage.byte_count() != inspection.byte_count || input_format(stage.format()) != expected {
            return Err(ImportStageError::InvalidSource.into());
        }
        Ok(Self { inspection, stage })
    }

    #[must_use]
    pub const fn operation_id(&self) -> Uuid {
        self.stage.operation_id()
    }

    #[must_use]
    pub const fn format(&self) -> InputFormat {
        self.inspection.format
    }

    #[must_use]
    pub const fn byte_count(&self) -> usize {
        self.inspection.byte_count
    }

    /// Removes the exact stage after all adapter-owned handles are closed.
    ///
    /// # Errors
    /// Returns a content-free failure if cleanup cannot be verified.
    pub fn cleanup(self) -> Result<(), DocumentImportError> {
        self.stage.cleanup().map_err(Into::into)
    }
}

/// Runs the prepared source only when the global import gate is enabled. The
/// current build always cleans the stage and returns `Disabled` before calling
/// a launcher. This function is present so future enablement cannot bypass the
/// exact launch/supervision/cleanup composition.
///
/// # Errors
/// Returns no extraction on a disabled gate, handle transfer, launch,
/// supervision, or private-stage cleanup failure. Stage cleanup failure
/// overrides an otherwise valid extraction.
pub fn extract_prepared<L: ContainedWorkerLauncher>(
    prepared: PreparedImport,
    launcher: &mut L,
    cancelled: &AtomicBool,
) -> Result<ValidatedExtraction, DocumentImportError> {
    extract_prepared_with_gate(prepared, launcher, cancelled, IMPORT_ENABLED)
}

fn extract_prepared_with_gate<L: ContainedWorkerLauncher>(
    mut prepared: PreparedImport,
    launcher: &mut L,
    cancelled: &AtomicBool,
    enabled: bool,
) -> Result<ValidatedExtraction, DocumentImportError> {
    if !enabled {
        prepared.cleanup()?;
        return Err(DocumentImportError::Disabled);
    }
    let operation_nonce = *prepared.operation_id().as_bytes();
    let format = prepared.format();
    let input = prepared.stage.take_input()?;
    let worker = launcher.launch(operation_nonce, format, input);
    let mut worker = match worker {
        Ok(worker) => worker,
        Err(error) => {
            prepared.cleanup()?;
            return Err(error.into());
        }
    };
    let extraction = supervise(&mut worker, format, cancelled);
    drop(worker);
    prepared.cleanup()?;
    extraction.map_err(Into::into)
}

impl std::fmt::Debug for PreparedImport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedImport")
            .field("operation_id", &self.operation_id())
            .field("format", &self.format())
            .field("byte_count", &self.byte_count())
            .finish_non_exhaustive()
    }
}

const fn input_format(format: NativeDocumentFormat) -> InputFormat {
    match format {
        NativeDocumentFormat::Pdf => InputFormat::Pdf,
        NativeDocumentFormat::Docx => InputFormat::Docx,
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::{collections::VecDeque, fs, io::Read, time::Duration};

    use ort_documents::worker_supervisor::{
        CleanupReceipt, CommonLaunchControls, ControlEvidence, LaunchReceipt, MacOsLaunchControls,
        NativeWorkerEvent, WorkerLimits,
    };
    use ort_platform::{IMPORTS_DIRECTORY, read_native_document};
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn prepares_the_exact_preflighted_pdf_as_a_private_handle() {
        let temporary = TempDir::new().unwrap();
        let selected = temporary.path().join("synthetic.pdf");
        let bytes = b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\n%%EOF\n";
        fs::write(&selected, bytes).unwrap();
        let source = read_native_document(&selected).unwrap();
        let staging = ImportStagingRoot::for_app_data(temporary.path()).unwrap();
        let prepared = PreparedImport::prepare(&staging, source).unwrap();
        assert_eq!(prepared.format(), InputFormat::Pdf);
        assert_eq!(prepared.byte_count(), bytes.len());
        let mut prepared = prepared;
        let mut input = prepared.stage.take_input().unwrap();
        let mut staged = Vec::new();
        input.read_to_end(&mut staged).unwrap();
        assert_eq!(staged, bytes);
        drop(input);
        prepared.cleanup().unwrap();
        assert_eq!(
            fs::read_dir(temporary.path().join(IMPORTS_DIRECTORY))
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn extension_only_claim_never_creates_a_stage() {
        let temporary = TempDir::new().unwrap();
        let selected = temporary.path().join("synthetic.pdf");
        fs::write(&selected, b"not a pdf").unwrap();
        let source = read_native_document(&selected).unwrap();
        let staging = ImportStagingRoot::for_app_data(temporary.path()).unwrap();
        assert!(matches!(
            PreparedImport::prepare(&staging, source),
            Err(DocumentImportError::Source(SourceError::FormatMismatch))
        ));
        assert_eq!(
            fs::read_dir(temporary.path().join(IMPORTS_DIRECTORY))
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn debug_output_never_contains_selected_content_or_path() {
        let temporary = TempDir::new().unwrap();
        let selected = temporary.path().join("PRIVATE_SELECTED_NAME.pdf");
        let bytes = b"%PDF-1.7\nPRIVATE_DOCUMENT_MARKER\n%%EOF\n";
        fs::write(&selected, bytes).unwrap();
        let staging = ImportStagingRoot::for_app_data(temporary.path()).unwrap();
        let prepared =
            PreparedImport::prepare(&staging, read_native_document(&selected).unwrap()).unwrap();
        let debug = format!("{prepared:?}");
        assert!(!debug.contains("PRIVATE_SELECTED_NAME"));
        assert!(!debug.contains("PRIVATE_DOCUMENT_MARKER"));
    }

    struct MockLauncher {
        calls: usize,
        fail: bool,
    }

    struct MockWorker {
        receipt: LaunchReceipt,
        operation_nonce: [u8; OPERATION_NONCE_BYTES],
        input: Option<File>,
        events: VecDeque<NativeWorkerEvent>,
    }

    impl ContainedWorkerLauncher for MockLauncher {
        type Worker = MockWorker;

        fn launch(
            &mut self,
            operation_nonce: [u8; OPERATION_NONCE_BYTES],
            format: InputFormat,
            mut input: File,
        ) -> Result<Self::Worker, NativeAdapterError> {
            self.calls += 1;
            assert_eq!(format, InputFormat::Pdf);
            if self.fail {
                return Err(NativeAdapterError::Unavailable);
            }
            let mut source = Vec::new();
            input.read_to_end(&mut source).unwrap();
            assert!(source.starts_with(b"%PDF-1.7"));
            let controls = CommonLaunchControls {
                fixed_verified_executable: ControlEvidence::Observed,
                minimal_environment: ControlEvidence::Observed,
                exact_descriptor_allowlist: ControlEvidence::Observed,
                read_only_input: ControlEvidence::Observed,
                private_output: ControlEvidence::Observed,
                network_denied: ControlEvidence::Observed,
                child_creation_denied: ControlEvidence::Observed,
                unrelated_files_denied: ControlEvidence::Observed,
                credentials_and_ipc_denied: ControlEvidence::Observed,
            };
            Ok(MockWorker {
                receipt: LaunchReceipt::macos(
                    operation_nonce,
                    controls,
                    MacOsLaunchControls {
                        helper_signature_verified: ControlEvidence::Observed,
                        app_sandbox_effective: ControlEvidence::Observed,
                        sandbox_inheritance_effective: ControlEvidence::Observed,
                        no_shared_entitlements: ControlEvidence::Observed,
                    },
                    WorkerLimits::production(),
                ),
                operation_nonce,
                input: Some(input),
                events: [
                    NativeWorkerEvent::Stdout(
                        br#"{"version":1,"format":"pdf","pageCount":1,"blocks":[{"page":1,"kind":"paragraph","text":"Synthetic extraction"}]}"#.to_vec(),
                    ),
                    NativeWorkerEvent::StdoutEof,
                    NativeWorkerEvent::StderrEof,
                    NativeWorkerEvent::Exited { code: Some(0) },
                ]
                .into(),
            })
        }
    }

    impl ContainedWorker for MockWorker {
        fn launch_receipt(&self) -> LaunchReceipt {
            self.receipt
        }

        fn receive(
            &mut self,
            _maximum_wait: Duration,
        ) -> Result<Option<NativeWorkerEvent>, NativeAdapterError> {
            Ok(self.events.pop_front())
        }

        fn terminate_tree(
            &mut self,
            _deadline: std::time::Instant,
        ) -> Result<(), NativeAdapterError> {
            Ok(())
        }

        fn cleanup(
            &mut self,
            _deadline: std::time::Instant,
        ) -> Result<CleanupReceipt, NativeAdapterError> {
            self.input.take();
            Ok(CleanupReceipt {
                operation_nonce: self.operation_nonce,
                worker_reaped: ControlEvidence::Observed,
                process_tree_empty: ControlEvidence::Observed,
                stdout_closed: ControlEvidence::Observed,
                stderr_closed: ControlEvidence::Observed,
                input_closed: ControlEvidence::Observed,
                output_closed: ControlEvidence::Observed,
                containment_closed: ControlEvidence::Observed,
            })
        }
    }

    fn prepared(temporary: &TempDir) -> PreparedImport {
        let selected = temporary.path().join("synthetic.pdf");
        fs::write(&selected, b"%PDF-1.7\nSynthetic\n%%EOF\n").unwrap();
        let staging = ImportStagingRoot::for_app_data(temporary.path()).unwrap();
        PreparedImport::prepare(&staging, read_native_document(&selected).unwrap()).unwrap()
    }

    #[test]
    fn disabled_public_path_cleans_without_calling_the_launcher() {
        let temporary = TempDir::new().unwrap();
        let mut launcher = MockLauncher {
            calls: 0,
            fail: false,
        };
        assert!(matches!(
            extract_prepared(prepared(&temporary), &mut launcher, &AtomicBool::new(false)),
            Err(DocumentImportError::Disabled)
        ));
        assert_eq!(launcher.calls, 0);
        assert_eq!(
            fs::read_dir(temporary.path().join(IMPORTS_DIRECTORY))
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn composed_internal_path_cleans_after_success_or_launch_failure() {
        let temporary = TempDir::new().unwrap();
        let mut launcher = MockLauncher {
            calls: 0,
            fail: false,
        };
        let extraction = extract_prepared_with_gate(
            prepared(&temporary),
            &mut launcher,
            &AtomicBool::new(false),
            true,
        )
        .unwrap();
        assert_eq!(extraction.blocks()[0].text, "Synthetic extraction");

        launcher.fail = true;
        assert!(matches!(
            extract_prepared_with_gate(
                prepared(&temporary),
                &mut launcher,
                &AtomicBool::new(false),
                true
            ),
            Err(DocumentImportError::Native(NativeAdapterError::Unavailable))
        ));
        assert_eq!(launcher.calls, 2);
        assert_eq!(
            fs::read_dir(temporary.path().join(IMPORTS_DIRECTORY))
                .unwrap()
                .count(),
            0
        );
    }
}
