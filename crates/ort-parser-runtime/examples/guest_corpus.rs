//! Explicit local corpus check; consumes only the fixture directory supplied.
use sha2::{Digest, Sha256};
fn main() {
    let root = std::env::args_os()
        .nth(1)
        .expect("synthetic fixture directory");
    let pdf = std::fs::read("target/wasm-containment/pdfium.wasm").unwrap();
    let docx = std::fs::read("target/wasm32-wasip1/release/ort-document-worker.wasm").unwrap();
    let hash: [u8; 32] = Sha256::digest(&docx).into();
    let mut paths = vec![];
    collect(std::path::Path::new(&root), &mut paths);
    paths.sort();
    let mut passed = 0;
    let mut failed = 0;
    for path in paths {
        let source = std::fs::read(&path).unwrap();
        let result = if path.extension().unwrap() == "pdf" {
            ort_parser_runtime::extract_pdf(&pdf, &source)
        } else {
            ort_parser_runtime::extract_docx(&docx, &hash, &source)
        };
        match result {
            Ok(value) => {
                verify_content(&path, &value);
                passed += 1;
                println!(
                    "PASS {}: {} pages, {} blocks",
                    path.display(),
                    value.page_count(),
                    value.blocks().len()
                );
            }
            Err(error) => {
                failed += 1;
                println!("FAIL {}: {error}", path.display());
            }
        }
    }
    println!("{passed} passed; {failed} failed");
    assert!(passed > 0);
    assert_eq!(failed, 0);
}
fn collect(root: &std::path::Path, paths: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        let path = entry.path();
        if kind.is_dir() {
            collect(&path, paths);
        } else if kind.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext == "pdf" || ext == "docx")
        {
            paths.push(path);
        }
    }
}

fn folded(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect()
}

fn verify_content(path: &std::path::Path, value: &ort_documents::import::ValidatedExtraction) {
    let source_path = if path.extension().unwrap() == "docx" {
        path.parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("pdf")
            .join(path.file_name().unwrap())
            .with_extension("source.json")
    } else {
        path.with_extension("source.json")
    };
    let original: ort_domain::ResumeDocument =
        serde_json::from_slice(&std::fs::read(source_path).unwrap()).unwrap();
    let extracted = folded(
        &value
            .blocks()
            .iter()
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
    );
    let mut expected = vec![
        original.contact.full_name.as_str(),
        original.contact.email.as_str(),
        original.contact.phone.as_str(),
        original.contact.location.as_str(),
    ];
    for section in &original.sections {
        if !section.entries.is_empty() {
            expected.push(&section.heading);
        }
        for entry in &section.entries {
            expected.extend([
                entry.heading.as_str(),
                entry.subheading.as_str(),
                entry.date_range.as_str(),
                entry.location.as_str(),
            ]);
            expected.extend(entry.fields.iter().map(|field| field.value.as_str()));
            expected.extend(entry.bullets.iter().map(|bullet| bullet.text.as_str()));
        }
    }
    assert!(
        expected
            .iter()
            .all(|value| extracted.contains(&folded(value))),
        "missing synthetic content in {}",
        path.display()
    );
    let mut ordered = vec![];
    for section in &original.sections {
        if section.entries.is_empty() {
            continue;
        }
        ordered.push(section.heading.as_str());
        for entry in &section.entries {
            ordered.push(entry.heading.as_str());
            ordered.extend(entry.bullets.iter().map(|bullet| bullet.text.as_str()));
        }
    }
    let mut cursor = 0;
    for item in ordered {
        let item = folded(item);
        if item.is_empty() {
            continue;
        }
        let offset = extracted[cursor..]
            .find(&item)
            .expect("synthetic heading/bullet order preserved");
        cursor += offset + item.len();
    }
}
