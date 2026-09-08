//! One request per disposable process. The guest has no OS capabilities.
//! Native packaging must apply App Sandbox and verify this executable identity.
use ort_documents::{
    import::{InputFormat, ValidatedExtraction},
    import_source::MAX_IMPORT_SOURCE_BYTES,
    worker_output::WorkerExtractionBuilder,
};
use std::io::{Read, Write};
#[cfg(unix)]
use std::time::{Duration, Instant};
mod guests {
    include!(concat!(env!("OUT_DIR"), "/guests.rs"));
}
fn main() {
    // Panic diagnostics must not disclose source text, guest pointers or paths.
    std::panic::set_hook(Box::new(|_| {}));
    if guests::DOCX.is_empty() || guests::PDF.is_empty() {
        std::process::exit(78);
    }
    if watchdog().is_err() {
        std::process::exit(70);
    }
    if run().is_err() {
        std::process::exit(65);
    }
}
#[cfg(unix)]
fn watchdog() -> Result<(), ()> {
    use rustix::process::{Resource, Rlimit, getppid, setrlimit};
    let parent = getppid().filter(|pid| pid.as_raw_pid() > 1).ok_or(())?;
    setrlimit(
        Resource::Core,
        Rlimit {
            current: Some(0),
            maximum: Some(0),
        },
    )
    .map_err(|_| ())?;
    let start = Instant::now();
    std::thread::Builder::new()
        .name("ort-parser-watchdog".into())
        .spawn(move || {
            loop {
                if getppid() != Some(parent) || start.elapsed() >= Duration::from_secs(60) {
                    std::process::exit(124);
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        })
        .map_err(|_| ())?;
    Ok(())
}
#[cfg(not(unix))]
fn watchdog() -> Result<(), ()> {
    Err(())
}
fn run() -> Result<(), ()> {
    let mut input = std::io::stdin().lock();
    let mut header = [0; 9];
    input.read_exact(&mut header).map_err(|_| ())?;
    if &header[..4] != b"ORTW" {
        return Err(());
    }
    let format = match header[4] {
        1 => InputFormat::Docx,
        2 => InputFormat::Pdf,
        _ => return Err(()),
    };
    let length = usize::try_from(u32::from_le_bytes(header[5..].try_into().map_err(|_| ())?))
        .map_err(|_| ())?;
    if length == 0 || length > MAX_IMPORT_SOURCE_BYTES {
        return Err(());
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(length).map_err(|_| ())?;
    bytes.resize(length, 0);
    input.read_exact(&mut bytes).map_err(|_| ())?;
    let mut trailing = [0];
    if input.read(&mut trailing).map_err(|_| ())? != 0 {
        return Err(());
    }
    let result = match format {
        InputFormat::Docx => {
            ort_parser_runtime::extract_docx(guests::DOCX, &guests::DOCX_HASH, &bytes)
        }
        InputFormat::Pdf => ort_parser_runtime::extract_pdf(guests::PDF, &bytes),
    }
    .map_err(|_| ())?;
    let output = wire(&result)?;
    std::io::stdout().lock().write_all(&output).map_err(|_| ())
}
fn wire(extraction: &ValidatedExtraction) -> Result<Vec<u8>, ()> {
    let mut builder = WorkerExtractionBuilder::new(extraction.format(), extraction.page_count())
        .map_err(|_| ())?;
    for block in extraction.blocks() {
        builder
            .push(block.page, block.kind, block.text.clone())
            .map_err(|_| ())?;
    }
    builder.finish().map_err(|_| ())
}
