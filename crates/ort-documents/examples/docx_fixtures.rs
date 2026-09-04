//! Writes only built-in synthetic fixtures into a newly created directory.
#[path = "../tests/support/mod.rs"]
mod support;
use ort_documents::render_docx;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let root = PathBuf::from(args.next().ok_or("supply a new output directory")?);
    if args.next().is_some() {
        return Err("only one output directory is accepted".into());
    }
    fs::create_dir(&root)?;
    for name in support::OUTPUT_FIXTURE_KINDS {
        let document = support::fixture(name);
        let bytes = render_docx(&document).map_err(|_| "synthetic render failed")?;
        for (extension, bytes) in [
            ("docx", bytes),
            ("json", serde_json::to_vec_pretty(&document)?),
            (
                "txt",
                ort_documents::render_plain_text(&document)
                    .map_err(|_| "synthetic text render failed")?
                    .into_bytes(),
            ),
        ] {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(root.join(format!("{name}.{extension}")))?;
            file.write_all(&bytes)?;
        }
    }
    println!(
        "Eight synthetic DOCX/source/text fixtures written; no user profile or vault accessed."
    );
    Ok(())
}
