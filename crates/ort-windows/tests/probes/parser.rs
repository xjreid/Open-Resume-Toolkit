//! Synthetic fixture only; no parent-launcher or application dependencies.
#![cfg_attr(windows, windows_subsystem = "windows")]
use std::io::{Read, Write};
fn main() {
    let mut request = Vec::new();
    std::io::stdin()
        .take(10 * 1024 * 1024 + 10)
        .read_to_end(&mut request)
        .unwrap();
    match request.first() {
        Some(b'E') => std::io::stdout().write_all(b"PROBE-OK").unwrap(),
        Some(b'F') => std::io::stdout()
            .write_all(&vec![0x41; 1024 * 1024])
            .unwrap(),
        Some(b'S') => std::thread::sleep(std::time::Duration::from_secs(5)),
        Some(b'P') => isolation_probe(&request),
        _ => std::process::exit(67),
    }
}

#[cfg(windows)]
fn isolation_probe(request: &[u8]) {
    use std::{
        io::ErrorKind,
        net::{Ipv4Addr, SocketAddr, TcpStream},
        time::Duration,
    };
    let text = std::str::from_utf8(request).unwrap();
    let fields: Vec<_> = text.lines().collect();
    assert_eq!(fields.len(), 4);
    // Positive control: the helper can execute and read its own pinned image.
    let executable = std::env::current_exe().unwrap();
    assert!(!std::fs::read(&executable).unwrap().is_empty());
    if !matches!(std::fs::read(fields[1]), Err(error) if error.kind() == ErrorKind::PermissionDenied)
    {
        std::process::exit(81);
    }
    let write = std::fs::write(executable.parent().unwrap().join("forbidden.txt"), b"X");
    if !matches!(write, Err(error) if error.kind() == ErrorKind::PermissionDenied) {
        std::process::exit(82);
    }
    if std::process::Command::new(&executable)
        .arg("forbidden-child")
        .spawn()
        .is_ok()
    {
        std::process::exit(83);
    }
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, fields[2].parse::<u16>().unwrap()));
    match TcpStream::connect_timeout(&address, Duration::from_secs(1)) {
        // Windows filtering can silently drop the AppContainer connection.
        // The parent separately proves the listener is live and no connection arrives.
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::PermissionDenied | ErrorKind::TimedOut
            ) => {}
        _ => std::process::exit(84),
    }
    if !credential_denied(fields[3]) {
        std::process::exit(85);
    }
    std::io::stdout().write_all(b"ISOLATED").unwrap();
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn credential_denied(target: &str) -> bool {
    use windows_sys::Win32::{
        Foundation::{ERROR_ACCESS_DENIED, ERROR_NO_SUCH_LOGON_SESSION, GetLastError},
        Security::Credentials::{CRED_TYPE_GENERIC, CredFree, CredReadW},
    };
    let target: Vec<u16> = target.encode_utf16().chain(Some(0)).collect();
    let mut credential = std::ptr::null_mut();
    // SAFETY: synthetic exact target; successful output is freed without logging.
    if unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &raw mut credential) } != 0 {
        unsafe { CredFree(credential.cast()) };
        return false;
    }
    matches!(
        unsafe { GetLastError() },
        ERROR_ACCESS_DENIED | ERROR_NO_SUCH_LOGON_SESSION
    )
}

#[cfg(not(windows))]
fn isolation_probe(_: &[u8]) {
    std::process::exit(78);
}
