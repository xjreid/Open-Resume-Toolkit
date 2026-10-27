//! GHSA-qwgh-2vcv-g2f7 affects the inline-cursor implementation (>=0.11.0).
//! Exercise the actual 0.10.4 dependency after a caught callback panic, using
//! the adversarial bytes from the maintainer's reproduction. The independent
//! version guard in tools/tests/osv-policy.test.mjs forbids affected versions.

use block_buffer_legacy::{EagerBuffer, generic_array::typenum::U4};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn legacy_digest_cursor_survives_compression_panic_with_adversarial_bytes() {
    let mut buffer = EagerBuffer::<U4>::new(&[1, 2]);
    let result = catch_unwind(AssertUnwindSafe(|| {
        buffer.digest_blocks(&[3, 0xff], |_| panic!("synthetic compression failure"));
    }));
    assert!(result.is_err());
    assert_eq!(buffer.get_pos(), 2);
    assert_eq!(buffer.get_data(), &[1, 2]);
    buffer.reset();
    buffer.digest_blocks(&[4, 5, 6], |_| panic!("partial input must not compress"));
    assert_eq!(buffer.get_data(), &[4, 5, 6]);
}

#[test]
fn legacy_generated_block_panic_preserves_a_valid_cursor() {
    let mut buffer = EagerBuffer::<U4>::default();
    let mut output = [0; 1];
    let result = catch_unwind(AssertUnwindSafe(|| {
        buffer.set_data(&mut output, |blocks| {
            if let Some(block) = blocks.first_mut() {
                block[0] = 0xff;
                panic!("synthetic block generation failure");
            }
        });
    }));
    assert!(result.is_err());
    assert_eq!(buffer.get_pos(), 0);
    assert!(buffer.get_data().is_empty());
    buffer.set_data(&mut output, |blocks| {
        for block in blocks {
            block.fill(7);
        }
    });
    assert_eq!(output, [7]);
    assert_eq!(buffer.get_pos(), 1);
}
