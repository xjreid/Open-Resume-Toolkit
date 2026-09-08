use crate::Job;
use ort_documents::import::MAX_EXTRACTION_BYTES;
use wasmi::{Caller, Extern, ExternType, FuncType, Linker, Module, ValType};
const STDERR_LIMIT: usize = 16 * 1024;
fn denied() -> wasmi::Error {
    wasmi::Error::new("guest operation refused")
}
fn offset(value: i32) -> Result<usize, wasmi::Error> {
    usize::try_from(value).map_err(|_| denied())
}
fn memory(caller: &Caller<'_, Job>) -> Result<wasmi::Memory, wasmi::Error> {
    caller
        .get_export("memory")
        .and_then(Extern::into_memory)
        .ok_or_else(denied)
}
fn charge(caller: &mut Caller<'_, Job>, count: usize) -> Result<(), wasmi::Error> {
    let fuel = caller
        .get_fuel()?
        .checked_sub(
            u64::try_from(count)
                .map_err(|_| denied())?
                .saturating_add(32),
        )
        .ok_or_else(denied)?;
    caller.set_fuel(fuel)
}
fn write_u32(caller: &mut Caller<'_, Job>, address: i32, value: usize) -> Result<(), wasmi::Error> {
    let bytes = u32::try_from(value).map_err(|_| denied())?.to_le_bytes();
    memory(caller)?
        .write(caller, offset(address)?, &bytes)
        .map_err(|_| denied())
}
fn vector(
    caller: &Caller<'_, Job>,
    base: usize,
    index: usize,
) -> Result<(usize, usize), wasmi::Error> {
    let address = base
        .checked_add(index.checked_mul(8).ok_or_else(denied)?)
        .ok_or_else(denied)?;
    let mut bytes = [0; 8];
    memory(caller)?
        .read(caller, address, &mut bytes)
        .map_err(|_| denied())?;
    let pointer = u32::from_le_bytes(bytes[..4].try_into().map_err(|_| denied())?);
    let length = u32::from_le_bytes(bytes[4..].try_into().map_err(|_| denied())?);
    Ok((
        usize::try_from(pointer).map_err(|_| denied())?,
        usize::try_from(length).map_err(|_| denied())?,
    ))
}
fn io(
    mut caller: Caller<'_, Job>,
    fd: i32,
    vectors: i32,
    count: i32,
    result: i32,
    read: bool,
) -> Result<i32, wasmi::Error> {
    if (read && fd != 0) || (!read && fd != 1 && fd != 2) {
        return Ok(8);
    }
    let count = offset(count)?;
    if count > 16 {
        return Err(denied());
    }
    let base = offset(vectors)?;
    charge(&mut caller, count)?;
    let mut transferred = 0usize;
    for index in 0..count {
        let (pointer, length) = vector(&caller, base, index)?;
        let budget = if read {
            length.min(caller.data().input.len() - caller.data().offset)
        } else {
            length
        };
        charge(&mut caller, budget)?;
        let memory = memory(&caller)?;
        let (data, job) = memory.data_and_store_mut(&mut caller);
        let end = pointer
            .checked_add(length)
            .filter(|end| *end <= data.len())
            .ok_or_else(denied)?;
        let actual = if read {
            let n = length.min(job.input.len() - job.offset);
            data[pointer..pointer + n].copy_from_slice(&job.input[job.offset..job.offset + n]);
            job.offset += n;
            n
        } else if fd == 1 {
            let next = job
                .output
                .len()
                .checked_add(length)
                .filter(|n| *n <= MAX_EXTRACTION_BYTES)
                .ok_or_else(denied)?;
            job.output
                .try_reserve(next - job.output.len())
                .map_err(|_| denied())?;
            job.output.extend_from_slice(&data[pointer..end]);
            length
        } else {
            job.stderr_bytes = job
                .stderr_bytes
                .checked_add(length)
                .filter(|n| *n <= STDERR_LIMIT)
                .ok_or_else(denied)?;
            length // Count only: no stderr bytes retained or printed.
        };
        transferred = transferred.checked_add(actual).ok_or_else(denied)?;
        if read && actual < length {
            break;
        }
    }
    write_u32(&mut caller, result, transferred)?;
    Ok(0)
}

pub(super) fn link(linker: &mut Linker<Job>, module: &Module) -> Result<(), wasmi::Error> {
    for import in module.imports() {
        if import.module() != "wasi_snapshot_preview1" {
            return Err(denied());
        }
        let (parameters, results) = match import.name() {
            "random_get" | "environ_get" | "environ_sizes_get" => (2, 1),
            "fd_read" | "fd_write" => (4, 1),
            "proc_exit" => (1, 0),
            _ => return Err(denied()),
        };
        let ExternType::Func(ty) = import.ty() else {
            return Err(denied());
        };
        if ty != &FuncType::new(vec![ValType::I32; parameters], vec![ValType::I32; results]) {
            return Err(denied());
        }
    }
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "proc_exit",
        |code: i32| -> Result<(), wasmi::Error> { Err(wasmi::Error::i32_exit(code)) },
    )?;
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_read",
        |caller: Caller<'_, Job>, fd: i32, vectors: i32, count: i32, result: i32| {
            io(caller, fd, vectors, count, result, true)
        },
    )?;
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "fd_write",
        |caller: Caller<'_, Job>, fd: i32, vectors: i32, count: i32, result: i32| {
            io(caller, fd, vectors, count, result, false)
        },
    )?;
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "environ_get",
        |mut caller: Caller<'_, Job>, _: i32, _: i32| -> Result<i32, wasmi::Error> {
            charge(&mut caller, 0)?;
            Ok(0)
        },
    )?;
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "environ_sizes_get",
        |mut caller: Caller<'_, Job>, count: i32, size: i32| -> Result<i32, wasmi::Error> {
            charge(&mut caller, 8)?;
            write_u32(&mut caller, count, 0)?;
            write_u32(&mut caller, size, 0)?;
            Ok(0)
        },
    )?;
    linker.func_wrap(
        "wasi_snapshot_preview1",
        "random_get",
        |mut caller: Caller<'_, Job>, pointer: i32, length: i32| -> Result<i32, wasmi::Error> {
            let length = offset(length)?;
            if length > 256 {
                return Err(denied());
            }
            charge(&mut caller, length)?;
            // Deterministic guest hash seeding, not a cryptographic randomness service.
            memory(&caller)?
                .write(&mut caller, offset(pointer)?, &vec![0x42; length])
                .map_err(|_| denied())?;
            Ok(0)
        },
    )?;
    Ok(())
}
