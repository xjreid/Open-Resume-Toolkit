# Installed macOS development app refresh and PDF smoke

Date: 2026-09-03 UTC (2026-09-02 local). Platform: macOS arm64.
Built source: `296610a49461363ff47da6353558ea5b35ebf3f4` (clean worktree).
The user reported all four CI jobs passing; no CI URL/log was independently
retrieved. This is a local development installation, not a release qualification.

## Build and installation

- Read the development packaging instructions and existing evidence. Verified
  the development identity before building. Ran:

  ```sh
  export PATH=/opt/homebrew/opt/rustup/bin:$PATH
  node tools/assert-dev-profile.mjs
  pnpm --filter @ort/desktop tauri build \
    --config src-tauri/tauri.preview.conf.json \
    --bundles app -- --locked --offline
  ```

- Release compilation, frontend build and `.app` packaging succeeded. The first
  invocation incorrectly placed `--offline` before Tauri's argument separator;
  it exited before building and was corrected as shown. Build output is retained
  locally in ignored `target/dev-app-refresh-build.log`.
- Verified arm64, `com.openresumetoolkit.dev`, ad-hoc hardened-runtime signature,
  and `codesign --verify --deep --strict`. No notarization or DMG refresh was
  performed. Executable SHA-256:
  `ecd01fa08ef6e5d82a62b7024dac56d27149bfa426480291dc398c6a8034b88a`.
- With explicit user permission, quit both older running copies using Command-Q.
  The workspace copy showed saved draft revision 13 and published snapshot 2;
  the older installed copy had a blank window. No edits were made or discarded.
- Backed up both previous bundles and compared executable hashes. Local rollback
  copies are under ignored
  `target/dev-app-backups/refresh-296610a-1IPRpR/` (`Installed previous.app`,
  `Workspace previous.app`, and the retained `Installed original.app`).
- Staged the new bundle separately in `/Applications`, verified its signature and
  executable equality, retained the original, then renamed the staged bundle to
  `/Applications/Open Resume Toolkit Dev.app`. Reverified the installed signature
  and hash. No profile directory, saved record, Keychain item or access policy was
  deliberately changed. Normal app startup accessed its existing dev vault; no
  Keychain permission/password prompt was needed.

## Native checks actually run

- Launched the exact installed bundle. It displayed **Encrypted storage ready**,
  **Saved**, draft revision **13**, and published snapshot **2**, with the same
  existing synthetic resume. No save or publication was requested.
- Closed only the separate gated browser-overlay window to show the main editor.
- **Preview saved draft** rendered the one-page synthetic resume successfully in
  the actual packaged WKWebView. Inspected the displayed canvas at 100% and 150%
  zoom; the app reported **Preview ready** and enabled PDF export. This supplies
  native custom-scheme/local-worker evidence for this build and measured case,
  beyond the previous loopback Chrome mock-command test.
- Expanded the accessible-text section and checked that the synthetic name,
  contact link, role, organization, custom skill and bullet were exposed. This
  was an accessibility-tree/UI check, not a VoiceOver qualification.
- Opened **Export this preview (.pdf)**. The native Save panel proposed
  `resume.pdf`; selected **Cancel**, after which the app reported no file created.
  The draft and published revision indicators remained unchanged.
- Two automation scroll calls reported `noWindowsAvailable`; a scoped process
  check confirmed the same installed process remained running. Reconnecting and
  clicking controls worked, with the preview intact. No matching ORT crash report
  was found. Scrolling is not claimed as verified by those failed calls.
- Left the refreshed installed application open for user inspection. The preview
  remains session-only and expires after ten minutes; it can be regenerated.

No full `just check`, renderer golden corpus, native file-write/overwrite test,
DOCX reader test or Windows test was rerun during this install-only checkpoint.
Those prior checks remain documented in their respective evidence; current CI
success is user-reported. Documentation validation uses JSON parsing, formatting
checks and `git diff --check`.

## Remaining gates

Multi-page navigation, remaining zoom/keyboard/VoiceOver cases, native PDF new-file
and existing-file outcomes, quit during render/dialog, stale/expired/failure
cases, and Windows WebView2/vault/filesystem behavior still need native evidence.
Signed identity/update behavior is not established by this ad-hoc dev update.
Import remains disabled (`IMPORT_ENABLED=false`, document worker exit 78);
containment, replacement/recovery, storage tools and the complete offline journey
remain M2 work. SQLCipher logging mitigations and encryption/memory protection
were not changed. No application source was modified for this refresh.

Suggested commit: `docs(testing): record refreshed macOS dev app and native PDF smoke`
