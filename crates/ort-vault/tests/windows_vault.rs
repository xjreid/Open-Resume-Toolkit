#![cfg(windows)]
//! Opt-in, disposable native qualification. Never reads user/application entries.
use ort_vault::{DatabaseKey, DatabaseKeyVault, OsDatabaseKeyVault, VaultError, VaultReference};
use std::{
    io::{Read, Write},
    process::{Child, Command, Stdio},
};

const CHILD_INSTALL: &str = "ORT_SYNTHETIC_VAULT_CHILD_INSTALL";

#[test]
fn creation_child() {
    let Ok(install) = std::env::var(CHILD_INSTALL) else {
        return;
    };
    assert_eq!(std::env::var("ORT_RUN_OS_VAULT_TESTS").as_deref(), Ok("1"));
    let value: u8 = std::env::var("ORT_SYNTHETIC_VAULT_CHILD_BYTE")
        .unwrap()
        .parse()
        .unwrap();
    let reference = VaultReference::new("platform-test", &install, "windows-race").unwrap();
    let key = DatabaseKey::from_bytes(vec![value; 32]).unwrap();
    std::io::stdin().read_exact(&mut [0]).unwrap();
    match OsDatabaseKeyVault::new().store_new(&reference, &key) {
        Ok(()) => println!("ORT-CREATED"),
        Err(VaultError::AlreadyExists) => println!("ORT-EXISTS"),
        Err(error) => panic!("synthetic credential creation failed: {error}"),
    }
}

struct TestChild(Option<Child>);
impl Drop for TestChild {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

struct Cleanup(VaultReference);
impl Drop for Cleanup {
    fn drop(&mut self) {
        let _ = OsDatabaseKeyVault::new().delete(&self.0);
    }
}

#[test]
#[ignore = "creates/deletes one synthetic Windows credential across two processes; explicit native opt-in"]
fn independent_processes_cannot_replace_the_winning_database_key() {
    assert_eq!(std::env::var("ORT_RUN_OS_VAULT_TESTS").as_deref(), Ok("1"));
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).unwrap();
    let install = hex::encode(bytes);
    let reference = VaultReference::new("platform-test", &install, "windows-race").unwrap();
    let _cleanup = Cleanup(reference.clone());
    let mut children: Vec<_> = [0x35_u8, 0xa7]
        .into_iter()
        .map(|value| {
            let child = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "creation_child", "--nocapture"])
                .env("ORT_RUN_OS_VAULT_TESTS", "1")
                .env(CHILD_INSTALL, &install)
                .env("ORT_SYNTHETIC_VAULT_CHILD_BYTE", value.to_string())
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            (value, TestChild(Some(child)))
        })
        .collect();
    for (_, child) in &mut children {
        child
            .0
            .as_mut()
            .unwrap()
            .stdin
            .take()
            .unwrap()
            .write_all(&[1])
            .unwrap();
    }
    let mut winner = None;
    let mut losers = 0;
    for (value, mut child) in children {
        let output = child.0.take().unwrap().wait_with_output().unwrap();
        assert!(output.status.success(), "synthetic child failed");
        let text = String::from_utf8(output.stdout).unwrap();
        if text.contains("ORT-CREATED") {
            assert!(
                winner.replace(value).is_none(),
                "two processes overwrote the same key"
            );
        } else {
            assert!(text.contains("ORT-EXISTS"));
            losers += 1;
        }
    }
    assert_eq!(losers, 1);
    let winner = winner.expect("one successful creator");
    let loaded = OsDatabaseKeyVault::new().load(&reference).unwrap();
    assert!(loaded.expose_for(|key| key.iter().all(|byte| *byte == winner)));
}
