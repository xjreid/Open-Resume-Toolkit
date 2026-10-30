//! Parent-side Windows import boundary; source bytes travel only over private pipes.
use ort_documents::{
    import::{InputFormat, ValidatedExtraction},
    import_source::inspect_source,
};
use ort_windows::{ContainedParser, ParserError};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

/// Packaging must opt into this reviewed policy together with the helper's hash.
pub const WINDOWS_PARSER_POLICY: &str = "appcontainer-job-wasm-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ParserProcessError {
    #[error("the bundled parser helper is unavailable or unverified")]
    Identity,
    #[error("the document could not be imported")]
    Failed,
    #[error("document import was cancelled or exceeded its deadline")]
    Cancelled,
    #[error("parser cleanup could not be confirmed")]
    Cleanup,
}

/// Only trusted native composition may choose an executable and compiled digest.
pub struct ParserHelper {
    executable: PathBuf,
    sha256: [u8; 32],
}
impl ParserHelper {
    /// # Errors
    /// Rejects relative or traversing package paths. Per-job identity and native
    /// containment are verified again before any document bytes are delivered.
    pub fn new(executable: PathBuf, sha256: [u8; 32]) -> Result<Self, ParserProcessError> {
        if !executable.is_absolute()
            || executable
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(ParserProcessError::Identity);
        }
        Ok(Self { executable, sha256 })
    }

    /// Runs one zero-capability process with the same metered guests as macOS.
    /// # Errors
    /// Returns no extraction unless the verified process succeeds, both output
    /// streams close, the Job is empty, and native cleanup is confirmed.
    pub fn extract(
        &self,
        format: InputFormat,
        input: &[u8],
        cancel: &AtomicBool,
    ) -> Result<ValidatedExtraction, ParserProcessError> {
        let end = Instant::now() + Duration::from_secs(60);
        inspect_source(input, format).map_err(|_| ParserProcessError::Failed)?;
        let mut request = Vec::new();
        request
            .try_reserve_exact(9 + input.len())
            .map_err(|_| ParserProcessError::Failed)?;
        request.extend_from_slice(b"ORTW");
        request.push(match format {
            InputFormat::Docx => 1,
            InputFormat::Pdf => 2,
        });
        let length = u32::try_from(input.len()).map_err(|_| ParserProcessError::Failed)?;
        request.extend_from_slice(&length.to_le_bytes());
        request.extend_from_slice(input);
        let output =
            ContainedParser::exchange(&self.executable, &self.sha256, &request, end, cancel)
                .map_err(|error| match error {
                    ParserError::Identity | ParserError::Containment => {
                        ParserProcessError::Identity
                    }
                    ParserError::Io => ParserProcessError::Failed,
                    ParserError::Cancelled | ParserError::Timeout => ParserProcessError::Cancelled,
                    ParserError::Cleanup => ParserProcessError::Cleanup,
                })?;
        let extraction =
            ValidatedExtraction::decode(&output, format).map_err(|_| ParserProcessError::Failed)?;
        if cancel.load(Ordering::Acquire) || Instant::now() >= end {
            return Err(ParserProcessError::Cancelled);
        }
        Ok(extraction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn helpers_require_fixed_absolute_package_paths() {
        for path in ["helper.exe", r"C:\package\..\helper.exe"] {
            assert!(matches!(
                ParserHelper::new(path.into(), [0; 32]),
                Err(ParserProcessError::Identity)
            ));
        }
    }
    #[test]
    fn invalid_sources_are_rejected_before_native_launch() {
        let helper = ParserHelper::new(r"C:\missing\helper.exe".into(), [0; 32]).unwrap();
        assert_eq!(
            helper
                .extract(InputFormat::Pdf, b"invalid", &AtomicBool::new(false))
                .err(),
            Some(ParserProcessError::Failed)
        );
    }
}
