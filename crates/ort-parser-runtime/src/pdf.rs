use crate::{MEMORY_BYTES, MODULE_BYTES, PDF_FUEL, ParserError, pdf_host};
use ort_documents::{
    import::{
        BlockKind, InputFormat, MAX_EXTRACTED_CHARACTERS, MAX_PAGES, ValidatedExtraction,
        section_kind,
    },
    import_source::inspect_source,
    worker_output::WorkerExtractionBuilder,
};
use sha2::{Digest, Sha256};
use wasmi::{
    Config, Engine, Instance, Module, Store, StoreLimits, StoreLimitsBuilder, WasmParams,
    WasmResults,
};

pub const PDFIUM_SHA256: [u8; 32] = [
    0x32, 0x83, 0x85, 0x7c, 0x1d, 0x26, 0xd4, 0xb1, 0x1c, 0x64, 0x74, 0x3d, 0xeb, 0x41, 0x39, 0x0c,
    0x98, 0x73, 0x38, 0x92, 0x51, 0x5e, 0xfe, 0xa4, 0xe4, 0x42, 0x6c, 0xc0, 0x64, 0x96, 0xd5, 0x12,
];
const MAX_OBJECTS: i32 = 20_000;
struct Guest {
    store: Store<StoreLimits>,
    instance: Instance,
}
impl Guest {
    fn call<P: WasmParams, R: WasmResults>(
        &mut self,
        name: &str,
        params: P,
    ) -> Result<R, ParserError> {
        self.instance
            .get_typed_func::<P, R>(&self.store, name)
            .map_err(|_| ParserError::Module)?
            .call(&mut self.store, params)
            .map_err(|_| ParserError::Execution)
    }
    fn pointer<P: WasmParams>(&mut self, name: &str, params: P) -> Result<i32, ParserError> {
        let pointer: i32 = self.call(name, params)?;
        if pointer <= 0 {
            return Err(ParserError::Execution);
        }
        Ok(pointer)
    }
    fn memory(&self) -> Result<wasmi::Memory, ParserError> {
        self.instance
            .get_memory(&self.store, "memory")
            .ok_or(ParserError::Module)
    }
    fn page_text(&mut self, page: i32, remaining: usize) -> Result<String, ParserError> {
        let objects: i32 = self.call("FPDFPage_CountObjects", page)?;
        if !(0..=MAX_OBJECTS).contains(&objects) {
            return Err(ParserError::Output);
        }
        let mut image_like = false;
        for index in 0..objects {
            let object = self.pointer("FPDFPage_GetObject", (page, index))?;
            let kind: i32 = self.call("FPDFPageObj_GetType", object)?;
            image_like |= kind == 3 || kind == 5;
        }
        let text_page = self.pointer("FPDFText_LoadPage", page)?;
        let count: i32 = self.call("FPDFText_CountChars", text_page)?;
        let count_usize = usize::try_from(count).map_err(|_| ParserError::Output)?;
        // PDFium counts Unicode positions; UTF-16 may require two code units.
        if count_usize > remaining {
            return Err(ParserError::Output);
        }
        let capacity = (count_usize + 1) * 2;
        let pointer = self.pointer(
            "malloc",
            i32::try_from(capacity).map_err(|_| ParserError::Output)?,
        )?;
        let copied: i32 = self.call("FPDFText_GetText", (text_page, 0, count, pointer))?;
        let copied = usize::try_from(copied).map_err(|_| ParserError::Output)?;
        if copied == 0 || copied > count_usize + 1 {
            return Err(ParserError::Output);
        }
        let mut buffer = vec![0u8; copied * 2];
        self.memory()?
            .read(
                &self.store,
                usize::try_from(pointer).map_err(|_| ParserError::Output)?,
                &mut buffer,
            )
            .map_err(|_| ParserError::Output)?;
        let mut utf16: Vec<u16> = buffer
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| u16::from_le_bytes(*b))
            .collect();
        if utf16.pop() != Some(0) {
            return Err(ParserError::Output);
        }
        let text = String::from_utf16(&utf16).map_err(|_| ParserError::Output)?;
        if image_like && text.chars().filter(|c| !c.is_whitespace()).count() < 16 {
            return Err(ParserError::Output);
        }
        self.call::<_, ()>("free", pointer)?;
        self.call::<_, ()>("FPDFText_ClosePage", text_page)?;
        Ok(text)
    }
}
/// Extracts text with the pinned `PDFium` guest and no host OS capabilities.
///
/// # Errors
/// Rejects module mismatch, source failures, traps, fuel/memory exhaustion,
/// scanned pages and page/object/text/output limits. No partial result escapes.
pub fn extract_pdf(module_bytes: &[u8], input: &[u8]) -> Result<ValidatedExtraction, ParserError> {
    if module_bytes.len() > MODULE_BYTES || Sha256::digest(module_bytes).as_slice() != PDFIUM_SHA256
    {
        return Err(ParserError::Module);
    }
    inspect_source(input, InputFormat::Pdf).map_err(|_| ParserError::Source)?;
    let mut config = Config::default();
    config
        .consume_fuel(true)
        .set_max_recursion_depth(256)
        .set_max_stack_height(65_536)
        .set_max_cached_stacks(0);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, module_bytes).map_err(|_| ParserError::Module)?;
    let linker = pdf_host::link(&engine, &module).map_err(|_| ParserError::Module)?;
    let limits = StoreLimitsBuilder::new()
        .memory_size(MEMORY_BYTES)
        .memories(1)
        .tables(1)
        .table_elements(20_000)
        .instances(1)
        .trap_on_grow_failure(true)
        .build();
    let mut store = Store::new(&engine, limits);
    store.limiter(|limits| limits);
    store
        .set_fuel(PDF_FUEL)
        .map_err(|_| ParserError::Execution)?;
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .map_err(|_| ParserError::Execution)?;
    let mut guest = Guest { store, instance };
    guest.call::<_, ()>("__wasm_call_ctors", ())?;
    guest.call::<_, ()>("FPDF_InitLibrary", ())?;
    let length = i32::try_from(input.len()).map_err(|_| ParserError::Source)?;
    let pointer = guest.pointer("malloc", length)?;
    guest
        .memory()?
        .write(
            &mut guest.store,
            usize::try_from(pointer).map_err(|_| ParserError::Execution)?,
            input,
        )
        .map_err(|_| ParserError::Execution)?;
    let document = guest.pointer("FPDF_LoadMemDocument64", (pointer, length, 0))?;
    let count: i32 = guest.call("FPDF_GetPageCount", document)?;
    let pages = u16::try_from(count).map_err(|_| ParserError::Output)?;
    if pages == 0 || pages > MAX_PAGES {
        return Err(ParserError::Output);
    }
    let mut builder =
        WorkerExtractionBuilder::new(InputFormat::Pdf, pages).map_err(|_| ParserError::Output)?;
    let mut remaining = MAX_EXTRACTED_CHARACTERS;
    for index in 0..count {
        let page = guest.pointer("FPDF_LoadPage", (document, index))?;
        let text = guest.page_text(page, remaining)?;
        remaining = remaining
            .checked_sub(text.chars().count())
            .ok_or(ParserError::Output)?;
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        for line in normalized.split('\n') {
            let trimmed = line.trim();
            let kind = if section_kind(trimmed).is_some() {
                BlockKind::Heading
            } else if ["- ", "• ", "* "]
                .iter()
                .any(|prefix| trimmed.starts_with(prefix))
            {
                BlockKind::ListItem
            } else {
                BlockKind::Paragraph
            };
            builder
                .push(
                    u16::try_from(index + 1).map_err(|_| ParserError::Output)?,
                    kind,
                    line.to_owned(),
                )
                .map_err(|_| ParserError::Output)?;
        }
        guest.call::<_, ()>("FPDF_ClosePage", page)?;
    }
    guest.call::<_, ()>("FPDF_CloseDocument", document)?;
    guest.call::<_, ()>("free", pointer)?;
    guest.call::<_, ()>("FPDF_DestroyLibrary", ())?;
    let wire = builder.finish().map_err(|_| ParserError::Output)?;
    ValidatedExtraction::decode(&wire, InputFormat::Pdf).map_err(|_| ParserError::Output)
}
