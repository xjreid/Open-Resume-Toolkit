//! Evaluation only: pinned Wasm, synthetic in-memory PDF, no host OS capabilities.
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use wasmi::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, Val};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bytes = load_pinned()?;
    let mut config = Config::default();
    config.consume_fuel(true).set_max_recursion_depth(256);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, &bytes)?;
    let limits = StoreLimitsBuilder::new()
        .memory_size(512 * 1024 * 1024)
        .memories(1)
        .tables(1)
        .table_elements(20_000)
        .instances(1)
        .trap_on_grow_failure(true)
        .build();
    let mut store = Store::new(&engine, limits);
    store.limiter(|limits| limits);
    store.set_fuel(50_000_000)?;
    let linker = link_module(&engine, &module)?;
    let instance = linker.instantiate_and_start(&mut store, &module)?;
    instance
        .get_typed_func::<(), ()>(&store, "__wasm_call_ctors")?
        .call(&mut store, ())?;
    instance
        .get_typed_func::<(), ()>(&store, "FPDF_InitLibrary")?
        .call(&mut store, ())?;
    let pdf = synthetic_pdf();
    let length = i32::try_from(pdf.len())?;
    let pointer = instance
        .get_typed_func::<i32, i32>(&store, "malloc")?
        .call(&mut store, length)?;
    assert!(pointer > 0);
    let memory = instance
        .get_memory(&store, "memory")
        .ok_or("missing memory")?;
    memory.write(&mut store, usize::try_from(pointer)?, &pdf)?;
    let document = instance
        .get_typed_func::<(i32, i32, i32), i32>(&store, "FPDF_LoadMemDocument64")?
        .call(&mut store, (pointer, length, 0))?;
    assert!(document > 0);
    let pages = instance
        .get_typed_func::<i32, i32>(&store, "FPDF_GetPageCount")?
        .call(&mut store, document)?;
    assert_eq!(pages, 1);
    let page = instance
        .get_typed_func::<(i32, i32), i32>(&store, "FPDF_LoadPage")?
        .call(&mut store, (document, 0))?;
    assert!(page > 0);
    let text_page = instance
        .get_typed_func::<i32, i32>(&store, "FPDFText_LoadPage")?
        .call(&mut store, page)?;
    assert!(text_page > 0);
    let count = instance
        .get_typed_func::<i32, i32>(&store, "FPDFText_CountChars")?
        .call(&mut store, text_page)?;
    assert!((1..1000).contains(&count));
    let output = instance
        .get_typed_func::<i32, i32>(&store, "malloc")?
        .call(&mut store, (count + 1) * 2)?;
    assert!(output > 0);
    let copied = instance
        .get_typed_func::<(i32, i32, i32, i32), i32>(&store, "FPDFText_GetText")?
        .call(&mut store, (text_page, 0, count, output))?;
    assert!(copied > 0 && copied <= count + 1);
    let mut bytes = vec![0; usize::try_from(copied)? * 2];
    memory.read(&store, usize::try_from(output)?, &mut bytes)?;
    let utf16 = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect::<Vec<_>>();
    let text = String::from_utf16(&utf16)?;
    assert!(text.contains("Experience") && text.contains("Built safely"));
    for (function, handle) in [
        ("FPDFText_ClosePage", text_page),
        ("FPDF_ClosePage", page),
        ("FPDF_CloseDocument", document),
        ("free", pointer),
        ("free", output),
    ] {
        instance
            .get_typed_func::<i32, ()>(&store, function)?
            .call(&mut store, handle)?;
    }
    println!(
        "PASS: pinned PDFium Wasm extracted the expected synthetic text without host OS capabilities; fuel remaining {}",
        store.get_fuel()?
    );
    instance
        .get_typed_func::<(), ()>(&store, "FPDF_DestroyLibrary")?
        .call(&mut store, ())?;
    Ok(())
}

fn synthetic_pdf() -> Vec<u8> {
    let stream = b"BT /F1 18 Tf 72 720 Td (Experience) Tj 0 -24 Td (- Built safely) Tj ET";
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        format!("<< /Length {} >>\nstream\n", stream.len())
            .into_bytes()
            .into_iter()
            .chain(stream.iter().copied())
            .chain(b"\nendstream".iter().copied())
            .collect(),
    ];

    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn link_module(
    engine: &Engine,
    module: &Module,
) -> Result<Linker<StoreLimits>, Box<dyn std::error::Error>> {
    let mut linker = Linker::<StoreLimits>::new(engine);
    for import in module.imports() {
        let wasmi::ExternType::Func(ty) = import.ty() else {
            return Err("unexpected non-function import".into());
        };
        let name = import.name().to_owned();
        linker.func_new(
            import.module(),
            import.name(),
            ty.clone(),
            move |mut caller, params, results| match name.as_str() {
                "_emscripten_memcpy_js" => {
                    let offsets = params
                        .iter()
                        .map(|value| {
                            value
                                .i32()
                                .and_then(|v| usize::try_from(v).ok())
                                .ok_or_else(|| wasmi::Error::new("invalid copy"))
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let [destination, source, length] = offsets.as_slice() else {
                        return Err(wasmi::Error::new("invalid copy arity"));
                    };
                    let fuel = caller
                        .get_fuel()?
                        .checked_sub(
                            u64::try_from(*length).map_err(|_| wasmi::Error::new("copy length"))?,
                        )
                        .ok_or_else(|| wasmi::Error::new("copy fuel exhausted"))?;
                    caller.set_fuel(fuel)?;
                    let memory = caller
                        .get_export("memory")
                        .and_then(wasmi::Extern::into_memory)
                        .ok_or_else(|| wasmi::Error::new("memory absent"))?;
                    let data = memory.data_mut(&mut caller);
                    let end = source
                        .checked_add(*length)
                        .filter(|end| *end <= data.len())
                        .ok_or_else(|| wasmi::Error::new("copy bounds"))?;
                    destination
                        .checked_add(*length)
                        .filter(|end| *end <= data.len())
                        .ok_or_else(|| wasmi::Error::new("copy bounds"))?;
                    data.copy_within(*source..end, *destination);
                    Ok(())
                }
                "emscripten_date_now" => {
                    results[0] = Val::F64(0.0.into());
                    Ok(())
                }
                "emscripten_resize_heap" | "environ_get" => {
                    results[0] = Val::I32(0);
                    Ok(())
                }
                "environ_sizes_get" => {
                    let memory = caller
                        .get_export("memory")
                        .and_then(wasmi::Extern::into_memory)
                        .ok_or_else(|| wasmi::Error::new("memory absent"))?;
                    for value in params {
                        let offset = usize::try_from(
                            value
                                .i32()
                                .ok_or_else(|| wasmi::Error::new("invalid offset"))?,
                        )
                        .map_err(|_| wasmi::Error::new("invalid offset"))?;
                        memory
                            .write(&mut caller, offset, &[0; 4])
                            .map_err(|_| wasmi::Error::new("write bounds"))?;
                    }
                    results[0] = Val::I32(0);
                    Ok(())
                }
                n if n.starts_with("__syscall_") => {
                    results[0] = Val::I32(-63);
                    Ok(())
                }
                "fd_read" | "fd_write" | "fd_close" | "fd_sync" | "fd_seek" => {
                    results[0] = Val::I32(8);
                    Ok(())
                }
                _ => Err(wasmi::Error::new(format!(
                    "unimplemented candidate import: {name}"
                ))),
            },
        )?;
    }
    Ok(linker)
}

fn load_pinned() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/wasm-containment/pdfium.wasm");
    let bytes = std::fs::read(path)?;
    assert_eq!(
        Sha256::digest(&bytes)
            .iter()
            .fold(String::new(), |mut out, byte| {
                write!(out, "{byte:02x}").unwrap();
                out
            }),
        "3283857c1d26d4b11c64743deb41390c98733892515efea4e4426cc06496d512"
    );
    Ok(bytes)
}
