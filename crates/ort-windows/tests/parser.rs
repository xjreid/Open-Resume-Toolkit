#![cfg(windows)]
//! Native containment qualification with a minimal disposable helper, never ORT.
use ort_windows::{ContainedParser, ParserError};
use sha2::{Digest, Sha256};
use std::{
    sync::{Mutex, atomic::AtomicBool},
    time::{Duration, Instant},
};
static SERIAL: Mutex<()> = Mutex::new(());
fn serial() -> std::sync::MutexGuard<'static, ()> {
    assert_eq!(
        std::env::var("ORT_RUN_WINDOWS_PARSER_TESTS").as_deref(),
        Ok("1")
    );
    SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
struct CredentialCleanup(keyring::Entry);
impl Drop for CredentialCleanup {
    fn drop(&mut self) {
        self.0.delete_credential().unwrap();
    }
}

fn fixture(mode: u8, timeout: Duration) -> Result<Vec<u8>, ParserError> {
    exchange(&[mode], timeout)
}

fn exchange(request: &[u8], timeout: Duration) -> Result<Vec<u8>, ParserError> {
    let temporary = tempfile::tempdir().unwrap();
    let executable = temporary.path().join("probe.exe");
    // Cargo may hard-link its bin artifact into target/debug. Packages contain
    // independent files; exercise that layout without relaxing the link check.
    std::fs::copy(env!("CARGO_BIN_EXE_ort-windows-probe"), &executable).unwrap();
    let expected: [u8; 32] = Sha256::digest(std::fs::read(&executable).unwrap()).into();
    ContainedParser::exchange(
        &executable,
        &expected,
        request,
        Instant::now() + timeout,
        &AtomicBool::new(false),
    )
}

#[test]
#[ignore = "explicit native-probes and ORT_RUN_WINDOWS_PARSER_TESTS=1 opt-in required"]
fn verified_helper_exits_with_bounded_output_and_confirmed_cleanup() {
    let _serial = serial();
    let bytes = fixture(b'E', Duration::from_secs(15)).unwrap();
    assert_eq!(bytes, b"PROBE-OK");
}

#[test]
#[ignore = "explicit native-probes and ORT_RUN_WINDOWS_PARSER_TESTS=1 opt-in required"]
fn counterfeit_image_and_cancelled_jobs_are_rejected_without_output() {
    let _serial = serial();
    let temporary = tempfile::tempdir().unwrap();
    let executable = temporary.path().join("probe.exe");
    std::fs::copy(env!("CARGO_BIN_EXE_ort-windows-probe"), &executable).unwrap();
    assert_eq!(
        ContainedParser::exchange(
            &executable,
            &[0; 32],
            b"E",
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(false)
        ),
        Err(ParserError::Identity)
    );
    assert_eq!(
        ContainedParser::exchange(
            &executable,
            &[0; 32],
            b"E",
            Instant::now() + Duration::from_secs(5),
            &AtomicBool::new(true)
        ),
        Err(ParserError::Cancelled)
    );
}

#[test]
#[ignore = "explicit native-probes and ORT_RUN_WINDOWS_PARSER_TESTS=1 opt-in required"]
fn source_access_credentials_network_writes_and_children_are_denied() {
    let _serial = serial();
    let temporary = ort_windows::testing::tempdir().unwrap();
    let sensitive = temporary.path().join("synthetic-private.txt");
    std::fs::write(&sensitive, b"SYNTHETIC-PRIVATE-CONTROL").unwrap();
    assert_eq!(
        std::fs::read(&sensitive).unwrap(),
        b"SYNTHETIC-PRIVATE-CONTROL"
    );
    let server = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = server.local_addr().unwrap().port();
    let control = std::net::TcpStream::connect(server.local_addr().unwrap()).unwrap();
    drop(server.accept().unwrap());
    drop(control);
    let mut random = [0_u8; 16];
    getrandom::fill(&mut random).unwrap();
    let account = hex::encode(random);
    let service = "com.openresumetoolkit.platform-test.containment";
    let credential = keyring::Entry::new(service, &account).unwrap();
    let cleanup = CredentialCleanup(credential);
    cleanup.0.set_secret(b"SYNTHETIC-CREDENTIAL").unwrap();
    assert_eq!(cleanup.0.get_secret().unwrap(), b"SYNTHETIC-CREDENTIAL");
    let request = format!("P\n{}\n{port}\n{account}.{service}", sensitive.display());
    assert_eq!(
        exchange(request.as_bytes(), Duration::from_secs(15)).unwrap(),
        b"ISOLATED"
    );
    server.set_nonblocking(true).unwrap();
    assert_eq!(
        server.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

#[test]
#[ignore = "explicit native-probes and ORT_RUN_WINDOWS_PARSER_TESTS=1 opt-in required"]
fn timeout_and_output_flood_do_not_prevent_a_subsequent_clean_job() {
    let _serial = serial();
    assert_eq!(
        fixture(b'S', Duration::from_secs(2)),
        Err(ParserError::Timeout)
    );
    assert_eq!(fixture(b'F', Duration::from_secs(15)), Err(ParserError::Io));
    assert_eq!(fixture(b'E', Duration::from_secs(15)).unwrap(), b"PROBE-OK");
}
