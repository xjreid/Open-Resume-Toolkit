//! Capability-restricted guest execution. No native file, process or vault access.
//! Callers must supply bundled module bytes and their trusted build-manifest hash.
use ort_documents::import::{InputFormat, ValidatedExtraction};
use ort_documents::import_source::{MAX_IMPORT_SOURCE_BYTES, inspect_source};
use sha2::{Digest, Sha256};
use wasmi::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};
mod pdf;
mod pdf_abi;
mod pdf_host;
mod wasi;
pub use pdf::{PDFIUM_SHA256, extract_pdf};

pub const MEMORY_BYTES: usize = 512 * 1024 * 1024;
pub const FUEL: u64 = 50_000_000;
pub const PDF_FUEL: u64 = 500_000_000;
pub const MODULE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ParserError {
    #[error("parser module does not match its trusted manifest")]
    Module,
    #[error("document source is invalid or oversized")]
    Source,
    #[error("parser execution failed or exceeded its resource budget")]
    Execution,
    #[error("parser output is invalid or oversized")]
    Output,
}

struct Job {
    limits: StoreLimits,
    input: Vec<u8>,
    offset: usize,
    output: Vec<u8>,
    stderr_bytes: usize,
}

/// Executes the constrained DOCX guest with an exact import/signature allowlist.
/// No WASI implementation or ambient host descriptors are installed.
///
/// # Errors
/// Refuses unpinned modules, unsupported imports, invalid sources, traps,
/// exhausted fuel, nonzero exits and invalid extraction. Output is all-or-none.
pub fn extract_docx(
    module_bytes: &[u8],
    expected_sha256: &[u8; 32],
    input: &[u8],
) -> Result<ValidatedExtraction, ParserError> {
    if module_bytes.len() > MODULE_BYTES
        || Sha256::digest(module_bytes).as_slice() != expected_sha256
    {
        return Err(ParserError::Module);
    }
    if input.len() > MAX_IMPORT_SOURCE_BYTES || inspect_source(input, InputFormat::Docx).is_err() {
        return Err(ParserError::Source);
    }
    let mut config = Config::default();
    config
        .consume_fuel(true)
        .set_max_recursion_depth(256)
        .set_max_stack_height(65_536)
        .set_max_cached_stacks(0);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, module_bytes).map_err(|_| ParserError::Module)?;
    let mut linker = Linker::<Job>::new(&engine);
    wasi::link(&mut linker, &module).map_err(|_| ParserError::Module)?;
    let limits = StoreLimitsBuilder::new()
        .memory_size(MEMORY_BYTES)
        .memories(1)
        .tables(1)
        .table_elements(20_000)
        .instances(1)
        .trap_on_grow_failure(true)
        .build();
    let mut store = Store::new(
        &engine,
        Job {
            limits,
            input: input.to_vec(),
            offset: 0,
            output: vec![],
            stderr_bytes: 0,
        },
    );
    store.limiter(|job| &mut job.limits);
    store.set_fuel(FUEL).map_err(|_| ParserError::Execution)?;
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(|_| ParserError::Execution)?;
    let entry = instance
        .get_typed_func::<(), ()>(&store, "_start")
        .map_err(|_| ParserError::Module)?;
    if let Err(error) = entry.call(&mut store, ())
        && error.i32_exit_status() != Some(0)
    {
        return Err(ParserError::Execution);
    }
    ValidatedExtraction::decode(&store.data().output, InputFormat::Docx)
        .map_err(|_| ParserError::Output)
}

#[cfg(test)]
mod tests;
