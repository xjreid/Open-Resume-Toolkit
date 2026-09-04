//! Bounded extraction-message construction for sandboxed parser adapters.
//!
//! A parser supplies already extracted text and structural hints here. This
//! module does not open or parse a PDF/DOCX and grants no process, path, network,
//! storage, or IPC authority.

use serde::Serialize;

use crate::import::{
    BlockKind, EXTRACTION_VERSION, ExtractedBlock, ImportError, InputFormat, MAX_BLOCK_CHARACTERS,
    MAX_BLOCKS, MAX_EXTRACTED_CHARACTERS, MAX_EXTRACTION_BYTES, MAX_PAGES, ValidatedExtraction,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WorkerOutputError {
    #[error("parser extraction exceeds its configured limit")]
    LimitExceeded,
    #[error("parser extraction block ordering or content is invalid")]
    InvalidBlock,
    #[error("no readable text was extracted; OCR is not available")]
    NoReadableText,
    #[error("parser extraction could not be encoded")]
    Encoding,
    #[error("parser extraction allocation failed")]
    Allocation,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExtractionWire<'a> {
    version: u16,
    format: InputFormat,
    page_count: u16,
    blocks: &'a [ExtractedBlock],
}

/// Single-use builder shared by future `PDFium` and constrained DOCX adapters.
/// Debug output reports counts only and never includes extracted text.
pub struct WorkerExtractionBuilder {
    format: InputFormat,
    page_count: u16,
    blocks: Vec<ExtractedBlock>,
    character_count: usize,
    last_page: u16,
    readable: bool,
}

impl WorkerExtractionBuilder {
    /// Starts one extraction with a parser-observed page count.
    ///
    /// # Errors
    /// Rejects zero pages and values above the fixed parent protocol limit.
    pub fn new(format: InputFormat, page_count: u16) -> Result<Self, WorkerOutputError> {
        if page_count == 0 || page_count > MAX_PAGES {
            return Err(WorkerOutputError::LimitExceeded);
        }
        Ok(Self {
            format,
            page_count,
            blocks: Vec::new(),
            character_count: 0,
            last_page: 1,
            readable: false,
        })
    }

    /// Adds one parser-observed block without trimming or interpreting its text.
    ///
    /// # Errors
    /// Rejects invalid/out-of-order pages, block/character overflow, allocation
    /// failure, and unsupported control characters before retaining the block.
    pub fn push(
        &mut self,
        page: u16,
        kind: BlockKind,
        text: String,
    ) -> Result<(), WorkerOutputError> {
        if page < self.last_page || page == 0 || page > self.page_count {
            return Err(WorkerOutputError::InvalidBlock);
        }
        if self.blocks.len() >= MAX_BLOCKS {
            return Err(WorkerOutputError::LimitExceeded);
        }
        let characters = text.chars().count();
        let total = self
            .character_count
            .checked_add(characters)
            .ok_or(WorkerOutputError::LimitExceeded)?;
        if characters > MAX_BLOCK_CHARACTERS || total > MAX_EXTRACTED_CHARACTERS {
            return Err(WorkerOutputError::LimitExceeded);
        }
        if text
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        {
            return Err(WorkerOutputError::InvalidBlock);
        }
        self.blocks
            .try_reserve(1)
            .map_err(|_| WorkerOutputError::Allocation)?;
        self.readable |= text.chars().any(|character| !character.is_whitespace());
        self.character_count = total;
        self.last_page = page;
        self.blocks.push(ExtractedBlock { page, kind, text });
        Ok(())
    }

    /// Encodes exactly one bounded protocol message and re-runs the independent
    /// parent decoder as a local consistency check.
    ///
    /// # Errors
    /// Rejects empty/whitespace-only extraction, encoding overflow, or any
    /// producer/consumer protocol disagreement.
    pub fn finish(self) -> Result<Vec<u8>, WorkerOutputError> {
        if !self.readable {
            return Err(WorkerOutputError::NoReadableText);
        }
        let wire = ExtractionWire {
            version: EXTRACTION_VERSION,
            format: self.format,
            page_count: self.page_count,
            blocks: &self.blocks,
        };
        let bytes = serde_json::to_vec(&wire).map_err(|_| WorkerOutputError::Encoding)?;
        if bytes.len() > MAX_EXTRACTION_BYTES {
            return Err(WorkerOutputError::LimitExceeded);
        }
        match ValidatedExtraction::decode(&bytes, self.format) {
            Ok(_) => Ok(bytes),
            Err(ImportError::NoReadableText) => Err(WorkerOutputError::NoReadableText),
            Err(ImportError::LimitExceeded) => Err(WorkerOutputError::LimitExceeded),
            Err(ImportError::InvalidExtraction | ImportError::UnsupportedControl) => {
                Err(WorkerOutputError::Encoding)
            }
        }
    }
}

impl std::fmt::Debug for WorkerExtractionBuilder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WorkerExtractionBuilder")
            .field("format", &self.format)
            .field("page_count", &self.page_count)
            .field("block_count", &self.blocks.len())
            .field("character_count", &self.character_count)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_parser_order_kind_pages_and_text() {
        let mut builder = WorkerExtractionBuilder::new(InputFormat::Pdf, 2).unwrap();
        builder
            .push(1, BlockKind::Heading, "Résumé\n概要".into())
            .unwrap();
        builder
            .push(2, BlockKind::ListItem, "• Exact parser text".into())
            .unwrap();
        let bytes = builder.finish().unwrap();
        let extraction = ValidatedExtraction::decode(&bytes, InputFormat::Pdf).unwrap();
        assert_eq!(extraction.page_count(), 2);
        assert_eq!(extraction.blocks()[0].kind, BlockKind::Heading);
        assert_eq!(extraction.blocks()[0].text, "Résumé\n概要");
        assert_eq!(extraction.blocks()[1].page, 2);
        assert_eq!(extraction.blocks()[1].text, "• Exact parser text");
    }

    #[test]
    fn page_order_controls_and_empty_output_fail_before_encoding() {
        assert!(WorkerExtractionBuilder::new(InputFormat::Pdf, 0).is_err());
        assert!(WorkerExtractionBuilder::new(InputFormat::Pdf, MAX_PAGES + 1).is_err());
        let mut builder = WorkerExtractionBuilder::new(InputFormat::Docx, 2).unwrap();
        builder
            .push(2, BlockKind::Paragraph, "later".into())
            .unwrap();
        assert_eq!(
            builder.push(1, BlockKind::Paragraph, "earlier".into()),
            Err(WorkerOutputError::InvalidBlock)
        );
        assert_eq!(
            builder.push(2, BlockKind::Paragraph, "bad\0control".into()),
            Err(WorkerOutputError::InvalidBlock)
        );

        let mut empty = WorkerExtractionBuilder::new(InputFormat::Pdf, 1).unwrap();
        empty.push(1, BlockKind::Paragraph, " \n\t".into()).unwrap();
        assert_eq!(empty.finish(), Err(WorkerOutputError::NoReadableText));
    }

    #[test]
    fn block_and_aggregate_limits_are_checked_before_retention() {
        let mut oversized = WorkerExtractionBuilder::new(InputFormat::Pdf, 1).unwrap();
        assert_eq!(
            oversized.push(
                1,
                BlockKind::Paragraph,
                "x".repeat(MAX_BLOCK_CHARACTERS + 1)
            ),
            Err(WorkerOutputError::LimitExceeded)
        );

        let mut aggregate = WorkerExtractionBuilder::new(InputFormat::Docx, 1).unwrap();
        for _ in 0..MAX_BLOCKS {
            aggregate.push(1, BlockKind::Paragraph, "x".into()).unwrap();
        }
        assert_eq!(
            aggregate.push(1, BlockKind::Paragraph, "extra".into()),
            Err(WorkerOutputError::LimitExceeded)
        );

        let mut characters = WorkerExtractionBuilder::new(InputFormat::Pdf, 1).unwrap();
        characters
            .push(1, BlockKind::Paragraph, "x".repeat(25_000))
            .unwrap();
        characters
            .push(1, BlockKind::Paragraph, "y".repeat(25_000))
            .unwrap();
        assert_eq!(
            characters.push(1, BlockKind::Paragraph, "z".into()),
            Err(WorkerOutputError::LimitExceeded)
        );
    }

    #[test]
    fn debug_never_contains_extracted_text() {
        let mut builder = WorkerExtractionBuilder::new(InputFormat::Pdf, 1).unwrap();
        builder
            .push(1, BlockKind::Paragraph, "PRIVATE_PARSER_TEXT".into())
            .unwrap();
        assert!(!format!("{builder:?}").contains("PRIVATE_PARSER_TEXT"));
    }
}
