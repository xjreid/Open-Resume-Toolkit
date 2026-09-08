//! Narrow macOS `AppKit` termination bridge. The rest of the workspace forbids unsafe Rust.
//! No UI command or content is accepted here; callers supply a process-lifetime callback.

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod native {
    use std::sync::OnceLock;

    static REQUEST: OnceLock<Box<dyn Fn() -> bool + Send + Sync>> = OnceLock::new();

    unsafe extern "C" {
        fn ort_macos_install_termination(callback: extern "C" fn() -> bool) -> bool;
        fn ort_macos_reply_termination(approve: bool) -> bool;
    }

    extern "C" fn request() -> bool {
        // Never unwind through Objective-C. A failed callback cancels the OS request.
        std::panic::catch_unwind(|| REQUEST.get().is_some_and(|callback| callback()))
            .unwrap_or(false)
    }

    pub fn install(callback: impl Fn() -> bool + Send + Sync + 'static) -> bool {
        if REQUEST.set(Box::new(callback)).is_err() {
            return false;
        }
        // SAFETY: C takes no borrowed pointer; this static trampoline and OnceLock
        // callback live for the process. Native code rejects non-main-thread use.
        unsafe { ort_macos_install_termination(request) }
    }

    pub fn reply(approve: bool) -> bool {
        // SAFETY: scalar-only ABI; native code checks main-thread and pending state.
        unsafe { ort_macos_reply_termination(approve) }
    }
}

/// Installs the one-shot process-lifetime hook on the `AppKit` main thread.
/// Returns false if the delegate already implements termination or installation fails.
#[cfg(target_os = "macos")]
pub fn install(callback: impl Fn() -> bool + Send + Sync + 'static) -> bool {
    native::install(callback)
}

/// Replies once to an outstanding native termination request on the main thread.
/// Returns false when there is no such request. False never authorizes termination.
#[cfg(target_os = "macos")]
#[must_use]
pub fn reply(approve: bool) -> bool {
    native::reply(approve)
}
