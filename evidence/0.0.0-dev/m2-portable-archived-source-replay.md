# M2 portable archived-source PDF replay

Date: 2026-09-04. Base commit: `1087aef`; implementation changes are
uncommitted. Local verification platform: macOS arm64. Status: implemented and
synthetically verified; installed native-dialog, assistive-technology, and
cross-platform evidence remain pending. The user reported all five hosted CI
jobs passing for the base commit.

## Implemented boundary

- The PDF panel can select a format-1.1 `.ort-backup` through the native Open
  dialog using only a bounded passphrase request. JavaScript cannot provide a
  path, backup bytes, document, receipt, renderer, or export destination.
- Native code bounds and reads the selected regular file using the established
  backup input boundary, then authenticates and validates the complete
  Argon2id/XChaCha20-Poly1305 container before inspecting any portable record.
  Wrong passphrases, tampering, and unsupported content share the existing
  non-oracular invalid-backup result.
- After authentication, native code retains only the newest 20 render manifests
  whose exact draft or immutable-publication source is present. The session is
  identified by an opaque UUIDv7, replaces any earlier session, and expires on
  access after ten minutes. The response contains only that ticket, expiry,
  content-free receipts, and bounded unavailable/incompatible counts.
- Receipts naming a different renderer, template, or font bundle are reported
  as incompatible and are not offered for replay. Receipts without an exact
  retained source are also excluded. No newer document is substituted.
- Replay accepts only the opaque archive and manifest UUIDv7 values. It renders
  the retained source with the installed fixed bundle and compares every
  receipt field before exposing PDF bytes or bounded accessible text. It does
  not write a receipt or source into the active encrypted profile.
- A verified result uses the existing independent ten-minute preview ticket and
  exact-byte, no-overwrite PDF export. Clearing or expiring the archive drops
  its retained sources; clearing it does not invalidate an already verified
  preview.

## Privacy and lifecycle properties

- The selected path, passphrase, archive metadata, resume documents, settings,
  backup bytes, and native errors never return to the webview.
- At most one archive session and one generated PDF preview exist in native
  memory. Opening another archive releases the prior native archive state, and
  UI unmount, explicit clear, or expiry sends an identity-bound release.
- Opening, replaying, canceling, failing, clearing, or exporting does not
  restore, merge, replace, or otherwise mutate the active profile. The external
  backup is read-only.
- Superseded renderer binaries and historical PDF bytes remain unbundled. An
  incompatible receipt is inspection-only; this checkpoint does not promise
  universal historical binary replay.

## Verification

- Domain tests reject extra path/content fields, invalid passphrases, and
  noncanonical archive or manifest tickets.
- Native tests cover source/revision matching for archived drafts and
  publications, missing-source accounting, old-renderer filtering, the 20-entry
  bound, identity-safe release, replacement, and ten-minute expiry.
- An authenticated-container test creates a synthetic encrypted backup, opens
  its source session, and confirms that both a wrong passphrase and ciphertext
  tampering return the same invalid-backup class.
- Contract tests enforce exact response keys, UUIDv7 tickets, safe timestamps,
  100-record totals, 20 visible receipts, unavailable/incompatible accounting,
  unique manifest IDs, and content-free release responses.
- Desktop tests verify passphrase-only open, ticket-only replay/release, complete
  receipt correlation, no path authority, and truthful accessible UI copy.
- Full `just check` passes: formatting, TypeScript, 19 repository policy tests,
  20 generated-contract tests, 62 desktop tests, production frontend builds,
  source security checks, strict all-target/all-feature Clippy, and the complete
  Rust workspace suite. The desktop native suite includes 20 passing tests.

## Remaining native gates

- Exercise native Open cancel, valid/wrong passphrase, expired session, clear,
  replay, accessible review, and Save cancel/new/existing-name behavior in
  installed WKWebView and WebView2 builds.
- Confirm memory/lifecycle behavior under quit, interrupted dialogs, low-memory,
  and large valid backups on supported macOS and Windows versions.
- Run VoiceOver and NVDA through the new form, status messages, history list,
  verified preview, and export flow.
