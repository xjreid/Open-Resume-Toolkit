//! Synthetic unsupported-volume qualification; never reads application data.
fn main() {
    let root = std::path::PathBuf::from(std::env::args_os().nth(1).expect("disposable volume"));
    assert_eq!(
        std::fs::read(root.join("ort-disposable-volume")).unwrap(),
        b"ORT synthetic filesystem qualification"
    );
    let output = root.join("synthetic-export.txt");
    assert!(!output.exists());
    let result = ort_platform::ExportDestination::from_native_dialog(&output)
        .and_then(|destination| destination.write(b"ORT synthetic export payload"));
    assert!(result.is_err(), "unsupported filesystem must fail closed");
    assert!(!output.exists());
    for entry in std::fs::read_dir(&root).unwrap() {
        let entry = entry.unwrap();
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(".ort-export-")
        {
            assert!(entry.file_type().unwrap().is_dir());
            assert_eq!(
                std::fs::read_dir(entry.path()).unwrap().count(),
                0,
                "no named plaintext payload"
            );
        }
    }
    println!(
        "PASS: unsupported-volume publication refused; no output or named plaintext payload (empty staging directories may remain)"
    );
}
