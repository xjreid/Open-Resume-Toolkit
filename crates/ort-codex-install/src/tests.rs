use super::*;
use std::io::Cursor;
#[cfg(unix)]
use std::os::unix::fs::symlink;

fn binary() -> Vec<u8> {
    let mut bytes = vec![0; 128];
    bytes[..8].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1]);
    bytes[12..16].copy_from_slice(&[2, 0, 0, 0]);
    bytes
}
fn spec(bytes: &[u8]) -> Manifest {
    Manifest {
        version: "test".into(),
        executable_bytes: bytes.len() as u64,
        executable_sha256: hex::encode(Sha256::digest(bytes)),
        archive_bytes: 0,
        archive_sha256: String::new(),
        download_url: String::new(),
        archive_entry: String::new(),
    }
}
#[test]
fn digest_length_and_macho_are_independent_checks() {
    let bytes = binary();
    let expected = spec(&bytes);
    assert!(
        check_bytes(
            &mut Cursor::new(&bytes),
            &mut Vec::new(),
            expected.executable_bytes,
            &expected.executable_sha256,
            true
        )
        .is_ok()
    );
    for len in [0, 127, 129] {
        assert!(
            check_bytes(
                &mut Cursor::new(&bytes),
                &mut Vec::new(),
                len,
                &expected.executable_sha256,
                true
            )
            .is_err()
        );
    }
    let mut changed = bytes.clone();
    changed[100] = 1;
    assert!(
        check_bytes(
            &mut Cursor::new(changed),
            &mut Vec::new(),
            128,
            &expected.executable_sha256,
            true
        )
        .is_err()
    );
    for index in [0, 4, 7, 12] {
        let mut changed = bytes.clone();
        changed[index] ^= 1;
        let digest = spec(&changed).executable_sha256;
        assert!(
            check_bytes(
                &mut Cursor::new(changed),
                &mut Vec::new(),
                128,
                &digest,
                true
            )
            .is_err()
        );
    }
}
#[test]
fn reads_are_bounded_even_when_input_grows() {
    let bytes = vec![7; 10_000];
    let mut cursor = Cursor::new(bytes);
    assert!(check_bytes(&mut cursor, &mut Vec::new(), 100, "wrong", false).is_err());
    assert!(cursor.position() <= 101);
}
#[test]
fn source_links_directories_and_wrong_lengths_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::write(&source, binary()).unwrap();
    assert!(open_payload(&source, 128).is_ok());
    assert!(open_payload(&source, 129).is_err());
    assert!(open_payload(temp.path(), 128).is_err());
    #[cfg(unix)]
    {
        let link = temp.path().join("link");
        symlink(&source, &link).unwrap();
        assert!(open_payload(&link, 128).is_err());
        let hard = temp.path().join("hard");
        fs::hard_link(&source, &hard).unwrap();
        assert!(open_payload(&source, 128).is_err());
    }
}
#[cfg(unix)]
fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, u32) {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    fs::write(&source, binary()).unwrap();
    let directory = root.path().join("protected");
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let destination = directory.join("Codex/codex");
    let owner = fs::metadata(&directory).unwrap().uid();
    (root, source, destination, owner)
}
#[test]
#[cfg(unix)]
fn successful_copy_is_atomic_executable_and_repeatable() {
    let (_root, source, destination, owner) = fixture();
    let anchor = destination.parent().unwrap().parent().unwrap();
    install_at(&source, &destination, &spec(&binary()), owner, anchor).unwrap();
    assert_eq!(fs::read(&destination).unwrap(), binary());
    assert_eq!(fs::metadata(&destination).unwrap().mode() & 0o777, 0o755);
    install_at(&source, &destination, &spec(&binary()), owner, anchor).unwrap();
    assert_eq!(
        fs::read_dir(destination.parent().unwrap()).unwrap().count(),
        1
    );
}
#[test]
#[cfg(unix)]
fn tampered_or_truncated_payload_preserves_installed_file() {
    let (_root, source, destination, owner) = fixture();
    let anchor = destination.parent().unwrap().parent().unwrap();
    fs::create_dir(destination.parent().unwrap()).unwrap();
    fs::write(&destination, b"old runtime").unwrap();
    let mut bad = binary();
    bad[80] = 1;
    fs::write(&source, bad).unwrap();
    assert!(install_at(&source, &destination, &spec(&binary()), owner, anchor).is_err());
    assert_eq!(fs::read(&destination).unwrap(), b"old runtime");
    fs::write(&source, b"short").unwrap();
    assert!(install_at(&source, &destination, &spec(&binary()), owner, anchor).is_err());
    assert_eq!(fs::read(&destination).unwrap(), b"old runtime");
    assert_eq!(
        fs::read_dir(destination.parent().unwrap()).unwrap().count(),
        1
    );
}
#[test]
#[cfg(unix)]
fn destination_symlinks_hardlinks_and_writable_parents_fail_closed() {
    for kind in [
        "symlink",
        "hardlink",
        "directory",
        "parent_link",
        "parent_writable",
        "target_writable",
        "wrong_owner",
    ] {
        let (root, source, destination, owner) = fixture();
        let anchor = destination.parent().unwrap().parent().unwrap();
        let untouched = root.path().join("untouched");
        fs::write(&untouched, b"do not change").unwrap();
        fs::create_dir(destination.parent().unwrap()).unwrap();
        match kind {
            "symlink" => symlink(&untouched, &destination).unwrap(),
            "hardlink" => fs::hard_link(&untouched, &destination).unwrap(),
            "directory" => fs::create_dir(&destination).unwrap(),
            "parent_link" => {
                fs::remove_dir(destination.parent().unwrap()).unwrap();
                symlink(root.path(), destination.parent().unwrap()).unwrap();
            }
            "parent_writable" => fs::set_permissions(
                destination.parent().unwrap(),
                fs::Permissions::from_mode(0o777),
            )
            .unwrap(),
            "target_writable" => {
                fs::write(&destination, b"old").unwrap();
                fs::set_permissions(&destination, fs::Permissions::from_mode(0o666)).unwrap();
            }
            _ => (),
        }
        let check_owner = if kind == "wrong_owner" {
            owner + 1
        } else {
            owner
        };
        assert!(
            install_at(&source, &destination, &spec(&binary()), check_owner, anchor).is_err(),
            "{kind}"
        );
        assert_eq!(fs::read(&untouched).unwrap(), b"do not change");
    }
}
#[test]
fn elevated_entry_point_rejects_non_administrators_without_writing() {
    if !rustix::process::geteuid().is_root() {
        assert_eq!(
            privileged_main(vec![std::ffi::OsString::from("/anything")].into_iter()),
            Err("PLAN_INSTALL_PERMISSION_DENIED")
        );
    }
}

#[test]
#[cfg(unix)]
fn fifo_and_socket_sources_are_rejected_without_blocking() {
    let root = tempfile::tempdir().unwrap();
    let fifo = root.path().join("fifo");
    assert!(
        std::process::Command::new("/usr/bin/mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    assert!(open_payload(&fifo, 128).is_err());
    let socket = root.path().join("socket");
    let _listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
    assert!(open_payload(&socket, 128).is_err());
}

#[test]
fn modifications_after_open_are_detected_before_publication() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source");
    fs::write(&source, binary()).unwrap();
    let mut open = open_payload(&source, 128).unwrap();
    fs::write(&source, b"changed after verification").unwrap();
    assert!(
        check_bytes(
            &mut open,
            &mut Vec::new(),
            128,
            &spec(&binary()).executable_sha256,
            true
        )
        .is_err()
    );
}

#[test]
fn fragmented_reads_and_write_failures_are_handled() {
    struct Fragmented(Cursor<Vec<u8>>);
    impl Read for Fragmented {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let size = buffer.len().min(3);
            self.0.read(&mut buffer[..size])
        }
    }
    struct FailedWriter;
    impl Write for FailedWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("simulated disk full"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let bytes = binary();
    let expected = spec(&bytes);
    assert!(
        check_bytes(
            &mut Fragmented(Cursor::new(bytes.clone())),
            &mut Vec::new(),
            128,
            &expected.executable_sha256,
            true
        )
        .is_ok()
    );
    assert_eq!(
        check_bytes(
            &mut Cursor::new(bytes),
            &mut FailedWriter,
            128,
            &expected.executable_sha256,
            true
        ),
        Err("PLAN_INSTALL_IO_FAILED")
    );
}
