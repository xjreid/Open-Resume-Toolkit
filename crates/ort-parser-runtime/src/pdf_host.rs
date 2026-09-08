use crate::{MEMORY_BYTES, pdf_abi};
use wasmi::{Caller, Engine, Extern, ExternType, Linker, Module, StoreLimits, Val};
fn refused() -> wasmi::Error {
    wasmi::Error::new("guest operation refused")
}
fn charge(caller: &mut Caller<'_, StoreLimits>, bytes: usize) -> Result<(), wasmi::Error> {
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
fn memory(caller: &Caller<'_, StoreLimits>) -> Result<wasmi::Memory, wasmi::Error> {
    caller
        .get_export("memory")
        .and_then(Extern::into_memory)
        .ok_or_else(refused)
}
pub(super) fn link(engine: &Engine, module: &Module) -> Result<Linker<StoreLimits>, wasmi::Error> {
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
                    // Exception/time/mapping helpers have no host implementation. A
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
    caller: &mut Caller<'_, StoreLimits>,
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
