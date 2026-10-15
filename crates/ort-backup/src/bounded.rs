//! Exact serialization budgets without allocating a second copy to measure it.
use crate::BackupError;
use serde::Serialize;
use std::io::{self, Write};

pub const MAX_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;

struct BoundedWriter<W> {
    inner: W,
    written: usize,
    limit: usize,
}
impl<W: Write> Write for BoundedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.written) {
            return Err(io::Error::other("portable serialization budget exceeded"));
        }
        self.inner.write_all(bytes)?;
        self.written += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Counts canonical JSON bytes, stopping as soon as the limit is exceeded.
/// # Errors
/// Rejects invalid values and exhausted budgets without allocating JSON output.
pub fn serialized_size<T: Serialize + ?Sized>(
    value: &T,
    limit: usize,
) -> Result<usize, BackupError> {
    let mut writer = BoundedWriter {
        inner: io::sink(),
        written: 0,
        limit,
    };
    serde_json::to_writer(&mut writer, value).map_err(|_| BackupError::InvalidContent)?;
    Ok(writer.written)
}

pub(crate) fn to_vec<T: Serialize + ?Sized>(
    value: &T,
    limit: usize,
) -> Result<Vec<u8>, BackupError> {
    let mut writer = BoundedWriter {
        inner: Vec::new(),
        written: 0,
        limit,
    };
    serde_json::to_writer(&mut writer, value).map_err(|_| BackupError::InvalidContent)?;
    Ok(writer.inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::ser::SerializeSeq;
    struct TooMany(std::cell::Cell<usize>);
    impl Serialize for TooMany {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut sequence = serializer.serialize_seq(None)?;
            for _ in 0..1_000_000 {
                self.0.set(self.0.get() + 1);
                sequence.serialize_element(&"0123456789")?;
            }
            sequence.end()
        }
    }
    #[test]
    fn counting_and_output_stop_before_visiting_unbounded_input() {
        let value = TooMany(std::cell::Cell::new(0));
        assert!(serialized_size(&value, 1024).is_err());
        assert!(value.0.get() < 100);
        value.0.set(0);
        assert!(to_vec(&value, 1024).is_err());
        assert!(value.0.get() < 100);
        assert_eq!(serialized_size(&"abc", 5).unwrap(), 5);
        assert_eq!(to_vec(&"abc", 5).unwrap(), b"\"abc\"");
    }
}
