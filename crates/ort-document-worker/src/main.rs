//! Native placeholder; the wasm32 guest handles only the constrained DOCX ABI.
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("Document import is disabled: the platform sandbox gate has not passed.");
    std::process::exit(78);
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use std::io::Write;
    // Guest stdin/stdout are memory transports supplied by the trusted host,
    // never inherited OS descriptors or a general WASI filesystem.
    let result = ort_document_worker::extract_docx(&mut std::io::stdin().lock());
    let Ok(bytes) = result else {
        std::process::exit(65);
    };
    if std::io::stdout().lock().write_all(&bytes).is_err() {
        std::process::exit(74);
    }
}
