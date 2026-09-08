use super::*;
use wasmi::{Instance, Val, ValType};

fn leb(mut value: u32) -> Vec<u8> {
    let mut result = vec![];
    loop {
        let byte = u8::try_from(value & 127).unwrap();
        value >>= 7;
        result.push(byte | if value == 0 { 0 } else { 128 });
        if value == 0 {
            return result;
        }
    }
}
fn section(module: &mut Vec<u8>, id: u8, bytes: &[u8]) {
    module.push(id);
    module.extend(leb(u32::try_from(bytes.len()).unwrap()));
    module.extend(bytes);
}
fn name(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend(leb(u32::try_from(value.len()).unwrap()));
    bytes.extend(value.as_bytes());
}
/// First-party binary fixture forwarding a typed function through a guest frame,
/// so the callback sees only this instance's exported linear memory.
fn fixture(namespace: &str, function: &str, params: usize, result: bool) -> Vec<u8> {
    let mut module = b"\0asm\x01\0\0\0".to_vec();
    let mut ty = vec![1, 0x60, u8::try_from(params).unwrap()];
    ty.extend(vec![0x7f; params]);
    ty.push(u8::from(result));
    if result {
        ty.push(0x7f);
    }
    section(&mut module, 1, &ty);
    let mut imports = vec![1];
    name(&mut imports, namespace);
    name(&mut imports, function);
    imports.extend([0, 0]);
    section(&mut module, 2, &imports);
    section(&mut module, 3, &[1, 0]);
    section(&mut module, 5, &[1, 0, 2]);
    let mut exports = vec![2];
    name(&mut exports, "run");
    exports.extend([0, 1]);
    name(&mut exports, "memory");
    exports.extend([2, 0]);
    section(&mut module, 7, &exports);
    let mut body = vec![0];
    for index in 0..params {
        body.extend([0x20, u8::try_from(index).unwrap()]);
    }
    body.extend([0x10, 0, 0x0b]);
    let mut code = vec![1];
    code.extend(leb(u32::try_from(body.len()).unwrap()));
    code.extend(body);
    section(&mut module, 10, &code);
    module
}
fn engine() -> Engine {
    let mut config = Config::default();
    config.consume_fuel(true);
    Engine::new(&config)
}
fn job(function: &str, params: usize) -> (Store<Job>, Instance) {
    let engine = engine();
    let module = Module::new(
        &engine,
        fixture("wasi_snapshot_preview1", function, params, true),
    )
    .unwrap();
    let mut linker = Linker::new(&engine);
    wasi::link(&mut linker, &module).unwrap();
    let mut store = Store::new(
        &engine,
        Job {
            limits: StoreLimitsBuilder::new().memory_size(MEMORY_BYTES).build(),
            input: b"synthetic".to_vec(),
            offset: 0,
            output: vec![],
            stderr_bytes: 0,
        },
    );
    store.limiter(|job| &mut job.limits);
    store.set_fuel(1_000_000).unwrap();
    let instance = linker.instantiate_and_start(&mut store, &module).unwrap();
    (store, instance)
}
fn vector(store: &mut Store<Job>, instance: Instance, pointer: u32, length: u32) {
    let memory = instance.get_memory(&*store, "memory").unwrap();
    memory
        .write(
            &mut *store,
            0,
            &[pointer.to_le_bytes(), length.to_le_bytes()].concat(),
        )
        .unwrap();
}
#[test]
fn wasi_reads_only_job_bytes_and_never_ambient_descriptors() {
    let (mut store, instance) = job("fd_read", 4);
    vector(&mut store, instance, 64, 32);
    let run = instance
        .get_typed_func::<(i32, i32, i32, i32), i32>(&store, "run")
        .unwrap();
    assert_eq!(run.call(&mut store, (3, 0, 1, 16)).unwrap(), 8);
    assert_eq!(store.data().offset, 0);
    assert_eq!(run.call(&mut store, (0, 0, 1, 16)).unwrap(), 0);
    let mut bytes = [0; 9];
    instance
        .get_memory(&store, "memory")
        .unwrap()
        .read(&store, 64, &mut bytes)
        .unwrap();
    assert_eq!(&bytes, b"synthetic");
    assert_eq!(store.data().offset, 9);
    assert_eq!(run.call(&mut store, (0, 0, 1, 16)).unwrap(), 0); // EOF
}
#[test]
fn wasi_rejects_invalid_vectors_and_charges_before_copy() {
    for (pointer, length, count) in [(u32::MAX, 32, 1), (131_060, 32, 1), (64, 32, 17)] {
        let (mut store, instance) = job("fd_write", 4);
        vector(&mut store, instance, pointer, length);
        let run = instance
            .get_typed_func::<(i32, i32, i32, i32), i32>(&store, "run")
            .unwrap();
        assert!(run.call(&mut store, (1, 0, count, 16)).is_err());
        assert!(store.data().output.is_empty());
    }
    let (mut store, instance) = job("fd_write", 4);
    vector(&mut store, instance, 64, 1000);
    store.set_fuel(500).unwrap();
    assert!(
        instance
            .get_typed_func::<(i32, i32, i32, i32), i32>(&store, "run")
            .unwrap()
            .call(&mut store, (1, 0, 1, 16))
            .is_err()
    );
    assert!(store.data().output.is_empty());
}
#[test]
fn wasi_output_and_stderr_are_cumulatively_bounded() {
    for fd in [1, 2] {
        let (mut store, instance) = job("fd_write", 4);
        vector(&mut store, instance, 64, 8192);
        let run = instance
            .get_typed_func::<(i32, i32, i32, i32), i32>(&store, "run")
            .unwrap();
        let limit = if fd == 1 {
            ort_documents::import::MAX_EXTRACTION_BYTES
        } else {
            16 * 1024
        };
        for _ in 0..limit / 8192 {
            assert_eq!(run.call(&mut store, (fd, 0, 1, 16)).unwrap(), 0);
        }
        assert!(run.call(&mut store, (fd, 0, 1, 16)).is_err());
        assert_eq!(store.data().output.len(), if fd == 1 { limit } else { 0 });
        assert_eq!(store.data().stderr_bytes, if fd == 2 { limit } else { 0 });
    }
}
#[test]
fn unknown_or_mismatched_imports_are_not_linked() {
    let engine = engine();
    for (namespace, function, params) in [
        ("env", "open", 2),
        ("wasi_snapshot_preview1", "path_open", 4),
        ("wasi_snapshot_preview1", "fd_read", 3),
    ] {
        let module = Module::new(&engine, fixture(namespace, function, params, true)).unwrap();
        assert!(wasi::link(&mut Linker::new(&engine), &module).is_err());
        assert!(pdf_host::link(&engine, &module).is_err());
    }
}
#[test]
fn pdf_host_copy_is_bounded_and_fueled_before_mutation() {
    let engine = engine();
    let module = Module::new(&engine, fixture("env", "_emscripten_memcpy_js", 3, false)).unwrap();
    let linker = pdf_host::link(&engine, &module).unwrap();
    let mut store = Store::new(
        &engine,
        StoreLimitsBuilder::new().memory_size(131_072).build(),
    );
    store.set_fuel(1000).unwrap();
    let instance = linker.instantiate_and_start(&mut store, &module).unwrap();
    let memory = instance.get_memory(&store, "memory").unwrap();
    memory.write(&mut store, 64, b"synthetic").unwrap();
    let run = instance
        .get_typed_func::<(i32, i32, i32), ()>(&store, "run")
        .unwrap();
    run.call(&mut store, (65, 64, 9)).unwrap();
    let mut bytes = [0; 9];
    memory.read(&store, 65, &mut bytes).unwrap();
    assert_eq!(&bytes, b"synthetic");
    assert!(run.call(&mut store, (131_071, 64, 9)).is_err());
    store.set_fuel(500).unwrap();
    assert!(run.call(&mut store, (128, 64, 1000)).is_err());
    memory.read(&store, 128, &mut bytes).unwrap();
    assert_eq!(bytes, [0; 9]);
}
#[test]
fn pdf_host_cannot_grow_over_store_limit_or_expose_files() {
    let engine = engine();
    for (namespace, function, params, values, expected) in [
        (
            "env",
            "emscripten_resize_heap",
            1,
            vec![Val::I32(196_608)],
            None,
        ),
        (
            "env",
            "__syscall_openat",
            4,
            vec![Val::I32(0); 4],
            Some(-63),
        ),
        (
            "wasi_snapshot_preview1",
            "fd_read",
            4,
            vec![Val::I32(0); 4],
            Some(8),
        ),
    ] {
        let module = Module::new(&engine, fixture(namespace, function, params, true)).unwrap();
        let linker = pdf_host::link(&engine, &module).unwrap();
        let mut store = Store::new(
            &engine,
            StoreLimitsBuilder::new()
                .memory_size(131_072)
                .trap_on_grow_failure(true)
                .build(),
        );
        store.limiter(|limits| limits);
        store.set_fuel(1_000_000).unwrap();
        let instance = linker.instantiate_and_start(&mut store, &module).unwrap();
        let mut output = [Val::default(ValType::I32)];
        let result =
            instance
                .get_func(&store, "run")
                .unwrap()
                .call(&mut store, &values, &mut output);
        if let Some(value) = expected {
            result.unwrap();
            assert_eq!(output[0].i32(), Some(value));
        } else {
            assert!(result.is_err());
        }
    }
}
#[test]
fn unpinned_bytes_are_rejected_before_guest_execution() {
    assert_eq!(
        extract_docx(b"not a module", &[0; 32], b"source").unwrap_err(),
        ParserError::Module
    );
    assert_eq!(
        extract_pdf(b"not a module", b"source").unwrap_err(),
        ParserError::Module
    );
}

#[test]
fn pdf_dates_use_fixed_utc_and_bound_the_exact_tm_write() {
    let engine = engine();
    for function in ["_gmtime_js", "_localtime_js"] {
        let module = Module::new(&engine, fixture("env", function, 3, false)).unwrap();
        let linker = pdf_host::link(&engine, &module).unwrap();
        let mut store = Store::new(&engine, StoreLimitsBuilder::new().build());
        store.set_fuel(1000).unwrap();
        let instance = linker.instantiate_and_start(&mut store, &module).unwrap();
        let memory = instance.get_memory(&store, "memory").unwrap();
        memory.write(&mut store, 64, &[0x42; 44]).unwrap();
        let run = instance
            .get_typed_func::<(i32, i32, i32), ()>(&store, "run")
            .unwrap();
        run.call(&mut store, (0, 0, 64)).unwrap();
        let mut bytes = [0; 44];
        memory.read(&store, 64, &mut bytes).unwrap();
        assert_eq!(i32::from_le_bytes(bytes[12..16].try_into().unwrap()), 1);
        assert_eq!(i32::from_le_bytes(bytes[20..24].try_into().unwrap()), 70);
        assert_eq!(&bytes[40..44], &[0x42; 4]);
        if function == "_gmtime_js" {
            assert_eq!(&bytes[32..40], &[0x42; 8]);
        }
        assert!(run.call(&mut store, (0, 0, 131_060)).is_err());
    }
}
