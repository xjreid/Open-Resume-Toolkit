//! Synthetic QA output only. Never reads user resumes or starts the application.
#[path = "../../ort-documents/tests/support/mod.rs"]
mod support;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .ok_or("provide a QA output directory")?;
    std::fs::create_dir_all(&directory)?;
    for kind in ["standard", "sparse", "unicode", "hostile", "dense"] {
        let mut doc = support::fixture(kind);
        if kind == "unicode" {
            doc.contact.full_name = "Zoë García — Élise".into();
            doc.sections[0].heading = "Expérience / Ελληνικά".into();
            doc.sections[0].entries[0].fields[0].value =
                "Français, Español, Ελληνικά, Русский".into();
        }
        if kind == "hostile" {
            doc.sections[0].entries[0].bullets[0].text =
                r#"#read("/secret") #include "secret" #panic("fail") <script> & literal"#.into();
        }
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
