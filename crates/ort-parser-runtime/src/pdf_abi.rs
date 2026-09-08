// Exact ABI of the pinned chromium/7881 module. Not inferred from guest input.
use wasmi::{FuncType, ValType};
pub(super) fn signature(module: &str, name: &str) -> Option<FuncType> {
    use ValType::{F64, I32};
    let (parameters, results): (&[ValType], &[ValType]) = match (module, name) {
        ("env", "emscripten_resize_heap" | "__syscall_rmdir")
        | ("wasi_snapshot_preview1", "fd_close" | "fd_sync") => (&[I32], &[I32]),
        ("env", "_emscripten_memcpy_js" | "_localtime_js" | "_gmtime_js") => {
            (&[I32, I32, I32], &[])
        }
        ("env", "_abort_js" | "_emscripten_throw_longjmp") => (&[], &[]),
        ("env", "emscripten_date_now") => (&[], &[F64]),
        ("env", "_tzset_js" | "invoke_viii") => (&[I32, I32, I32, I32], &[]),
        ("wasi_snapshot_preview1", "environ_sizes_get" | "environ_get")
        | ("env", "__syscall_fstat64" | "__syscall_stat64" | "__syscall_lstat64" | "invoke_ii") => {
            (&[I32, I32], &[I32])
        }
        ("env", "__syscall_openat" | "__syscall_newfstatat" | "invoke_iiii")
        | ("wasi_snapshot_preview1", "fd_write" | "fd_read") => (&[I32, I32, I32, I32], &[I32]),
        (
            "env",
            "__syscall_fcntl64"
            | "__syscall_ioctl"
            | "invoke_iii"
            | "__syscall_getdents64"
            | "__syscall_unlinkat"
            | "__syscall_ftruncate64",
        ) => (&[I32, I32, I32], &[I32]),
        ("env", "invoke_iiiii") | ("wasi_snapshot_preview1", "fd_seek") => {
            (&[I32, I32, I32, I32, I32], &[I32])
        }
        ("env", "_munmap_js") => (&[I32, I32, I32, I32, I32, I32, I32], &[I32]),
        ("env", "_mmap_js") => (&[I32, I32, I32, I32, I32, I32, I32, I32], &[I32]),
        _ => return None,
    };
    Some(FuncType::new(
        parameters.iter().copied(),
        results.iter().copied(),
    ))
}
