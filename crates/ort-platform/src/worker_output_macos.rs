//! Safe, owned pipe driver for the future native containment adapter.
//! This component creates no worker and grants no launch/cleanup receipt.

use std::{os::fd::OwnedFd, time::Duration};

use ort_documents::{
    import::MAX_EXTRACTION_BYTES,
    import_transport::{MAX_PIPE_CHUNK_BYTES, MAX_STDERR_BYTES},
    worker_supervisor::{NativeAdapterError, NativeWorkerEvent, SUPERVISOR_POLL_INTERVAL},
};
use rustix::{
    event::{PollFd, PollFlags, Timespec, poll},
    fs::{FileType, OFlags, fcntl_getfl, fcntl_setfl, fstat},
    io::{Errno, FdFlags, fcntl_getfd, fcntl_setfd, read},
};

/// Two owned private pipe read ends. Output events fit the common supervisor;
/// stderr bytes are zeroed before delivery, preserving only their count.
/// OS-observed exit, containment, parent death and tree cleanup remain separate.
pub struct MacosWorkerOutput {
    pipes: [Option<OwnedFd>; 2],
    totals: [usize; 2],
    next_stream: usize,
    failed: bool,
}

impl MacosWorkerOutput {
    /// Takes ownership even on failure. The caller must transfer only its own
    /// private read ends; regular files, sockets and write ends are rejected.
    ///
    /// # Errors
    /// Returns a content-free transport error if pipe setup cannot be verified.
    pub fn new(stdout: OwnedFd, stderr: OwnedFd) -> Result<Self, NativeAdapterError> {
        for pipe in [&stdout, &stderr] {
            prepare(pipe).map_err(|_| NativeAdapterError::EventTransport)?;
        }
        Ok(Self {
            pipes: [Some(stdout), Some(stderr)],
            totals: [0, 0],
            next_stream: 0,
            failed: false,
        })
    }

    /// Polls at most the supervisor interval and reads at most one fixed chunk.
    /// Interruptions and readiness races return control without retrying.
    ///
    /// # Errors
    /// Byte-limit, I/O and allocation failures close both readers permanently.
    pub fn receive(
        &mut self,
        maximum_wait: Duration,
    ) -> Result<Option<NativeWorkerEvent>, NativeAdapterError> {
        if self.failed {
            return Err(NativeAdapterError::EventTransport);
        }
        let result = self.receive_once(maximum_wait);
        if result.is_err() {
            self.close();
        }
        result
    }

    /// Releases only these owned descriptors and makes subsequent reads fail.
    /// This is not proof of worker reaping or whole-containment cleanup.
    pub fn close(&mut self) {
        self.failed = true;
        self.pipes = [None, None];
    }

    fn receive_once(
        &mut self,
        maximum_wait: Duration,
    ) -> Result<Option<NativeWorkerEvent>, NativeAdapterError> {
        let ready = self.readiness(maximum_wait)?;
        for turn in 0..2 {
            let stream = (self.next_stream + turn) % 2;
            let flags = ready[stream];
            if flags.is_empty() {
                continue;
            }
            if flags.contains(PollFlags::NVAL)
                || !flags.intersects(PollFlags::IN | PollFlags::HUP | PollFlags::ERR)
            {
                return Err(NativeAdapterError::EventTransport);
            }
            let pipe = self.pipes[stream]
                .as_ref()
                .ok_or(NativeAdapterError::EventTransport)?;
            self.next_stream = (stream + 1) % 2;
            let limit = if stream == 0 {
                MAX_EXTRACTION_BYTES
            } else {
                MAX_STDERR_BYTES
            };
            let remaining = limit - self.totals[stream];
            // Read one extra byte at the limit, so overflow cannot masquerade as EOF.
            let capacity = MAX_PIPE_CHUNK_BYTES.min(remaining + 1);
            let mut bytes = [0; MAX_PIPE_CHUNK_BYTES];
            let count = match read(pipe, &mut bytes[..capacity]) {
                Ok(count) => count,
                Err(Errno::INTR | Errno::AGAIN) => return Ok(None),
                Err(_) => return Err(NativeAdapterError::EventTransport),
            };
            if count > remaining {
                return Err(NativeAdapterError::EventTransport);
            }
            if count == 0 {
                self.pipes[stream] = None;
                return Ok(Some(if stream == 0 {
                    NativeWorkerEvent::StdoutEof
                } else {
                    NativeWorkerEvent::StderrEof
                }));
            }
            self.totals[stream] += count;
            if stream == 1 {
                bytes.fill(0);
            }
            let mut chunk = Vec::new();
            chunk
                .try_reserve_exact(count)
                .map_err(|_| NativeAdapterError::EventTransport)?;
            chunk.extend_from_slice(&bytes[..count]);
            return Ok(Some(if stream == 0 {
                NativeWorkerEvent::Stdout(chunk)
            } else {
                NativeWorkerEvent::Stderr(chunk)
            }));
        }
        Ok(None)
    }

    fn readiness(&self, maximum_wait: Duration) -> Result<[PollFlags; 2], NativeAdapterError> {
        let wait = maximum_wait.min(SUPERVISOR_POLL_INTERVAL);
        let timeout = Timespec {
            tv_sec: 0,
            tv_nsec: wait.subsec_nanos().into(),
        };
        let mut ready = [PollFlags::empty(); 2];
        match (&self.pipes[0], &self.pipes[1]) {
            (Some(stdout), Some(stderr)) => {
                let mut descriptors = [
                    PollFd::new(stdout, PollFlags::IN),
                    PollFd::new(stderr, PollFlags::IN),
                ];
                poll_once(&mut descriptors, &timeout)?;
                ready = [descriptors[0].revents(), descriptors[1].revents()];
            }
            (Some(pipe), None) | (None, Some(pipe)) => {
                let mut descriptors = [PollFd::new(pipe, PollFlags::IN)];
                poll_once(&mut descriptors, &timeout)?;
                ready[usize::from(self.pipes[0].is_none())] = descriptors[0].revents();
            }
            (None, None) => {}
        }
        Ok(ready)
    }
}

fn poll_once(descriptors: &mut [PollFd<'_>], timeout: &Timespec) -> Result<(), NativeAdapterError> {
    match poll(descriptors, Some(timeout)) {
        Ok(_) => Ok(()),
        Err(Errno::INTR) => {
            for descriptor in descriptors {
                descriptor.clear_revents();
            }
            Ok(())
        }
        Err(_) => Err(NativeAdapterError::EventTransport),
    }
}

fn prepare(pipe: &OwnedFd) -> rustix::io::Result<()> {
    let flags = fcntl_getfl(pipe)?;
    if FileType::from_raw_mode(fstat(pipe)?.st_mode) != FileType::Fifo
        || flags & OFlags::ACCMODE != OFlags::RDONLY
    {
        return Err(Errno::INVAL);
    }
    fcntl_setfl(pipe, flags | OFlags::NONBLOCK)?;
    fcntl_setfd(pipe, fcntl_getfd(pipe)? | FdFlags::CLOEXEC)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Write, pipe},
        time::Instant,
    };

    fn next(reader: &mut MacosWorkerOutput) -> NativeWorkerEvent {
        reader
            .receive(Duration::ZERO)
            .expect("read")
            .expect("event")
    }

    #[test]
    fn real_pipes_are_fair_redacted_bounded_and_drained_before_single_eof() {
        let (out, mut out_write) = pipe().expect("stdout");
        let (err, mut err_write) = pipe().expect("stderr");
        let mut reader = MacosWorkerOutput::new(out.into(), err.into()).expect("reader");
        for fd in reader.pipes.iter().flatten() {
            assert!(fcntl_getfl(fd).expect("flags").contains(OFlags::NONBLOCK));
            assert!(fcntl_getfd(fd).expect("flags").contains(FdFlags::CLOEXEC));
        }
        out_write.write_all(b"synthetic extraction").expect("write");
        err_write
            .write_all(b"do not reveal this stderr")
            .expect("write");
        let stdout = next(&mut reader);
        assert!(!format!("{stdout:?}").contains("synthetic extraction"));
        assert!(
            matches!(stdout, NativeWorkerEvent::Stdout(bytes) if bytes == b"synthetic extraction")
        );
        out_write.write_all(b"more stdout").expect("write");
        assert!(
            matches!(next(&mut reader), NativeWorkerEvent::Stderr(bytes) if bytes.len() == 25 && bytes.iter().all(|byte| *byte == 0))
        );
        drop(out_write);
        drop(err_write);
        assert!(
            matches!(next(&mut reader), NativeWorkerEvent::Stdout(bytes) if bytes == b"more stdout")
        );
        assert!(matches!(next(&mut reader), NativeWorkerEvent::StderrEof));
        assert!(matches!(next(&mut reader), NativeWorkerEvent::StdoutEof));
        assert!(reader.receive(Duration::ZERO).expect("finished").is_none());
        assert!(reader.pipes.iter().all(Option::is_none));
    }

    #[test]
    fn silent_wait_is_clamped_and_explicit_close_is_terminal() {
        let (out, _out_write) = pipe().expect("stdout");
        let (err, _err_write) = pipe().expect("stderr");
        let mut reader = MacosWorkerOutput::new(out.into(), err.into()).expect("reader");
        let started = Instant::now();
        assert!(
            reader
                .receive(Duration::from_secs(60))
                .expect("wait")
                .is_none()
        );
        // Generous scheduler tolerance; this does not claim a real-time guarantee.
        assert!(started.elapsed() < Duration::from_secs(1));
        reader.close();
        reader.close();
        assert_eq!(
            reader.receive(Duration::ZERO).unwrap_err(),
            NativeAdapterError::EventTransport
        );
        assert!(reader.pipes.iter().all(Option::is_none));
    }

    #[test]
    fn exact_stdout_and_stderr_limits_accept_eof_but_reject_the_next_byte() {
        for stream in 0..2 {
            for overflow in [false, true] {
                let (out, mut out_write) = pipe().expect("stdout");
                let (err, mut err_write) = pipe().expect("stderr");
                let mut reader = MacosWorkerOutput::new(out.into(), err.into()).expect("reader");
                let limit = if stream == 0 {
                    MAX_EXTRACTION_BYTES
                } else {
                    MAX_STDERR_BYTES
                };
                let writer = if stream == 0 {
                    &mut out_write
                } else {
                    &mut err_write
                };
                for _ in 0..limit / MAX_PIPE_CHUNK_BYTES {
                    writer
                        .write_all(&[b'x'; MAX_PIPE_CHUNK_BYTES])
                        .expect("write");
                    assert!(
                        matches!(next(&mut reader), NativeWorkerEvent::Stdout(bytes) | NativeWorkerEvent::Stderr(bytes) if bytes.len() == MAX_PIPE_CHUNK_BYTES)
                    );
                }
                if overflow {
                    writer.write_all(b"!").expect("overflow byte");
                }
                drop(out_write);
                drop(err_write);
                if overflow {
                    // The other stream's EOF may be delivered first due to fairness.
                    if reader.receive(Duration::ZERO).is_ok() {
                        assert!(reader.receive(Duration::ZERO).is_err());
                    }
                    assert!(reader.receive(Duration::ZERO).is_err());
                    assert!(reader.pipes.iter().all(Option::is_none));
                } else {
                    let first = next(&mut reader);
                    let second = next(&mut reader);
                    assert!(matches!(
                        (first, second),
                        (NativeWorkerEvent::StdoutEof, NativeWorkerEvent::StderrEof)
                            | (NativeWorkerEvent::StderrEof, NativeWorkerEvent::StdoutEof)
                    ));
                }
            }
        }
    }

    #[test]
    fn oversized_burst_is_split_into_fixed_chunks() {
        let (out, mut writer) = pipe().expect("stdout");
        let (err, err_writer) = pipe().expect("stderr");
        let mut reader = MacosWorkerOutput::new(out.into(), err.into()).expect("reader");
        drop(err_writer);
        let producer =
            std::thread::spawn(move || writer.write_all(&[b'x'; MAX_PIPE_CHUNK_BYTES + 1]));
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut total = 0;
        let mut chunks = 0;
        loop {
            assert!(Instant::now() < deadline, "bounded synthetic test deadline");
            match reader.receive(SUPERVISOR_POLL_INTERVAL).expect("read") {
                Some(NativeWorkerEvent::Stdout(bytes)) => {
                    assert!(!bytes.is_empty() && bytes.len() <= MAX_PIPE_CHUNK_BYTES);
                    assert!(bytes.iter().all(|byte| *byte == b'x'));
                    total += bytes.len();
                    chunks += 1;
                }
                Some(NativeWorkerEvent::StdoutEof) => break,
                None | Some(NativeWorkerEvent::StderrEof) => {}
                _ => panic!("unexpected synthetic event"),
            }
        }
        producer.join().expect("writer thread").expect("write");
        assert_eq!(total, MAX_PIPE_CHUNK_BYTES + 1);
        assert!(chunks >= 2);
    }

    #[test]
    fn invalid_input_releases_both_owned_readers_and_drop_preserves_other_handles() {
        let (socket, _other) = std::os::unix::net::UnixStream::pair().expect("socket pair");
        let (err, mut err_write) = pipe().expect("stderr");
        assert!(MacosWorkerOutput::new(socket.into(), err.into()).is_err());
        assert!(err_write.write(b"x").is_err());
        let file = tempfile::tempfile().expect("regular file");
        let (err, mut err_write) = pipe().expect("stderr");
        assert!(MacosWorkerOutput::new(file.into(), err.into()).is_err());
        assert_eq!(
            err_write.write(b"x").unwrap_err().kind(),
            std::io::ErrorKind::BrokenPipe
        );
        let (out, out_write) = pipe().expect("stdout");
        let (err, _err_write) = pipe().expect("stderr");
        assert!(MacosWorkerOutput::new(out_write.into(), err.into()).is_err());
        drop(out);
        let (out, mut out_write) = pipe().expect("stdout");
        let (err, mut err_write) = pipe().expect("stderr");
        let unrelated = tempfile::tempfile().expect("unrelated");
        drop(MacosWorkerOutput::new(out.into(), err.into()).expect("reader"));
        assert!(out_write.write(b"x").is_err());
        assert!(err_write.write(b"x").is_err());
        unrelated.sync_all().expect("unrelated handle remains open");
    }
}
