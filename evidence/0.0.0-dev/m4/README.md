# M4 implementation evidence

The overlay adds a recoverable application-material workspace while leaving the
published master and main resume editor in place. Direct AI requests use the
existing credential, catalog, cap, cancellation, and accounting boundary.
Versioned tailoring responses select published IDs; the validator resolves
facts from the exact published revision, bounds change points to three, and
accepts only supported Required Qualification Alerts with a matching job span.
Cover letters and answers are assembled from selected published evidence and
remain editable. The overlay includes resume, cover-letter, and answer review,
PDF preview/download, native macOS PDF drag-out through private temporary files,
and explicit Finish Application cleanup. Tracker retention and browser capture
belong to M5.

## Checks run on 2026-09-22

- `cargo test -p ort-ai --lib --offline --quiet`: 17 passed. Fixtures cover
  malformed and fabricated tailoring, exact source selection, required versus
  preferred alerts, typed mismatch evidence, language alerts, personal questions,
  and unsupported evidence IDs.
- `cargo test -p ort-desktop --lib --offline --quiet`: 57 passed. This includes
  encrypted workspace recovery/revision checks, a generated proposal to user
  edit to PDF render journey, cover-letter schema validation, and private PDF
  drag-file permissions and cleanup.
- `cargo clippy -p ort-desktop --lib --all-targets --offline -- -D warnings`:
  passed.
- `pnpm --filter @ort/desktop test`: 28 files, 152 tests passed.
- `pnpm --filter @ort/desktop lint` and `pnpm --filter @ort/desktop build:web`:
  passed.

The user requested that testing in the app stop. No native UI walkthrough,
live-provider generation, or browser file-drop acceptance test was run after
that request. The automated journey checks the local validator, encrypted
edits, and PDF rendering; native drag initiation is compiled and its private
file lifecycle is checked without opening the app.
