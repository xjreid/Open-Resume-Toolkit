//! Safe, narrow Windows security primitives. No renderer or shell interface.
//!
//! Windows API calls live only in reviewed modules. The rest of the workspace
//! retains `unsafe_code = "forbid"`.

#[cfg(windows)]
#[allow(unsafe_code)]
mod security;

#[cfg(windows)]
#[allow(unsafe_code)]
mod pipes;

#[cfg(windows)]
#[allow(unsafe_code)]
mod parser;

#[cfg(windows)]
#[allow(unsafe_code)]
mod registration;
#[cfg(windows)]
pub use registration::DevelopmentChromeRegistration;

#[cfg(windows)]
pub use parser::{ContainedParser, ParserError};

#[cfg(windows)]
pub use pipes::{PipeConnection, PipeListener, PipeOperation};

#[cfg(windows)]
pub use security::{
    CredentialLock, PrivateDirectory, reject_reparse_path, secure_file, selected_source_handles,
};

#[cfg(all(windows, feature = "test-support"))]
pub mod testing;
