use std::time::{Duration, Instant};

use ort_documents::import::{ImportError, InputFormat, MAX_EXTRACTION_BYTES};
use ort_documents::import_transport::{
    ExtractionTransport, MAX_PIPE_CHUNK_BYTES, MAX_STDERR_BYTES, TransportError, TransportProgress,
    WORKER_WALL_LIMIT, WorkerEvent,
};

const WIRE: &[u8] = br#"{"version":1,"format":"docx","pageCount":1,"blocks":[{"page":1,"kind":"paragraph","text":"SYNTHETIC_PRIVATE_MARKER"}]}"#;

fn send(transport: &mut ExtractionTransport, bytes: &[u8], now: Instant) {
    for chunk in bytes.chunks(MAX_PIPE_CHUNK_BYTES) {
        transport.observe(WorkerEvent::Stdout(chunk), now).unwrap();
    }
}

fn eof_and_exit(transport: &mut ExtractionTransport, now: Instant) {
    transport.observe(WorkerEvent::StdoutEof, now).unwrap();
    transport.observe(WorkerEvent::StderrEof, now).unwrap();
    assert_eq!(
        transport.observe(WorkerEvent::Exited { code: Some(0) }, now),
        Ok(TransportProgress::ReadyForCleanup)
    );
}

#[test]
fn all_chunk_splits_and_exit_before_or_after_pipe_drain_work() {
    let now = Instant::now();
    for split in 1..WIRE.len() {
        for exit_first in [false, true] {
            let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
            if exit_first {
                transport
                    .observe(WorkerEvent::Exited { code: Some(0) }, now)
                    .unwrap();
            }
            send(&mut transport, &WIRE[..split], now);
            transport
                .observe(WorkerEvent::Stderr(b"SYNTHETIC_SECRET"), now)
                .unwrap();
            send(&mut transport, &WIRE[split..], now);
            transport.observe(WorkerEvent::StdoutEof, now).unwrap();
            transport.observe(WorkerEvent::StderrEof, now).unwrap();
            if !exit_first {
                transport
                    .observe(WorkerEvent::Exited { code: Some(0) }, now)
                    .unwrap();
            }
            assert_eq!(
                transport.poll(now, false),
                Ok(TransportProgress::ReadyForCleanup)
            );
            assert_eq!(
                transport.finish(now, false).unwrap().blocks()[0].text,
                "SYNTHETIC_PRIVATE_MARKER"
            );
        }
    }
}

#[test]
fn stdout_byte_limit_is_checked_before_copy_and_errors_are_sticky() {
    let now = Instant::now();
    let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
    let mut exact = WIRE.to_vec();
    exact.resize(MAX_EXTRACTION_BYTES, b' ');
    send(&mut transport, &exact, now);
    eof_and_exit(&mut transport, now);
    assert!(transport.finish(now, false).is_ok());

    let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
    send(&mut transport, &exact, now);
    assert_eq!(
        transport.observe(WorkerEvent::Stdout(b" "), now),
        Err(TransportError::OutputLimit)
    );
    assert!(format!("{transport:?}").contains("stdout_bytes: 0"));
    assert_eq!(
        transport.observe(WorkerEvent::StdoutEof, now),
        Err(TransportError::OutputLimit)
    );
    assert_eq!(transport.poll(now, true), Err(TransportError::OutputLimit));
    assert_eq!(
        transport.finish(now, false).unwrap_err(),
        TransportError::OutputLimit
    );
}

#[test]
fn oversized_individual_chunks_and_stderr_floods_fail_without_retaining_stderr() {
    let now = Instant::now();
    for stdout in [false, true] {
        let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
        let oversized = vec![b'x'; MAX_PIPE_CHUNK_BYTES + 1];
        let event = if stdout {
            WorkerEvent::Stdout(&oversized)
        } else {
            WorkerEvent::Stderr(&oversized)
        };
        assert_eq!(
            transport.observe(event, now),
            Err(TransportError::OutputLimit)
        );
    }
    let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
    send(&mut transport, WIRE, now);
    for chunk in vec![b'x'; MAX_STDERR_BYTES].chunks(MAX_PIPE_CHUNK_BYTES) {
        transport.observe(WorkerEvent::Stderr(chunk), now).unwrap();
    }
    assert_eq!(
        transport.observe(WorkerEvent::Stderr(b"x"), now),
        Err(TransportError::OutputLimit)
    );
    assert!(!format!("{transport:?}").contains("SYNTHETIC_PRIVATE_MARKER"));
}

#[test]
fn no_result_before_both_pipe_eofs_and_successful_os_exit() {
    let now = Instant::now();
    for missing in 0..3 {
        let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
        send(&mut transport, WIRE, now);
        if missing != 0 {
            transport.observe(WorkerEvent::StdoutEof, now).unwrap();
        }
        if missing != 1 {
            transport.observe(WorkerEvent::StderrEof, now).unwrap();
        }
        if missing != 2 {
            transport
                .observe(WorkerEvent::Exited { code: Some(0) }, now)
                .unwrap();
        }
        assert_eq!(transport.poll(now, false), Ok(TransportProgress::Pending));
        assert_eq!(
            transport.finish(now, false).unwrap_err(),
            TransportError::Protocol
        );
    }
    for code in [None, Some(1), Some(78), Some(-1)] {
        let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
        send(&mut transport, WIRE, now);
        assert_eq!(
            transport.observe(WorkerEvent::Exited { code }, now),
            Err(TransportError::WorkerFailed)
        );
        assert_eq!(
            transport.finish(now, false).unwrap_err(),
            TransportError::WorkerFailed
        );
    }
}

#[test]
fn silence_slow_trickle_and_late_exit_cannot_extend_deadline() {
    let now = Instant::now();
    for produce_output in [false, true] {
        let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
        if produce_output {
            send(&mut transport, WIRE, now);
        }
        assert!(
            transport
                .poll(
                    now + WORKER_WALL_LIMIT
                        .checked_sub(Duration::from_nanos(1))
                        .unwrap(),
                    false
                )
                .is_ok()
        );
        assert_eq!(
            transport.poll(now + WORKER_WALL_LIMIT, false),
            Err(TransportError::TimedOut)
        );
    }
    let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
    for second in 0..60 {
        transport
            .observe(WorkerEvent::Stdout(b" "), now + Duration::from_secs(second))
            .unwrap();
    }
    assert_eq!(
        transport.observe(WorkerEvent::Stdout(b" "), now + WORKER_WALL_LIMIT),
        Err(TransportError::TimedOut)
    );
    let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
    send(&mut transport, WIRE, now);
    eof_and_exit(&mut transport, now);
    assert_eq!(
        transport
            .finish(now + WORKER_WALL_LIMIT, false)
            .unwrap_err(),
        TransportError::TimedOut
    );
}

#[test]
fn cancellation_io_failure_and_clock_regression_discard_even_complete_output() {
    let now = Instant::now();
    let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
    send(&mut transport, WIRE, now);
    eof_and_exit(&mut transport, now);
    assert_eq!(
        transport.finish(now, true).unwrap_err(),
        TransportError::Cancelled
    );
    let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
    send(&mut transport, WIRE, now);
    assert_eq!(
        transport.observe(WorkerEvent::IoFailure, now),
        Err(TransportError::Io)
    );
    assert_eq!(
        transport.finish(now, false).unwrap_err(),
        TransportError::Io
    );
    let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
    transport.poll(now + Duration::from_secs(1), false).unwrap();
    assert_eq!(transport.poll(now, false), Err(TransportError::Protocol));
}

#[test]
fn duplicate_events_and_data_after_eof_fail() {
    let now = Instant::now();
    for event in [
        WorkerEvent::StdoutEof,
        WorkerEvent::StderrEof,
        WorkerEvent::Exited { code: Some(0) },
        WorkerEvent::Stdout(b"x"),
        WorkerEvent::Stderr(b"x"),
    ] {
        let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
        send(&mut transport, WIRE, now);
        eof_and_exit(&mut transport, now);
        assert_eq!(transport.observe(event, now), Err(TransportError::Protocol));
    }
    for event in [WorkerEvent::Stdout(b""), WorkerEvent::Stderr(b"")] {
        let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
        assert_eq!(transport.observe(event, now), Err(TransportError::Protocol));
    }
}

#[test]
fn complete_transport_still_rejects_invalid_truncated_extra_and_mismatched_json() {
    let now = Instant::now();
    for wire in [
        b"".to_vec(),
        vec![0xff],
        WIRE[..WIRE.len() - 1].to_vec(),
        [WIRE, WIRE].concat(),
        [WIRE, b"trailing"].concat(),
    ] {
        let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
        send(&mut transport, &wire, now);
        eof_and_exit(&mut transport, now);
        assert_eq!(
            transport.finish(now, false).unwrap_err(),
            TransportError::Extraction(ImportError::InvalidExtraction)
        );
    }
    let mut transport = ExtractionTransport::new(InputFormat::Pdf, now);
    send(&mut transport, WIRE, now);
    eof_and_exit(&mut transport, now);
    assert_eq!(
        transport.finish(now, false).unwrap_err(),
        TransportError::Extraction(ImportError::InvalidExtraction)
    );
}

#[test]
fn unicode_can_cross_chunk_boundaries_and_debug_never_discloses_content() {
    let now = Instant::now();
    let wire = String::from_utf8(WIRE.to_vec())
        .unwrap()
        .replace("SYNTHETIC_PRIVATE_MARKER", "示例 — résumé");
    let mut transport = ExtractionTransport::new(InputFormat::Docx, now);
    for byte in wire.as_bytes() {
        transport
            .observe(WorkerEvent::Stdout(&[*byte]), now)
            .unwrap();
    }
    transport
        .observe(WorkerEvent::Stderr(b"PRIVATE_ERROR_WITH_PATH"), now)
        .unwrap();
    let debug = format!("{transport:?}");
    assert!(!debug.contains("résumé"));
    assert!(!debug.contains("PRIVATE_ERROR"));
    eof_and_exit(&mut transport, now);
    assert_eq!(
        transport.finish(now, false).unwrap().blocks()[0].text,
        "示例 — résumé"
    );
}
