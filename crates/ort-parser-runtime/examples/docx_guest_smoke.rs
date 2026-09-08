//! Explicit guest-artifact smoke; normal tests do not need a cross compiler.
use sha2::{Digest, Sha256};
fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm32-wasip1/release/ort-document-worker.wasm");
    let module = std::fs::read(path).expect("build the DOCX guest first");
    let hash: [u8; 32] = Sha256::digest(&module).into();
    // Development smoke uses the artifact it just built. Shipping must use a
    // separately bundled manifest hash, never compute expected hash from input.
    let mut resume = ort_domain::ResumeDocument::empty("Synthetic guest smoke");
    resume.contact.full_name = "Synthetic Person".into();
    let docx = ort_documents::render_docx(&resume).unwrap();
    let extraction = ort_parser_runtime::extract_docx(&module, &hash, &docx).expect("guest parse");
    assert!(
        extraction
            .blocks()
            .iter()
            .any(|block| block.text.contains("Synthetic Person"))
    );
    println!(
        "PASS: constrained DOCX guest extracted synthetic text through bounded memory-only WASI bindings"
    );
}
