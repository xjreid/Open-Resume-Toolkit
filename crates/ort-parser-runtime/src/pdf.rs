use crate::{MEMORY_BYTES, MODULE_BYTES, PDF_FUEL, ParserError, pdf_host};
use ort_documents::{
    import::{
        BlockKind, InputFormat, MAX_EXTRACTED_CHARACTERS, MAX_PAGES, TextLayout,
        ValidatedExtraction, section_kind,
    },
    import_source::inspect_source,
    worker_output::WorkerExtractionBuilder,
};
use sha2::{Digest, Sha256};
use wasmi::{Config, Engine, Instance, Module, Store, StoreLimitsBuilder, WasmParams, WasmResults};

pub const PDFIUM_SHA256: [u8; 32] = [
    0x32, 0x83, 0x85, 0x7c, 0x1d, 0x26, 0xd4, 0xb1, 0x1c, 0x64, 0x74, 0x3d, 0xeb, 0x41, 0x39, 0x0c,
    0x98, 0x73, 0x38, 0x92, 0x51, 0x5e, 0xfe, 0xa4, 0xe4, 0x42, 0x6c, 0xc0, 0x64, 0x96, 0xd5, 0x12,
];
const MAX_OBJECTS: i32 = 20_000;
#[derive(Default)]
struct PdfLine {
    text: String,
    layout: Option<TextLayout>,
}
fn flush_line(lines: &mut Vec<PdfLine>, line: &mut PdfLine) {
    if line.text.trim().is_empty() {
        *line = PdfLine::default();
    } else {
        line.text = line.text.trim().to_owned();
        lines.push(std::mem::take(line));
    }
}
// Quantization is bounded before casting, including nonfinite guest values.
#[allow(clippy::cast_possible_truncation)]
fn point(value: f64) -> Option<i32> {
    let scaled = (value * 1000.0).round();
    (scaled.is_finite() && scaled.abs() <= 14_400_000.0).then_some(scaled as i32)
}
fn glyph_layout(values: &[f64], height: f64, font: f64) -> Option<TextLayout> {
    let layout = TextLayout {
        left: point(values[0])?,
        right: point(values[1])?,
        top: point(height - values[3])?,
        bottom: point(height - values[2])?,
        font_size: u32::try_from(point(font)?).ok()?,
    };
    layout.is_valid().then_some(layout)
}
fn order_lines(lines: &mut [PdfLine]) {
    lines.sort_by_key(|line| line.layout.map(|value| (value.top, value.left)));
    let mut start = 0;
    while start < lines.len() {
        let Some(anchor) = lines[start].layout else {
            start += 1;
            continue;
        };
        let mut end = start + 1;
        while end < lines.len()
            && lines[end].layout.is_some_and(|value| {
                (i64::from(value.bottom) - i64::from(anchor.bottom)).abs()
                    <= i64::from(value.font_size.max(anchor.font_size)) / 2
            })
        {
            end += 1;
        }
        lines[start..end].sort_by_key(|line| line.layout.map(|value| value.left));
        start = end;
    }
}
// Section headings establish column margins. Group each column as a whole
// so a sidebar Skills section cannot absorb rows from Experience beside it.
fn order_columns(lines: &mut [PdfLine]) {
    let mut headings: Vec<_> = lines
        .iter()
        .filter(|line| section_kind(&line.text).is_some())
        .filter_map(|line| line.layout)
        .collect();
    headings.sort_by_key(|layout| layout.left);
    let mut columns: Vec<TextLayout> = Vec::new();
    for heading in &headings {
        if columns.last().is_none_or(|prior| {
            i64::from(heading.left) - i64::from(prior.left)
                > i64::from(heading.font_size.max(prior.font_size)) * 8
        }) {
            columns.push(*heading);
        }
    }
    if !(2..=3).contains(&columns.len()) {
        return;
    }
    let first_heading = headings.iter().map(|layout| layout.top).min().unwrap_or(0);
    // Keep the contact area above all section columns, in its visual order.
    lines.sort_by_key(|line| {
        line.layout.map_or(0, |layout| {
            if layout.bottom < first_heading {
                0
            } else {
                columns
                    .iter()
                    .rposition(|column| layout.left >= column.left - 5_000)
                    .map_or(1, |index| index + 1)
            }
        })
    });
}
struct Guest {
    store: Store<pdf_host::PdfHost>,
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
    fn page_text(
        &mut self,
        page: i32,
        remaining: usize,
    ) -> Result<(String, Vec<PdfLine>), ParserError> {
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
        let height: f64 = self.call("FPDF_GetPageHeight", page)?;
        let lines = self.positioned_lines(text_page, count, height, &text)?;
        self.call::<_, ()>("FPDFText_ClosePage", text_page)?;
        Ok((text, lines))
    }
    fn character_layout(
        &mut self,
        text_page: i32,
        index: i32,
        pointer: i32,
        height: f64,
    ) -> Result<Option<TextLayout>, ParserError> {
        let success: i32 = self.call(
            "FPDFText_GetCharBox",
            (
                text_page,
                index,
                pointer,
                pointer + 8,
                pointer + 16,
                pointer + 24,
            ),
        )?;
        if success == 0 {
            return Ok(None);
        }
        let mut bytes = [0; 32];
        self.memory()?
            .read(
                &self.store,
                usize::try_from(pointer).map_err(|_| ParserError::Output)?,
                &mut bytes,
            )
            .map_err(|_| ParserError::Output)?;
        let values: Vec<_> = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|value| f64::from_le_bytes(*value))
            .collect();
        let font: f64 = self.call("FPDFText_GetFontSize", (text_page, index))?;
        let success: i32 = self.call("FPDFText_GetMatrix", (text_page, index, pointer))?;
        if success == 0 {
            return Ok(None);
        }
        let mut matrix = [0; 24];
        self.memory()?
            .read(
                &self.store,
                usize::try_from(pointer).map_err(|_| ParserError::Output)?,
                &mut matrix,
            )
            .map_err(|_| ParserError::Output)?;
        // FS_MATRIX uses floats. Font size is in text space; its vertical
        // matrix vector supplies the effective size used by page coordinates.
        let c = f64::from(f32::from_le_bytes(
            matrix[8..12].try_into().map_err(|_| ParserError::Output)?,
        ));
        let d = f64::from(f32::from_le_bytes(
            matrix[12..16].try_into().map_err(|_| ParserError::Output)?,
        ));
        Ok(glyph_layout(&values, height, font * c.hypot(d)))
    }
    fn positioned_lines(
        &mut self,
        text_page: i32,
        count: i32,
        height: f64,
        original: &str,
    ) -> Result<Vec<PdfLine>, ParserError> {
        let pointer = self.pointer("malloc", 32)?;
        let mut lines = Vec::new();
        let mut line = PdfLine::default();
        let mut observed = String::new();
        let mut complete_layout = true;
        for index in 0..count {
            let unicode: u32 = self.call("FPDFText_GetUnicode", (text_page, index))?;
            let Some(character) = char::from_u32(unicode).filter(|c| *c != '\0') else {
                continue;
            };
            observed.push(character);
            if matches!(character, '\n' | '\r') {
                flush_line(&mut lines, &mut line);
                continue;
            }
            if character.is_whitespace() {
                line.text.push(character);
                continue;
            }
            let bounds = self.character_layout(text_page, index, pointer, height)?;
            complete_layout &= bounds.is_some();
            if let (Some(prior), Some(next)) = (line.layout, bounds) {
                let tolerance = i64::from(prior.font_size.max(next.font_size));
                let vertical = (i64::from(prior.bottom) - i64::from(next.bottom)).abs();
                let gap = i64::from(next.left) - i64::from(prior.right);
                if vertical > tolerance * 2 / 3
                    || gap > tolerance * 3 / 2
                    || i64::from(next.right) < i64::from(prior.left) - tolerance
                {
                    flush_line(&mut lines, &mut line);
                }
            }
            line.text.push(character);
            line.layout = match (line.layout, bounds) {
                (Some(prior), Some(next)) => Some(TextLayout {
                    left: prior.left.min(next.left),
                    top: prior.top.min(next.top),
                    right: prior.right.max(next.right),
                    bottom: prior.bottom.max(next.bottom),
                    font_size: prior.font_size.max(next.font_size),
                }),
                (None, next) => next,
                (prior, None) => prior,
            };
        }
        flush_line(&mut lines, &mut line);
        self.call::<_, ()>("free", pointer)?;
        // Unicode APIs may omit unmapped glyphs. Keep the original text-only
        // extraction if the positioned version cannot account for every word.
        if !complete_layout
            || observed
                .chars()
                .filter(|c| !c.is_whitespace())
                .ne(original.chars().filter(|c| !c.is_whitespace()))
            || lines.iter().any(|line| line.layout.is_none())
        {
            return Ok(original
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .split('\n')
                .map(|text| PdfLine {
                    text: text.into(),
                    layout: None,
                })
                .collect());
        }
        order_lines(&mut lines);
        order_columns(&mut lines);
        Ok(lines)
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
    let mut store = Store::new(&engine, pdf_host::PdfHost::new(limits));
    store.limiter(|host| &mut host.limits);
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
        let (text, lines) = guest.page_text(page, remaining)?;
        remaining = remaining
            .checked_sub(text.chars().count())
            .ok_or(ParserError::Output)?;
        for line in lines {
            let trimmed = line.text.trim();
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
                .push_with_layout(
                    u16::try_from(index + 1).map_err(|_| ParserError::Output)?,
                    kind,
                    line.text,
                    line.layout,
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
