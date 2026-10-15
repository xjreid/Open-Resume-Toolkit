//! Private drag artifacts with an OS lease for crash-safe retention cleanup.
use std::fs::{File, OpenOptions, Permissions};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::time::{Duration, SystemTime};

const OWNER: &str = ".ort-drag-owner";
const RETENTION: Duration = Duration::from_hours(24);

pub(crate) struct DragArtifact {
    _ownership: File,
    directory: tempfile::TempDir,
}
impl DragArtifact {
    pub(crate) fn new() -> std::io::Result<Self> {
        Self::create_in(&std::env::temp_dir())
    }
    fn create_in(root: &Path) -> std::io::Result<Self> {
        let directory = tempfile::Builder::new()
            .prefix("ort-drag-")
            .permissions(Permissions::from_mode(0o700))
            .tempdir_in(root)?;
        let ownership = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(directory.path().join(OWNER))?;
        ownership.try_lock().map_err(std::io::Error::other)?;
        Ok(Self {
            _ownership: ownership,
            directory,
        })
    }
    pub(crate) fn path(&self) -> &Path {
        self.directory.path()
    }
}

pub(crate) fn sweep_stale() {
    // Legacy artifacts have no lease. Only a previous boot proves their creator
    // cannot still be using them; failed boot-time detection preserves them.
    let boot = std::process::Command::new("/usr/sbin/sysctl")
        .args(["-n", "kern.boottime"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|text| {
            text.split("sec = ")
                .nth(1)?
                .split(',')
                .next()?
                .trim()
                .parse::<u64>()
                .ok()
        })
        .and_then(|seconds| SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(seconds)));
    sweep_in(&std::env::temp_dir(), SystemTime::now(), boot);
}

fn sweep_in(root: &Path, now: SystemTime, boot: Option<SystemTime>) {
    let Ok(probe) = tempfile::tempdir_in(root) else {
        return;
    };
    let Ok(owner) = probe.path().metadata().map(|meta| meta.uid()) else {
        return;
    };
    let Ok(children) = std::fs::read_dir(root) else {
        return;
    };
    for child in children.flatten() {
        let path = child.path();
        let Some(suffix) = child
            .file_name()
            .to_str()
            .and_then(|name| name.strip_prefix("ort-drag-"))
            .map(str::to_owned)
        else {
            continue;
        };
        if suffix.len() != 6 || !suffix.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            continue;
        }
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if !meta.is_dir()
            || meta.uid() != owner
            || now
                .duration_since(modified)
                .map_or(true, |age| age < RETENTION)
        {
            continue;
        }
        match std::fs::symlink_metadata(path.join(OWNER)) {
            Ok(lock_meta) => {
                if meta.mode() & 0o777 != 0o700
                    || !lock_meta.is_file()
                    || lock_meta.file_type().is_symlink()
                    || lock_meta.uid() != owner
                    || lock_meta.nlink() != 1
                    || lock_meta.mode() & 0o777 != 0o600
                {
                    continue;
                }
                let Ok(lock) = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(path.join(OWNER))
                else {
                    continue;
                };
                if lock.try_lock().is_err() {
                    continue;
                }
                let _ = std::fs::remove_dir_all(&path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if !matches!(meta.mode() & 0o777, 0o700 | 0o755)
                    || boot.is_none_or(|boot| modified >= boot)
                    || !legacy_contents(&path, owner)
                {
                    continue;
                }
                let _ = std::fs::remove_dir_all(&path);
            }
            _ => {}
        }
    }
}

fn legacy_contents(root: &Path, owner: u32) -> bool {
    let Ok(mut children) = std::fs::read_dir(root) else {
        return false;
    };
    children.all(|child| {
        let Ok(child) = child else {
            return false;
        };
        let Ok(meta) = std::fs::symlink_metadata(child.path()) else {
            return false;
        };
        let valid_id = child.file_name().to_str().is_some_and(|name| {
            uuid::Uuid::parse_str(name)
                .is_ok_and(|id| id.get_version_num() == 7 && id.to_string() == name)
        });
        if !valid_id || !meta.is_dir() || meta.uid() != owner || meta.mode() & 0o777 != 0o700 {
            return false;
        }
        let Ok(mut files) = std::fs::read_dir(child.path()) else {
            return false;
        };
        files.all(|file| {
            let Ok(file) = file else {
                return false;
            };
            let Ok(meta) = std::fs::symlink_metadata(file.path()) else {
                return false;
            };
            let allowed = file.file_name().to_str().is_some_and(|name| {
                matches!(
                    name,
                    "tailored-resume.pdf"
                        | "tailored-resume.docx"
                        | "cover-letter.pdf"
                        | "cover-letter.docx"
                )
            });
            allowed
                && meta.is_file()
                && !meta.file_type().is_symlink()
                && meta.uid() == owner
                && meta.nlink() == 1
                && meta.mode() & 0o777 == 0o600
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn age(root: &Path, time: SystemTime) {
        File::open(root)
            .unwrap()
            .set_times(std::fs::FileTimes::new().set_modified(time))
            .unwrap();
    }
    #[test]
    fn roots_are_private_and_cleanup_preserves_live_and_unrelated_directories() {
        let temp = tempfile::tempdir().unwrap();
        let live = DragArtifact::create_in(temp.path()).unwrap();
        assert_eq!(live.path().metadata().unwrap().mode() & 0o777, 0o700);
        let old = SystemTime::now() - RETENTION - Duration::from_secs(1);
        age(live.path(), old);
        let stale = DragArtifact::create_in(temp.path()).unwrap();
        let stale_path = stale.path().to_owned();
        let DragArtifact {
            _ownership: ownership,
            directory,
        } = stale;
        let _ = directory.keep();
        drop(ownership); // Simulate process death without TempDir destruction.
        age(&stale_path, old);
        let unrelated = temp.path().join("ort-drag-unrelated");
        std::fs::create_dir(&unrelated).unwrap();
        age(&unrelated, old);
        sweep_in(temp.path(), SystemTime::now(), None);
        assert!(live.path().exists());
        assert!(!stale_path.exists());
        assert!(unrelated.exists());
    }
    #[test]
    fn legacy_cleanup_requires_previous_boot_and_exact_owned_contents() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = tempfile::Builder::new()
            .prefix("ort-drag-")
            .tempdir_in(temp.path())
            .unwrap();
        std::fs::set_permissions(legacy.path(), Permissions::from_mode(0o755)).unwrap();
        let old = SystemTime::now() - Duration::from_hours(48);
        age(legacy.path(), old);
        sweep_in(
            temp.path(),
            SystemTime::now(),
            Some(old - Duration::from_secs(1)),
        );
        assert!(legacy.path().exists());
        sweep_in(
            temp.path(),
            SystemTime::now(),
            Some(old + Duration::from_secs(1)),
        );
        assert!(!legacy.path().exists());
    }
}
