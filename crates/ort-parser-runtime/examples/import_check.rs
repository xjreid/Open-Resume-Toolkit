//! Opt-in local compatibility check. Files are supplied explicitly; their
//! contents are never printed or copied into repository fixtures.
use ort_documents::{
    import::InputFormat, import_source::inspect_source, resume_import::map_resume,
};
use sha2::{Digest, Sha256};
fn main() {
    let paths: Vec<_> = std::env::args_os()
        .skip(1)
        .map(std::path::PathBuf::from)
        .collect();
    assert!(!paths.is_empty(), "supply PDF/DOCX paths");
    let pdf = std::fs::read("target/parser-guests/pdfium.wasm").unwrap();
    let docx = std::fs::read("target/parser-guests/docx.wasm").unwrap();
    let hash: [u8; 32] = Sha256::digest(&docx).into();
    for path in paths {
        let start = std::time::Instant::now();
        let bytes = std::fs::read(&path).unwrap();
        let format = match path
            .extension()
            .unwrap()
            .to_str()
            .unwrap()
            .to_lowercase()
            .as_str()
        {
            "pdf" => InputFormat::Pdf,
            "docx" => InputFormat::Docx,
            _ => panic!("PDF/DOCX only"),
        };
        inspect_source(&bytes, format).expect("source envelope");
        let extraction = match format {
            InputFormat::Pdf => ort_parser_runtime::extract_pdf(&pdf, &bytes),
            InputFormat::Docx => ort_parser_runtime::extract_docx(&docx, &hash, &bytes),
        }
        .expect("bounded parser extraction");
        let document = map_resume(&extraction).expect("editable review draft");
        // These test inputs are expected to be ready to map without exceeding
        // document limits. Invalid extracted drafts remain editable in the app.
        document
            .validate(ort_domain::DocumentLimits::default())
            .expect("mapped document validity");
        println!(
            "PASS {}: {} pages, {} blocks, {} sections, {} entries ({:.2}s)",
            path.file_name().unwrap().to_string_lossy(),
            extraction.page_count(),
            extraction.blocks().len(),
            document.sections.len(),
            document
                .sections
                .iter()
                .map(|section| section.entries.len())
                .sum::<usize>(),
            start.elapsed().as_secs_f64()
        );
    }
}
