//! Read-only import/export inventory for the pinned local evaluation artifact.
use sha2::Digest;
fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm-containment/pdfium.wasm");
    let bytes = std::fs::read(root).expect("download verified evaluation artifact first");
    assert_eq!(bytes.len(), 5_233_982);
    let expected: [u8; 32] = [
        0x32, 0x83, 0x85, 0x7c, 0x1d, 0x26, 0xd4, 0xb1, 0x1c, 0x64, 0x74, 0x3d, 0xeb, 0x41, 0x39,
        0x0c, 0x98, 0x73, 0x38, 0x92, 0x51, 0x5e, 0xfe, 0xa4, 0xe4, 0x42, 0x6c, 0xc0, 0x64, 0x96,
        0xd5, 0x12,
    ];
    assert_eq!(sha2::Sha256::digest(&bytes).as_slice(), expected);
    let engine = wasmi::Engine::default();
    let module = wasmi::Module::new(&engine, &bytes).expect("compile candidate");
    for import in module.imports() {
        println!(
            "import {}.{} {:?}",
            import.module(),
            import.name(),
            import.ty()
        );
    }
    for export in module.exports().filter(|item| {
        item.name().contains("FPDF")
            || matches!(
                item.name(),
                "memory" | "malloc" | "free" | "_initialize" | "__wasm_call_ctors"
            )
    }) {
        println!("export {} {:?}", export.name(), export.ty());
    }
}
