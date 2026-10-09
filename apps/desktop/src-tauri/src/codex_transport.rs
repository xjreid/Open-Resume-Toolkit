//! Deadline-aware writes: pipe backpressure must never prevent session retirement.
use std::{
    io::{ErrorKind, Write},
    process::ChildStdin,
    time::{Duration, Instant},
};

pub(crate) struct Writer(ChildStdin);
impl Writer {
    pub(crate) fn new(stdin: ChildStdin) -> Result<Self, &'static str> {
        #[cfg(unix)]
        {
            use rustix::fs::{OFlags, fcntl_getfl, fcntl_setfl};
            let flags = fcntl_getfl(&stdin).map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
            fcntl_setfl(&stdin, flags | OFlags::NONBLOCK)
                .map_err(|_| "PLAN_RUNTIME_UNAVAILABLE")?;
            Ok(Self(stdin))
        }
        #[cfg(not(unix))]
        {
            drop(stdin);
            Err("PLAN_PLATFORM_UNSUPPORTED")
        }
    }

    pub(crate) fn send(
        &mut self,
        mut bytes: &[u8],
        end: Instant,
        cancel: &dyn Fn() -> bool,
    ) -> Result<(), &'static str> {
        while !bytes.is_empty() {
            if cancel() {
                return Err("AI_CANCELLED");
            }
            if Instant::now() >= end {
                return Err("PLAN_REQUEST_TIMEOUT");
            }
            match self.0.write(bytes) {
                Ok(0) => return Err("PLAN_RUNTIME_UNAVAILABLE"),
                Ok(count) => bytes = &bytes[count..],
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    std::thread::sleep(
                        Duration::from_millis(5).min(end.saturating_duration_since(Instant::now())),
                    );
                }
                Err(_) => return Err("PLAN_RUNTIME_UNAVAILABLE"),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn cancelled_or_expired_dispatch_writes_zero_bytes() {
        for cancelled in [true, false] {
            let mut child = Command::new("/bin/cat")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            let mut writer = Writer::new(child.stdin.take().unwrap()).unwrap();
            let end = if cancelled {
                Instant::now() + Duration::from_secs(1)
            } else {
                Instant::now()
            };
            assert_eq!(
                writer.send(b"must not be dispatched", end, &|| cancelled),
                Err(if cancelled {
                    "AI_CANCELLED"
                } else {
                    "PLAN_REQUEST_TIMEOUT"
                })
            );
            drop(writer);
            let result = child.wait_with_output().unwrap();
            assert!(result.status.success());
            assert!(result.stdout.is_empty());
        }
    }
}
