//! Synthetic QA output only. Never reads user resumes or starts the application.
#[path = "../../ort-documents/tests/support/mod.rs"]
mod support;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .ok_or("provide a QA output directory")?;
    std::fs::create_dir_all(&directory)?;
    for kind in support::OUTPUT_FIXTURE_KINDS {
        let doc = support::fixture(kind);
        let artifact = ort_render::render_pdf(&doc)?;
        let root = std::path::Path::new(&directory);
        std::fs::write(root.join(format!("{kind}.pdf")), &artifact.bytes)?;
        std::fs::write(
            root.join(format!("{kind}.json")),
            serde_json::to_vec_pretty(&artifact.receipt)?,
        )?;
        std::fs::write(
            root.join(format!("{kind}.source.json")),
            serde_json::to_vec_pretty(&doc)?,
        )?;
        std::fs::write(
            root.join(format!("{kind}.txt")),
            ort_documents::render_plain_text(&doc).map_err(|_| "invalid synthetic fixture")?,
        )?;
        println!(
            "{kind}: {} pages, {} bytes, {}",
            artifact.receipt.page_count,
            artifact.bytes.len(),
            artifact.receipt.pdf_sha256
        );
    }
    Ok(())
}
