use crate::{MEMORY_BYTES, pdf_abi};
use wasmi::{Caller, Engine, Extern, ExternType, Linker, Module, StoreLimits, Val};
// Reentrant indirect calls share the same store, fuel and memory limiter.
// Bound host-mediated nesting separately from Wasmi's guest call-stack limit.
pub(super) struct PdfHost {
    pub(super) limits: StoreLimits,
    callback_depth: u8,
}
impl PdfHost {
    pub(super) fn new(limits: StoreLimits) -> Self {
        Self {
            limits,
            callback_depth: 0,
        }
    }
}
const MAX_CALLBACK_DEPTH: u8 = 16;
fn refused() -> wasmi::Error {
    wasmi::Error::new("guest operation refused")
}
fn charge(caller: &mut Caller<'_, PdfHost>, bytes: usize) -> Result<(), wasmi::Error> {
    let cost = u64::try_from(bytes)
        .map_err(|_| refused())?
        .checked_add(32)
        .ok_or_else(refused)?;
    let fuel = caller.get_fuel()?.checked_sub(cost).ok_or_else(refused)?;
    caller.set_fuel(fuel)
}
fn offset(value: &Val) -> Result<usize, wasmi::Error> {
    value
        .i32()
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(refused)
}
fn memory(caller: &Caller<'_, PdfHost>) -> Result<wasmi::Memory, wasmi::Error> {
    caller
        .get_export("memory")
        .and_then(Extern::into_memory)
        .ok_or_else(refused)
}
pub(super) fn link(engine: &Engine, module: &Module) -> Result<Linker<PdfHost>, wasmi::Error> {
    let mut linker = Linker::new(engine);
    for import in module.imports() {
        let expected = pdf_abi::signature(import.module(), import.name()).ok_or_else(refused)?;
        let ExternType::Func(actual) = import.ty() else {
            return Err(refused());
        };
        if actual != &expected {
            return Err(refused());
        }
        let name = import.name().to_owned();
        linker.func_new(
            import.module(),
            import.name(),
            expected,
            move |mut caller, params, results| {
                charge(&mut caller, 0)?;
                match name.as_str() {
                    "_emscripten_memcpy_js" => {
                        let destination = offset(&params[0])?;
                        let source = offset(&params[1])?;
                        let length = offset(&params[2])?;
                        charge(&mut caller, length)?;
                        let memory = memory(&caller)?;
                        let data = memory.data_mut(&mut caller);
                        let end = source
                            .checked_add(length)
                            .filter(|v| *v <= data.len())
                            .ok_or_else(refused)?;
                        destination
                            .checked_add(length)
                            .filter(|v| *v <= data.len())
                            .ok_or_else(refused)?;
                        data.copy_within(source..end, destination);
                    }
                    "emscripten_resize_heap" => {
                        let size = offset(&params[0])?;
                        if size > MEMORY_BYTES {
                            return Err(refused());
                        }
                        let memory = memory(&caller)?;
                        let current = memory.data_size(&caller);
                        if size > current {
                            let pages = (size - current).div_ceil(65_536);
                            charge(&mut caller, pages * 65_536)?;
                            memory
                                .grow(&mut caller, u64::try_from(pages).map_err(|_| refused())?)?;
                        }
                        results[0] = Val::I32(1);
                    }
                    "emscripten_date_now" => results[0] = Val::F64(0.0.into()),
                    "_tzset_js" => {
                        // Fixed UTC metadata parsing; no host timezone or clock.
                        charge(&mut caller, 16)?;
                        let memory = memory(&caller)?;
                        for (index, value) in params.iter().enumerate() {
                            let bytes = if index < 2 { &[0; 4] } else { b"UTC\0" };
                            memory
                                .write(&mut caller, offset(value)?, bytes)
                                .map_err(|_| refused())?;
                        }
                    }
                    "_gmtime_js" | "_localtime_js" => {
                        convert_time(&mut caller, params, name == "_gmtime_js")?;
                    }
                    "environ_get" => results[0] = Val::I32(0),
                    "environ_sizes_get" => {
                        charge(&mut caller, 8)?;
                        let memory = memory(&caller)?;
                        for value in params {
                            memory
                                .write(&mut caller, offset(value)?, &[0; 4])
                                .map_err(|_| refused())?;
                        }
                        results[0] = Val::I32(0);
                    }
                    "fd_read" | "fd_write" | "fd_close" | "fd_sync" | "fd_seek" => {
                        results[0] = Val::I32(8);
                    }
                    n if n.starts_with("__syscall_") => results[0] = Val::I32(-63),
                    "invoke_ii" | "invoke_iii" | "invoke_iiii" | "invoke_iiiii" | "invoke_viii" => {
                        invoke_guest(&mut caller, params, results)?;
                    }
                    // Exceptions and mapping helpers have no host implementation. A
                    // PDF requiring them fails closed, without retaining partial text.
                    _ => return Err(refused()),
                }
                Ok(())
            },
        )?;
    }
    Ok(linker)
}

fn convert_time(
    caller: &mut Caller<'_, PdfHost>,
    params: &[Val],
    utc: bool,
) -> Result<(), wasmi::Error> {
    charge(caller, 256)?;
    let low = params[0].i32().ok_or_else(refused)?.to_le_bytes();
    let high = params[1].i32().ok_or_else(refused)?.to_le_bytes();
    let seconds = i64::from_le_bytes([
        low[0], low[1], low[2], low[3], high[0], high[1], high[2], high[3],
    ]);
    let date = jiff::Timestamp::from_second(seconds)
        .map_err(|_| refused())?
        .to_zoned(jiff::tz::TimeZone::UTC);
    let fields = [
        i32::from(date.second()),
        i32::from(date.minute()),
        i32::from(date.hour()),
        i32::from(date.day()),
        i32::from(date.month()) - 1,
        i32::from(date.year()) - 1900,
        i32::from(date.weekday().to_sunday_zero_offset()),
        i32::from(date.day_of_year()) - 1,
        0,
        0,
    ];
    let mut bytes = [0; 40];
    for (index, field) in fields.iter().enumerate() {
        bytes[index * 4..index * 4 + 4].copy_from_slice(&field.to_le_bytes());
    }
    memory(caller)?
        .write(
            caller,
            offset(&params[2])?,
            &bytes[..if utc { 32 } else { 40 }],
        )
        .map_err(|_| refused())?;
    Ok(())
}

fn invoke_guest(
    caller: &mut Caller<'_, PdfHost>,
    params: &[Val],
    results: &mut [Val],
) -> Result<(), wasmi::Error> {
    let index = params
        .first()
        .and_then(Val::i32)
        .and_then(|value| u64::try_from(value).ok())
        .ok_or_else(refused)?;
    let table = caller
        .get_export("__indirect_function_table")
        .and_then(Extern::into_table)
        .ok_or_else(refused)?;
    let value = table.get(&*caller, index).ok_or_else(refused)?;
    let function = value
        .funcref()
        .and_then(|reference| reference.val().map(|function| **function))
        .ok_or_else(refused)?;
    let ty = function.ty(&*caller);
    if ty
        .params()
        .iter()
        .copied()
        .ne(params[1..].iter().map(Val::ty))
        || ty.results().iter().copied().ne(results.iter().map(Val::ty))
        || caller.data().callback_depth >= MAX_CALLBACK_DEPTH
    {
        return Err(refused());
    }
    caller.data_mut().callback_depth += 1;
    // A trap aborts the whole extraction. Never swallow exceptions or return
    // partial text; successful dispatch grants no additional host capability.
    let result = function.call(&mut *caller, &params[1..], results);
    caller.data_mut().callback_depth -= 1;
    result
}
