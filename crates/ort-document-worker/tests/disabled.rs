#[test]
fn worker_remains_inert_until_native_containment_is_proven() {
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_ort-document-worker"))
        .arg("--input")
        .arg("untrusted-argument-must-not-enable-parsing.pdf")
        .output()
        .expect("start inert worker");
    assert_eq!(result.status.code(), Some(78));
    assert!(result.stdout.is_empty());
    assert_eq!(
        String::from_utf8(result.stderr).unwrap(),
        "Document import is disabled: the platform sandbox gate has not passed.\n"
    );
}
