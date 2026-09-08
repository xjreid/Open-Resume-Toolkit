use sha2::{Digest, Sha256};
use std::{fmt::Write as _, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-env-changed=ORT_PARSER_GUEST_DIRECTORY");
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let mut generated = String::new();
    if let Some(directory) = std::env::var_os("ORT_PARSER_GUEST_DIRECTORY") {
        let directory = PathBuf::from(directory)
            .canonicalize()
            .expect("guest directory");
        for (name, constant) in [("docx.wasm", "DOCX"), ("pdfium.wasm", "PDF")] {
            let path = directory.join(name);
            println!("cargo:rerun-if-changed={}", path.display());
            let bytes = std::fs::read(&path).expect("bundled guest");
            assert!(
                !bytes.is_empty() && bytes.len() <= 8 * 1024 * 1024,
                "module size"
            );
            let digest: [u8; 32] = Sha256::digest(&bytes).into();
            if name == "pdfium.wasm" {
                let expected = "3283857c1d26d4b11c64743deb41390c98733892515efea4e4426cc06496d512";
                let hex = digest.iter().fold(String::new(), |mut output, byte| {
                    write!(output, "{byte:02x}").unwrap();
                    output
                });
                assert_eq!(hex, expected, "PDFium identity");
            }
            std::fs::write(out.join(name), &bytes).unwrap();
            writeln!(generated, "pub const {constant}: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{name}\"));").unwrap();
            if name == "docx.wasm" {
                writeln!(generated, "pub const DOCX_HASH: [u8;32] = {digest:?};").unwrap();
            }
        }
    } else {
        // Ordinary workspace checks do not silently fetch/build release assets.
        // This binary refuses every request when no bundled guests were supplied.
        generated.push_str("pub const DOCX: &[u8] = &[];\npub const PDF: &[u8] = &[];\npub const DOCX_HASH: [u8;32] = [0;32];\n");
    }
    std::fs::write(out.join("guests.rs"), generated).unwrap();
}
