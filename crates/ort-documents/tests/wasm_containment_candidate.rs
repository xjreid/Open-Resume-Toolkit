//! Synthetic alternative-containment evaluation only; no production parser.
//! Modules are fixed first-party binary fixtures, not user-provided code.
use wasmi::TrapCode;
use wasmi::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

fn leb(mut value: u32) -> Vec<u8> {
    let mut bytes = vec![];
    loop {
        let low = u8::try_from(value & 127).unwrap();
        value >>= 7;
        bytes.push(low | if value == 0 { 0 } else { 128 });
        if value == 0 {
            return bytes;
        }
    }
}
fn section(module: &mut Vec<u8>, kind: u8, bytes: &[u8]) {
    module.push(kind);
    module.extend(leb(u32::try_from(bytes.len()).unwrap()));
    module.extend(bytes);
}
fn fixture(instructions: &[u8], initial_pages: u32, import: bool) -> Vec<u8> {
    let mut module = b"\0asm\x01\0\0\0".to_vec();
    section(&mut module, 1, &[1, 0x60, 0, 1, 0x7f]); // () -> i32
    if import {
        section(
            &mut module,
            2,
            &[1, 3, b'e', b'n', b'v', 4, b'o', b'p', b'e', b'n', 0, 0],
        );
    }
    section(&mut module, 3, &[1, 0]);
    let mut memory = vec![1, 0]; // one memory, no declared maximum
    memory.extend(leb(initial_pages));
    section(&mut module, 5, &memory);
    section(
        &mut module,
        7,
        &[1, 3, b'r', b'u', b'n', 0, u8::from(import)],
    );
    let mut body = vec![0]; // no locals
    body.extend(instructions);
    body.push(0x0b);
    let mut code = vec![1];
    code.extend(leb(u32::try_from(body.len()).unwrap()));
    code.extend(body);
    section(&mut module, 10, &code);
    module
}
fn engine() -> Engine {
    let mut config = Config::default();
    config.consume_fuel(true);
    config.set_max_recursion_depth(64);
    Engine::new(&config)
}
fn store(engine: &Engine, bytes: usize) -> Store<StoreLimits> {
    let limits = StoreLimitsBuilder::new()
        .memory_size(bytes)
        .memories(1)
        .tables(0)
        .instances(1)
        .table_elements(0)
        .trap_on_grow_failure(true)
        .build();
    let mut store = Store::new(engine, limits);
    store.limiter(|limits| limits);
    store.set_fuel(10_000).unwrap();
    store
}
fn execute(code: &[u8], bytes: usize) -> Result<i32, wasmi::Error> {
    let engine = engine();
    let module = Module::new(&engine, code)?;
    let mut store = store(&engine, bytes);
    // Empty linker: no WASI, filesystem, network, process, clock or vault API.
    let instance =
        Linker::<StoreLimits>::new(&engine).instantiate_and_start(&mut store, &module)?;
    instance
        .get_typed_func::<(), i32>(&store, "run")?
        .call(&mut store, ())
}
#[test]
fn normal_execution_and_permitted_memory_growth_work() {
    assert_eq!(
        execute(&fixture(&[0x41, 42], 1, false), 131_072).unwrap(),
        42
    );
    assert_eq!(
        execute(&fixture(&[0x41, 1, 0x40, 0], 1, false), 131_072).unwrap(),
        1
    );
}
#[test]
fn guest_cannot_grow_past_host_limit_even_without_a_module_maximum() {
    let error = execute(&fixture(&[0x41, 2, 0x40, 0], 1, false), 131_072).unwrap_err();
    assert_eq!(error.as_trap_code(), Some(TrapCode::GrowthOperationLimited));
    // 8192-page growth plus the initial page exceeds 512 MiB. The runtime
    // rejects it before allocation; this does not stress host memory.
    let error = execute(
        &fixture(&[0x41, 0x80, 0xc0, 0, 0x40, 0], 1, false),
        512 * 1024 * 1024,
    )
    .unwrap_err();
    assert_eq!(error.as_trap_code(), Some(TrapCode::GrowthOperationLimited));
    assert!(execute(&fixture(&[0x41, 0], 3, false), 131_072).is_err());
}
#[test]
fn infinite_loop_exhausts_fuel_and_cannot_reset_it() {
    let error = execute(
        &fixture(&[0x03, 0x40, 0x0c, 0, 0x0b, 0x41, 0], 1, false),
        65_536,
    )
    .unwrap_err();
    assert_eq!(error.as_trap_code(), Some(TrapCode::OutOfFuel));
}
#[test]
fn recursion_and_out_of_bounds_access_trap() {
    let recursion = execute(&fixture(&[0x10, 0], 1, false), 65_536).unwrap_err();
    assert_eq!(recursion.as_trap_code(), Some(TrapCode::StackOverflow));
    let bounds = execute(
        &fixture(&[0x41, 0x80, 0x80, 4, 0x28, 2, 0], 1, false),
        65_536,
    )
    .unwrap_err();
    assert_eq!(bounds.as_trap_code(), Some(TrapCode::MemoryOutOfBounds));
}
#[test]
fn unprovided_host_authority_is_refused_before_execution() {
    let error = execute(&fixture(&[0x10, 0], 1, true), 65_536).unwrap_err();
    assert!(error.to_string().contains("definition"), "{error}");
}
