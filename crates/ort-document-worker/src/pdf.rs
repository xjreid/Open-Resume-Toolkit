//! Bounded PDF text extraction for the disposable parser worker.

use std::fs::File;
use std::io::{Read, Result as IoResult};
use std::path::Path;

use ort_documents::import::{
    BlockKind, InputFormat, MAX_BLOCK_CHARACTERS, MAX_EXTRACTED_CHARACTERS, MAX_PAGES, section_kind,
};
use ort_documents::import_source::{MAX_IMPORT_SOURCE_BYTES, SourceError, inspect_source};
use ort_documents::worker_output::{WorkerExtractionBuilder, WorkerOutputError};
use pdfium_render::prelude::{PdfPageObjectType, PdfPageObjectsCommon, Pdfium};
use sha2::{Digest, Sha256};

/// Maximum number of top-level page objects inspected before rejecting a PDF.
pub const MAX_PDF_PAGE_OBJECTS: usize = 20_000;
/// A page with an image/form and fewer readable characters is treated as scanned.
pub const MIN_PDF_TEXT_CHARACTERS_PER_IMAGE_PAGE: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PdfParseError {
    #[error("PDF input exceeds its configured limit")]
    InputLimit,
    #[error("PDF source is malformed or unsupported")]
    InvalidSource,
    #[error("PDF input could not be read")]
    InputRead,
    #[error("PDFium is unavailable on this platform")]
    UnsupportedPlatform,
    #[error("the pinned PDFium library is unavailable")]
    LibraryUnavailable,
    #[error("the PDFium library does not match the pinned release")]
    LibraryIdentity,
    #[error("PDFium could not parse the document")]
    InvalidPdf,
    #[error("PDF exceeds its configured page limit")]
    PageLimit,
    #[error("PDF exceeds its configured page complexity limit")]
    ComplexityLimit,
    #[error("PDF contains no readable text; OCR is not available")]
    NoReadableText,
    #[error("PDF contains an image-dominant page; OCR is not available")]
    PartiallyScanned,
    #[error("PDF extraction exceeds the worker protocol limit")]
    OutputLimit,
}

#[derive(Clone, Copy)]
struct PdfiumIdentity {
    filename: &'static str,
    byte_count: u64,
    sha256: &'static str,
}

// Binary digests are for the library inside the immutable chromium/7881
// non-V8 archives, not merely for the surrounding release archive.
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const PDFIUM_IDENTITY: Option<PdfiumIdentity> = Some(PdfiumIdentity {
    filename: "libpdfium.dylib",
    byte_count: 7_732_336,
    sha256: "1bc45b15466b34cef96641ce25c77a876e70010c6b114f909dda2f5325fc5bd7",
});

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const PDFIUM_IDENTITY: Option<PdfiumIdentity> = Some(PdfiumIdentity {
    filename: "libpdfium.dylib",
    byte_count: 7_471_800,
    sha256: "4eaad6c3e8d786cf6f66a45d7d014edf5c65f372f98c3070e66595ebb50e43d9",
});

#[cfg(all(target_os = "windows", target_arch = "aarch64"))]
const PDFIUM_IDENTITY: Option<PdfiumIdentity> = Some(PdfiumIdentity {
    filename: "pdfium.dll",
    byte_count: 6_658_048,
    sha256: "267a6f08a9c854d9949754a53b7630f23c0b67de5c7ca273b6abf178b49158c2",
});

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const PDFIUM_IDENTITY: Option<PdfiumIdentity> = Some(PdfiumIdentity {
    filename: "pdfium.dll",
    byte_count: 7_211_520,
    sha256: "79d4676b656cfb1abcea88f9ade3b4b0826c5200382db5f4ec72a636c598c118",
});

#[cfg(not(any(
    all(target_os = "macos", target_arch = "aarch64"),
    all(target_os = "macos", target_arch = "x86_64"),
    all(target_os = "windows", target_arch = "aarch64"),
    all(target_os = "windows", target_arch = "x86_64")
)))]
const PDFIUM_IDENTITY: Option<PdfiumIdentity> = None;

#[derive(Debug)]
struct PageSnapshot {
    text: String,
    object_count: usize,
    image_like_count: usize,
}

trait PdfTextEngine {
    fn pages(&self, bytes: &[u8]) -> Result<Vec<PageSnapshot>, PdfParseError>;
}

struct PdfiumEngine {
    pdfium: Pdfium,
}

impl PdfTextEngine for PdfiumEngine {
    fn pages(&self, bytes: &[u8]) -> Result<Vec<PageSnapshot>, PdfParseError> {
        let document = self
            .pdfium
            .load_pdf_from_byte_slice(bytes, None)
            .map_err(|_| PdfParseError::InvalidPdf)?;
        let count = document.pages().len();
        let page_count = u16::try_from(count).map_err(|_| PdfParseError::PageLimit)?;
        if page_count == 0 || page_count > MAX_PAGES {
            return Err(PdfParseError::PageLimit);
        }

        let mut pages = Vec::new();
        pages
            .try_reserve(usize::from(page_count))
            .map_err(|_| PdfParseError::OutputLimit)?;
        for index in 0..count {
            let page = document
                .pages()
                .get(index)
                .map_err(|_| PdfParseError::InvalidPdf)?;
            let objects = page.objects();
            let object_count = objects.len();
            if object_count > MAX_PDF_PAGE_OBJECTS {
                return Err(PdfParseError::ComplexityLimit);
            }
            let mut image_like_count = 0_usize;
            for object_index in 0..object_count {
                let object = objects
                    .get(object_index)
                    .map_err(|_| PdfParseError::InvalidPdf)?;
                if matches!(
                    object.object_type(),
                    PdfPageObjectType::Image | PdfPageObjectType::XObjectForm
                ) {
                    image_like_count = image_like_count
                        .checked_add(1)
                        .ok_or(PdfParseError::ComplexityLimit)?;
                }
            }
            let text = page.text().map_err(|_| PdfParseError::InvalidPdf)?.all();
            if text.chars().count() > MAX_EXTRACTED_CHARACTERS {
                return Err(PdfParseError::OutputLimit);
            }
            pages.push(PageSnapshot {
                text,
                object_count,
                image_like_count,
            });
        }
        Ok(pages)
    }
}

/// Verifies and loads only the pinned `PDFium` library, then extracts one
/// already-open PDF handle into extraction wire v1. No system-library fallback,
/// path discovery, password attempt, rendering, JavaScript, XFA, or OCR occurs.
///
/// The disposable worker contract permits exactly one call per process because
/// `PDFium` uses process-global bindings. Import remains disabled until native
/// containment and signed-package verification gates pass.
///
/// # Errors
/// Fails closed on source, library identity, `PDFium`, page/object, scanned-text,
/// and output limits. No partial extraction is returned.
pub fn extract_pdf(
    input: &mut impl Read,
    pinned_pdfium_library: &Path,
) -> Result<Vec<u8>, PdfParseError> {
    verify_pdfium_library(pinned_pdfium_library)?;
    let bindings = Pdfium::bind_to_library(pinned_pdfium_library)
        .map_err(|_| PdfParseError::LibraryUnavailable)?;
    let engine = PdfiumEngine {
        pdfium: Pdfium::new(bindings),
    };
    extract_with_engine(input, &engine)
}

fn extract_with_engine(
    input: &mut impl Read,
    engine: &impl PdfTextEngine,
) -> Result<Vec<u8>, PdfParseError> {
    let mut bytes = Vec::new();
    input
        .take(u64::try_from(MAX_IMPORT_SOURCE_BYTES).unwrap_or(u64::MAX) + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PdfParseError::InputRead)?;
    if bytes.is_empty() || bytes.len() > MAX_IMPORT_SOURCE_BYTES {
        return Err(PdfParseError::InputLimit);
    }
    inspect_source(&bytes, InputFormat::Pdf).map_err(map_source_error)?;
    encode_pages(engine.pages(&bytes)?)
}

fn encode_pages(pages: Vec<PageSnapshot>) -> Result<Vec<u8>, PdfParseError> {
    let page_count = u16::try_from(pages.len()).map_err(|_| PdfParseError::PageLimit)?;
    if page_count == 0 || page_count > MAX_PAGES {
        return Err(PdfParseError::PageLimit);
    }

    let mut total_readable = 0_usize;
    let mut image_dominant_page = false;
    for page in &pages {
        if page.object_count > MAX_PDF_PAGE_OBJECTS || page.image_like_count > page.object_count {
            return Err(PdfParseError::ComplexityLimit);
        }
        let readable = readable_characters(&page.text);
        total_readable = total_readable
            .checked_add(readable)
            .filter(|count| *count <= MAX_EXTRACTED_CHARACTERS)
            .ok_or(PdfParseError::OutputLimit)?;
        image_dominant_page |=
            page.image_like_count > 0 && readable < MIN_PDF_TEXT_CHARACTERS_PER_IMAGE_PAGE;
    }
    if total_readable == 0 {
        return Err(PdfParseError::NoReadableText);
    }
    if image_dominant_page {
        return Err(PdfParseError::PartiallyScanned);
    }

    let mut builder =
        WorkerExtractionBuilder::new(InputFormat::Pdf, page_count).map_err(map_output_error)?;
    for (index, page) in pages.into_iter().enumerate() {
        let page_number = u16::try_from(index + 1).map_err(|_| PdfParseError::PageLimit)?;
        let normalized = page.text.replace("\r\n", "\n").replace('\r', "\n");
        for line in normalized.split('\n') {
            if line.chars().count() > MAX_BLOCK_CHARACTERS {
                return Err(PdfParseError::OutputLimit);
            }
            builder
                .push(page_number, classify_line(line), line.to_owned())
                .map_err(map_output_error)?;
        }
    }
    builder.finish().map_err(map_output_error)
}

fn classify_line(line: &str) -> BlockKind {
    let trimmed = line.trim();
    if section_kind(trimmed).is_some() {
        BlockKind::Heading
    } else if trimmed.starts_with("- ") || trimmed.starts_with("• ") || trimmed.starts_with("* ")
    {
        BlockKind::ListItem
    } else {
        BlockKind::Paragraph
    }
}

fn readable_characters(text: &str) -> usize {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .count()
}

fn verify_pdfium_library(path: &Path) -> Result<(), PdfParseError> {
    let identity = PDFIUM_IDENTITY.ok_or(PdfParseError::UnsupportedPlatform)?;
    if !path.is_absolute()
        || path.file_name().and_then(|name| name.to_str()) != Some(identity.filename)
    {
        return Err(PdfParseError::LibraryIdentity);
    }
    let metadata = path
        .symlink_metadata()
        .map_err(|_| PdfParseError::LibraryUnavailable)?;
    if !metadata.file_type().is_file() || metadata.len() != identity.byte_count {
        return Err(PdfParseError::LibraryIdentity);
    }
    let mut file = File::open(path).map_err(|_| PdfParseError::LibraryUnavailable)?;
    let mut hasher = Sha256::new();
    copy_into_digest(&mut file, &mut hasher).map_err(|_| PdfParseError::LibraryUnavailable)?;
    let actual = hex::encode(hasher.finalize());
    if actual != identity.sha256 {
        return Err(PdfParseError::LibraryIdentity);
    }
    Ok(())
}

fn copy_into_digest(reader: &mut impl Read, digest: &mut Sha256) -> IoResult<()> {
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            return Ok(());
        }
        digest.update(&buffer[..read]);
    }
}

const fn map_source_error(error: SourceError) -> PdfParseError {
    match error {
        SourceError::LimitExceeded => PdfParseError::InputLimit,
        SourceError::FormatMismatch
        | SourceError::InvalidContainer
        | SourceError::UnsafePath
        | SourceError::ActiveContent
        | SourceError::ExpansionLimit => PdfParseError::InvalidSource,
    }
}

const fn map_output_error(error: WorkerOutputError) -> PdfParseError {
    match error {
        WorkerOutputError::NoReadableText => PdfParseError::NoReadableText,
        WorkerOutputError::LimitExceeded
        | WorkerOutputError::InvalidBlock
        | WorkerOutputError::Encoding
        | WorkerOutputError::Allocation => PdfParseError::OutputLimit,
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;
    use ort_documents::import::ValidatedExtraction;

    struct FakeEngine {
        pages: Vec<PageSnapshot>,
        failure: Option<PdfParseError>,
    }

    impl PdfTextEngine for FakeEngine {
        fn pages(&self, _bytes: &[u8]) -> Result<Vec<PageSnapshot>, PdfParseError> {
            self.failure.map_or_else(
                || {
                    Ok(self
                        .pages
                        .iter()
                        .map(|page| PageSnapshot {
                            text: page.text.clone(),
                            object_count: page.object_count,
                            image_like_count: page.image_like_count,
                        })
                        .collect())
                },
                Err,
            )
        }
    }

    fn valid_pdf() -> Vec<u8> {
        b"%PDF-1.7\n1 0 obj\n<<>>\nendobj\n%%EOF\n".to_vec()
    }

    fn engine(pages: Vec<PageSnapshot>) -> FakeEngine {
        FakeEngine {
            pages,
            failure: None,
        }
    }

    fn page(text: &str) -> PageSnapshot {
        PageSnapshot {
            text: text.to_owned(),
            object_count: 2,
            image_like_count: 0,
        }
    }

    #[test]
    fn maps_lines_pages_heading_list_unicode_and_line_endings() {
        let parser = engine(vec![
            page("Résumé\r\nExperience\r- Built safely"),
            page("Skills\n• Rust\n工程"),
        ]);
        let wire = extract_with_engine(&mut valid_pdf().as_slice(), &parser).unwrap();
        let extraction = ValidatedExtraction::decode(&wire, InputFormat::Pdf).unwrap();
        assert_eq!(extraction.page_count(), 2);
        assert_eq!(extraction.blocks().len(), 6);
        assert_eq!(extraction.blocks()[1].kind, BlockKind::Heading);
        assert_eq!(extraction.blocks()[2].kind, BlockKind::ListItem);
        assert_eq!(extraction.blocks()[3].page, 2);
        assert_eq!(extraction.blocks()[4].kind, BlockKind::ListItem);
        assert_eq!(extraction.blocks()[5].text, "工程");
    }

    #[test]
    fn rejects_image_only_and_partially_scanned_documents() {
        let mut image_only = page(" \n\t");
        image_only.image_like_count = 1;
        assert_eq!(
            extract_with_engine(&mut valid_pdf().as_slice(), &engine(vec![image_only])),
            Err(PdfParseError::NoReadableText)
        );

        let readable = page("Experience\nSubstantial readable employment history");
        let mut scan = page("page 2");
        scan.image_like_count = 1;
        assert_eq!(
            extract_with_engine(&mut valid_pdf().as_slice(), &engine(vec![readable, scan])),
            Err(PdfParseError::PartiallyScanned)
        );
    }

    #[test]
    fn ordinary_logo_does_not_trigger_scanned_detection() {
        let mut page = page("Résumé Example\nExperienced systems engineer");
        page.image_like_count = 1;
        assert!(extract_with_engine(&mut valid_pdf().as_slice(), &engine(vec![page])).is_ok());
    }

    #[test]
    fn rejects_page_object_text_and_protocol_limit_attacks() {
        let mut complex = page("readable");
        complex.object_count = MAX_PDF_PAGE_OBJECTS + 1;
        assert_eq!(
            encode_pages(vec![complex]),
            Err(PdfParseError::ComplexityLimit)
        );

        let too_many_pages = (0..=MAX_PAGES).map(|_| page("readable")).collect();
        assert_eq!(encode_pages(too_many_pages), Err(PdfParseError::PageLimit));

        let huge = page(&"x".repeat(MAX_BLOCK_CHARACTERS + 1));
        assert_eq!(encode_pages(vec![huge]), Err(PdfParseError::OutputLimit));

        let inverted = PageSnapshot {
            text: "readable".into(),
            object_count: 1,
            image_like_count: 2,
        };
        assert_eq!(
            encode_pages(vec![inverted]),
            Err(PdfParseError::ComplexityLimit)
        );
    }

    #[test]
    fn source_and_engine_failures_do_not_return_partial_output() {
        let parser = engine(vec![page("readable")]);
        assert_eq!(
            extract_with_engine(&mut b"not a PDF".as_slice(), &parser),
            Err(PdfParseError::InvalidSource)
        );
        let failing = FakeEngine {
            pages: Vec::new(),
            failure: Some(PdfParseError::InvalidPdf),
        };
        assert_eq!(
            extract_with_engine(&mut valid_pdf().as_slice(), &failing),
            Err(PdfParseError::InvalidPdf)
        );
    }

    #[test]
    fn read_and_input_limits_are_enforced_before_parser_entry() {
        struct BrokenReader;
        impl Read for BrokenReader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("synthetic"))
            }
        }

        let parser = engine(vec![page("readable")]);
        assert_eq!(
            extract_with_engine(&mut BrokenReader, &parser),
            Err(PdfParseError::InputRead)
        );
        let mut oversized = vec![0_u8; MAX_IMPORT_SOURCE_BYTES + 1];
        oversized[..8].copy_from_slice(b"%PDF-1.7");
        assert_eq!(
            extract_with_engine(&mut oversized.as_slice(), &parser),
            Err(PdfParseError::InputLimit)
        );
    }

    #[cfg(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64")
    ))]
    #[test]
    fn library_verifier_rejects_relative_wrong_named_symlink_and_wrong_digest() {
        let identity = PDFIUM_IDENTITY.unwrap();
        assert_eq!(
            verify_pdfium_library(Path::new(identity.filename)),
            Err(PdfParseError::LibraryIdentity)
        );

        let wrong_name = tempfile::NamedTempFile::new().unwrap();
        assert_eq!(
            verify_pdfium_library(wrong_name.path()),
            Err(PdfParseError::LibraryIdentity)
        );

        let directory = tempfile::tempdir().unwrap();
        let library = directory.path().join(identity.filename);
        let byte_count = usize::try_from(identity.byte_count).unwrap();
        std::fs::write(&library, vec![0_u8; byte_count]).unwrap();
        assert_eq!(
            verify_pdfium_library(&library),
            Err(PdfParseError::LibraryIdentity)
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link_directory = tempfile::tempdir().unwrap();
            let link = link_directory.path().join(identity.filename);
            symlink(&library, &link).unwrap();
            assert_eq!(
                verify_pdfium_library(&link),
                Err(PdfParseError::LibraryIdentity)
            );
        }
    }

    #[cfg(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64")
    ))]
    #[test]
    fn compiled_library_identity_is_present_in_release_manifest() {
        let identity = PDFIUM_IDENTITY.unwrap();
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../pdfium-manifest.json")).unwrap();
        assert_eq!(manifest["apiBuild"], 7881);
        assert_eq!(manifest["configuration"]["v8"], false);
        assert_eq!(manifest["configuration"]["xfa"], false);
        assert_eq!(manifest["configuration"]["systemLibraryFallback"], false);
        let assets = manifest["assets"].as_array().unwrap();
        assert!(assets.iter().any(|asset| {
            asset["libraryBytes"].as_u64() == Some(identity.byte_count)
                && asset["librarySha256"].as_str() == Some(identity.sha256)
                && asset["libraryPath"]
                    .as_str()
                    .is_some_and(|path| path.ends_with(identity.filename))
        }));
    }

    #[test]
    fn diagnostics_do_not_include_document_text_or_library_path() {
        let sensitive = "PRIVATE_PDF_TEXT";
        let result = encode_pages(vec![page(sensitive)]);
        assert!(!format!("{result:?}").contains(sensitive));
        assert!(!format!("{:?}", PdfParseError::LibraryIdentity).contains("/Users/"));
    }
}
