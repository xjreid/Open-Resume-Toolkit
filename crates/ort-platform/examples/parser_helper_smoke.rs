#[cfg(target_os = "macos")]
fn main() {
    use ort_documents::import::InputFormat;
    use ort_platform::ParserHelper;
    use sha2::{Digest, Sha256};
    use std::sync::atomic::AtomicBool;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let executable = std::env::args_os().nth(1).map_or_else(
        || root.join("target/ORT Parser Helper.app/Contents/MacOS/ort-parser-helper"),
        std::path::PathBuf::from,
    );
    // Qualification only: production obtains this digest from its compiled
    // packaging manifest, not from an executable selected by the renderer.
    let hash: [u8; 32] = Sha256::digest(std::fs::read(&executable).unwrap()).into();
    let details = std::process::Command::new("/usr/bin/codesign")
        .args(["-d", "--verbose=4"])
        .arg(&executable)
        .output()
        .unwrap();
    assert!(details.status.success());
    let details = String::from_utf8(details.stderr).unwrap();
    let cdhash = details
        .lines()
        .find_map(|line| line.strip_prefix("CDHash="))
        .unwrap()
        .to_owned();
    let helper = ParserHelper::new(executable.clone(), hash, cdhash.clone()).unwrap();
    let cancelled = AtomicBool::new(true);
    let input =
        std::fs::read(root.join("target/m2-style-parity-repeat-v1/technical/pdf/standard.pdf"))
            .unwrap();
    assert!(
        helper
            .extract(InputFormat::Pdf, &input, &cancelled)
            .is_err()
    );
    for (format, path) in [
        (
            InputFormat::Pdf,
            "target/m2-style-parity-repeat-v1/technical/pdf/standard.pdf",
        ),
        (
            InputFormat::Docx,
            "target/m2-style-parity-repeat-v1/technical/docx/standard.docx",
        ),
    ] {
        let input = std::fs::read(root.join(path)).unwrap();
        let result = helper
            .extract(format, &input, &AtomicBool::new(false))
            .expect("verified helper extraction");
        assert!(
            result
                .blocks()
                .iter()
                .any(|block| block.text.contains("Example"))
        );
    }
    let wrong = ParserHelper::new(executable, hash, "0".repeat(40)).unwrap();
    assert!(matches!(
        wrong.extract(InputFormat::Pdf, &input, &AtomicBool::new(false)),
        Err(ort_platform::ParserProcessError::Identity)
    ));
    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let signal = std::sync::Arc::clone(&cancel);
    let timer = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        signal.store(true, std::sync::atomic::Ordering::Release);
    });
    let started = std::time::Instant::now();
    assert!(matches!(
        helper.extract(InputFormat::Pdf, &input, &cancel),
        Err(ort_platform::ParserProcessError::Cancelled)
    ));
    timer.join().unwrap();
    assert!(started.elapsed() < std::time::Duration::from_secs(3));
    println!(
        "PASS: signed helper, two formats, wrong running-code identity denial, mid-job cancellation and verified reaping"
    );
}
#[cfg(not(target_os = "macos"))]
fn main() {}
