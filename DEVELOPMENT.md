# Development

Open Resume Toolkit implements the M0 architecture skeleton, the local M1
encrypted-storage slice, and an M2 offline editor checkpoint. The development
app can autosave synthetic resume drafts and publish immutable snapshots through
its OS-vault-backed encrypted database, and export a saved draft or published
snapshot as unencrypted UTF-8 text, constrained DOCX, or the exact locally
previewed PDF through a native Save dialog. It can also create a passphrase-
protected portable backup of saved profile records through a native Save dialog.
It can authenticate and inspect an existing backup through a native Open dialog,
or stage it into a fresh encrypted replacement profile that is activated safely
on restart. PDF/DOCX import, AI, updater, and browser-native messaging remain
gated or unimplemented.

## Prerequisites

- macOS 10.15 or newer with Xcode Command Line Tools, or a supported Windows development environment with WebView2 and Microsoft C++ Build Tools;
- Node.js 24.16.0;
- pnpm 11.19.0 through Corepack;
- Rust 1.98.0 through rustup;
- `just` 1.x.

On macOS with Homebrew, install missing developer tools with:

```sh
brew install rustup just
export PATH="$(brew --prefix rustup)/bin:$PATH"
rustup toolchain install 1.98.0 --profile minimal --component clippy,llvm-tools,rustfmt
```

Add the Rust path export to your shell profile for future terminals, then run:

```sh
just bootstrap
just check
just dev
```

`just check` is the canonical offline-capable repository gate after bootstrap.
It validates the checked-in JavaScript and Rust dependency licenses against
`config/dependency-license-policy.json`, writes a machine-readable inventory to
`target/licenses/dependency-inventory.json`, regenerates contracts, and fails if
the checked-in bindings drift. License metadata for every locked target must
already be available locally when running without network access.

`just dev` always verifies the `com.openresumetoolkit.dev` identity before it starts. Development and test builds must use only synthetic data.

The main window reports `ready` after it opens the isolated development profile
under the platform application-data directory. Its database key is stored in
macOS Keychain or Windows Credential Manager. A vault or database failure leaves
the editor unavailable rather than creating a plaintext fallback or silently
replacing a key.

For the M2 editor manual check:

1. Enter synthetic contact information, add sections, entries, bullets, links,
   and custom fields; a custom field can be marked as a skill.
2. Pause for about 1.2 seconds and wait for **Saved**, or choose **Save draft**.
   Invalid content blocks both autosave and manual save; correct the inline
   errors before proceeding. Limits come from the generated Rust contracts.
3. Reorder sections/entries/bullets/fields with their up/down buttons (also
   available through Tab and Space). Verify **Undo edit** and **Redo edit**;
   history holds up to 30 document states in memory for this session only.
4. Choose **Publish snapshot**, then expand its read-only text review. Editing
   the draft must not change that review. Publishing is disabled for unsaved
   changes or content already matching the latest published snapshot.
5. Restart only after **Saved** and verify the draft, ordering, custom fields,
   and published snapshot return. See the recorded native smoke check in
   `evidence/0.0.0-dev/m2-editor-smoke.md`.

For the M2 quit-safety check, change the synthetic title and immediately use
Command-Q on macOS (Ctrl-Q on Windows), the application's Quit menu item, or the
main window's close button. Choose **Keep editing**, **Save and quit**, or
**Discard unsaved edits and quit**. Invalid edits disable Save and quit. Escape
keeps editing. A running save/publication/document export/portable backup is
allowed to finish before quit; failed saves keep the editor open. Closing the
main window currently quits the whole development workspace; closing the overlay
alone does not quit the editor.
See `evidence/0.0.0-dev/m2-close-guard-smoke.md` for verified paths and limitations.

The desktop test suite includes axe-core semantic checks for the complete
initial main and overlay routes, including PDF preview, quit, encrypted-backup,
recovery, storage, and all-local-data deletion controls. Live component cases
cover the loaded editor, required-field feedback, revision-conflict recovery,
quit-dialog focus restoration, exact destructive confirmation feedback,
announced recovery/deletion progress, and focus placement after a local-data
deletion outcome. Run the checks with:

```sh
pnpm --filter @ort/desktop test
```

The harness has a failing unnamed-button positive control and disables only the
color-contrast rule because jsdom cannot resolve native layout or colors. The
destructive commands remain synthetic and never access a profile or vault.
These checks do not replace keyboard, zoom, forced-color, reduced-motion,
VoiceOver, NVDA, WKWebView, WebView2, native-dialog, interruption, or
cross-platform testing. See
`evidence/0.0.0-dev/m2-desktop-accessibility-automation.md`.

For the M2 text-export check:

1. Wait for **Saved**, then choose **Export saved draft (.txt)**. Alternatively,
   choose **Export published snapshot (.txt)** to export the latest immutable
   snapshot; unsaved editor changes are never included in either choice.
2. Choose a private local folder and a **new** `.txt` filename. The file is
   unencrypted, outside the encrypted database, and may be uploaded by any sync
   service managing that folder. Only synthetic data is allowed in development.
3. Check the success notice and inspect the text. The internal title, IDs,
   revision metadata, empty fields, and app branding must not appear. Ordering,
   Unicode, custom field values, bullets, and link destinations must be preserved.
4. Cancel a second export: no file should be written and the editor should remain
   usable. Select an existing test filename: even if the OS offers **Replace**,
   this checkpoint refuses replacement and asks for a new filename.

Text output is bounded to 256 KiB and requires a filesystem supporting hard
links (for example APFS/NTFS); unsupported destinations fail closed, not through
an unsafe overwrite fallback. Normal completion removes its hidden sibling
`.ort-export-*` staging directory. Interruptions or filesystem errors can leave
staging plaintext in the chosen folder; automatic crash cleanup is not yet
implemented. Inspect the chosen folder before retrying an uncertain result.
Post-write cleanup/durability warnings distinguish an already-written file from
a failed export. Windows Save-dialog, ACL, and filesystem behavior still require
native VM verification; CI compilation alone does not prove those behaviors.
See `evidence/0.0.0-dev/m2-text-export-smoke.md` for checkpoint evidence.

## DOCX export checkpoint

In **Document export**, choose **Word document (.docx) — plain layout v1**,
then export either the saved draft or latest published snapshot. The same
revision checks, native Save dialog, one-operation gate, no-overwrite policy,
unencrypted-file warning and quit wait apply to both formats. Choose a new
`.docx` filename; `.docm`, special names and existing entries are refused.
Unsaved edits never substitute for the selected saved content. Canceling or
failing an export does not change drafts, snapshots or autosave state.

DOCX format v1 (`plain_docx_v1`) is an intentionally plain, editable,
single-column layout, not one of the final three template categories or a
preview of PDF output. It has semantic headings and real bullet lists, visible
link destinations and clickable allowlisted hyperlinks. Font availability and
pagination depend on the reader; inspect the document before sending it.
No font binaries, macros, fields, images or external templates are embedded.
Exports are capped at 2 MiB, with a separate 1 MiB XML expansion ceiling;
text exports retain their 256 KiB cap. Recovery/replacement and Windows-native
Save-dialog/ACL/reader verification remain gated.

Synthetic tests need no desktop launch, OS vault or document-reader installation:

```sh
cargo test --locked -p ort-documents -p ort-platform -p ort-application -p ort-desktop
cargo run --locked -p ort-documents --example docx_fixtures -- target/docx-review-fixtures
python3 tools/verify-docx-fixtures.py target/docx-review-fixtures
```

Use a **new** output directory each time; the fixture generator deliberately
refuses an existing directory. Python uses only its standard library (`python`
on Windows). The shared eight-case output corpus covers standard, sparse,
supported multilingual, code-like literal, dense four-page, omitted optional
data, multi-section/field/link ordering, and exact two-page resumes. All four CI
jobs independently check deterministic DOCX and plain-text bytes, package/CRC/
XML integrity, exact source-text parity, heading/list semantics and hyperlink
relationships. The shared plain-text golden also has to match the PDF fixture
path, so the three output formats cannot silently diverge.
Local headless-render and manual/platform limits are recorded in
`evidence/0.0.0-dev/m2-output-golden-corpus.md`. There is no production Python
or LibreOffice dependency; the exporter is Rust-only.

The first DOCX CI run passed three jobs but Windows failed the golden-byte
check after its Rust tests passed. Git's CRLF checkout conversion changed the
four XML assets embedded by `include_str!`. `.gitattributes` now pins only
those assets to LF; no global Git setting or golden hash change is required.
`node --test tools/tests/docx-checkout.test.mjs` exercises real Git checkouts
with positive/negative controls and runs early in all four CI jobs, as well as
through `just check`. Existing Windows working copies should use a fresh
checkout if these files still contain CRLF; preserve local edits first.
The verifier now identifies noncanonical line endings explicitly. The same
repair updates the pinned Node setup action to its Node 24 runtime; the project's
Node version stays unchanged. The user confirmed all four CI jobs passed for
`748d13b` before PDF development resumed; that result was not independently retrieved.
See `evidence/0.0.0-dev/windows-docx-checkout.md`.

## Local PDF preview and export

The main editor offers **Preview saved draft** and **Preview published snapshot**.
Save pending draft edits first. Rendering is manual and uses the selected exact
saved revision. The fixed `plain_pdf_v1` layout uses US Letter, one-inch margins,
11 pt Libertinus Serif, and no user-authored Typst or system fonts. The bundled
renderer is Typst 0.15.1; PDF.js 6.3.289 displays its exact output locally.

Review pages with Previous/Next and 100–200% zoom. The scrollable page is keyboard
focusable; an accessible, read-only content view follows it. PDF links are visible
text in the privileged preview, not clickable navigation. **Export this preview**
uses the same cached bytes, not a second render. It opens the native Save dialog
for a new `.pdf` filename and never replaces an existing file. Cancel creates no
file; committed cleanup/durability warnings remain distinct from failures.

Edits or newer saved revisions mark a draft preview stale and disable export.
Published preview revisions are independent of draft edits. Generate a fresh
preview after changes or ten-minute expiry. Clear/refresh/unmount releases the
preview; Rust holds at most one cached PDF and checks expiry on access. Renderer
reloads can leave that bounded cache until its next access or process exit; this
is not guaranteed timed memory erasure. No preview file is staged on disk.

Current development limits are 4 MiB, five pages, 800 content blocks and 200 hard
line breaks, in addition to document limits. Overflow, uncovered glyphs (covered
by an explicit negative control), compile warnings, and unsupported layouts are
explicit failures, never silent truncation or font substitution. Tabs become four
spaces. Text and DOCX remain alternatives. This original plain fixture is not the
final template catalogue. The fixed typography/language options and full bundled
licenses are visible in the panel. Each successful preview stores a content-free
receipt in the encrypted profile before the PDF is exposed to the UI. Repeated
identical source/revision/PDF identities increment a counter instead of adding
duplicate rows. Storage keeps the newest 100 identities and the UI shows the
newest 20; resume text, PDF bytes, paths and preview tickets are not retained.
For a receipt whose exact revision is still the current draft or any retained
immutable published snapshot, **Verify & replay** regenerates with the installed
bundle and exposes a preview only when every receipt field matches. An older
publication can be reviewed through bounded accessible text and exported from
the verified expiring preview. It never substitutes a newer revision or
different output. Superseded draft bodies are not retained, superseded renderer
binaries are not bundled, so those cases currently remain inspection-only. M2
still requires retained structured publications to be regenerable with the
current supported renderer under a new receipt that clearly identifies the
effective tuple and does not claim historical bytes.

Use **Replay from an encrypted portable backup** to select a format-1.1 backup
and enter its passphrase. ORT authenticates the complete archive on a blocking
native worker, then retains at most the newest 20 exact manifest/source pairs in
one memory-only session for ten minutes. It returns only an opaque archive
ticket, content-free receipt metadata, and counts for receipts whose source is
missing or whose renderer bundle differs. The selected path, passphrase, source
documents, settings, and backup metadata do not cross into the webview.

Choose **Verify archived receipt & replay** to regenerate one retained draft or
publication with the installed renderer. The preview is exposed only if every
receipt field matches; otherwise no PDF or accessible text is returned. A
successful replay supplies bounded accessible text and uses the existing
expiring, exact-byte, no-overwrite PDF export. Opening or replaying a backup does
not restore, merge, write to, or add render history to the active profile.
Clearing or expiring the archive session drops the retained sources; a verified
preview remains independently bounded by its own ten-minute ticket.

Headless structural verification, without launching the app or OS vault:

```sh
cargo run --locked -p ort-render --example pdf_fixtures -- target/pdf-review-fixtures
node tools/verify-pdf-fixtures.mjs target/pdf-review-fixtures
# Optional second independent parser (requires pypdf):
python3 tools/verify-pdf-fonts.py target/pdf-review-fixtures
```

Every CI runner checks all eight PDF golden bytes, exact shared plain-text parity,
fixed page counts, visible text/order, page geometry, semantic structure roles,
safe links, and absence of active content. New template sources retain LF bytes under Windows
checkout conversion. `tools/smoke-pdf-browser.mjs` performs optional headless
Chromium QA against a loopback production frontend with synthetic mocked native
commands; supply `ORT_PLAYWRIGHT_MODULE` / `ORT_BROWSER_EXECUTABLE` if needed.
It never proves native IPC, WKWebView/WebView2, save dialogs, vaults or ACLs.
See `evidence/0.0.0-dev/m2-output-golden-corpus.md` for the expanded output
checkpoint and `evidence/0.0.0-dev/m2-pdf-preview.md` for preview behavior and
manual gates. Portable archived-source replay is recorded in
`evidence/0.0.0-dev/m2-portable-archived-source-replay.md`.

## Encrypted portable backup export

Wait for **Saved**, then use **Create encrypted backup**. Enter the same synthetic
passphrase twice; the fields clear as soon as the operation begins. The passphrase
is never recoverable by ORT, so store it separately from the backup. The native
Save dialog accepts only a new `.ort-backup` filename. Existing entries, special
filenames, and last-moment target races fail without replacement. Canceling writes
nothing. A selected iCloud, OneDrive, Dropbox, or other synchronized folder sends
the encrypted archive to that provider under the user's account.

The format-1.1 archive contains the saved draft, immutable published snapshots,
portable settings, and bounded content-free PDF render manifests. It excludes the
SQLCipher/database key, vault reference, provider credentials, native IPC secrets,
diagnostics, PDF bytes, preview tickets, and unsaved editor changes. Creation uses
the existing Argon2id/XChaCha20-Poly1305 container and performs its KDF on a
blocking native worker. Only one file operation may run; quit waits for it. The
response contains format/byte and cleanup/durability receipts, never the path,
passphrase, content, or native error text.

## Delete all local data checkpoint

The **Encrypted profile storage** panel includes **Delete all local ORT data**.
This is destructive and requires the exact phrase `DELETE ALL LOCAL ORT DATA`.
It deletes the active encrypted profile, saved and unsaved resume state,
published snapshot, settings, render history, diagnostics, retained safety copy,
staged restore data, and every database-vault key identified by those exact
profiles. User-selected backups and exported PDF, DOCX, or text files are never
targeted, and the application is not uninstalled.

Deletion first closes the live SQLCipher store and validates every fixed target.
A private, durable intent marker is created before the first vault or filesystem
deletion. Once committed, interruption or a temporarily unavailable vault leaves
cleanup pending; startup must resume that exact cleanup before it can create a
new empty encrypted profile. Unsafe symlinks, unknown profile entries, malformed
manifests, or invalid markers fail closed. The interface distinguishes an action
that never started from committed cleanup that requires restart.

For a synthetic native check, first create an encrypted backup and save its
hash outside ORT. Then add a saved draft, published snapshot and PDF render
receipt. Type the exact deletion phrase and confirm that the editor reloads as a
new empty profile, storage counts return to zero, and the external backup remains
byte-for-byte unchanged. Restart once more and confirm the empty profile remains.
Native macOS/Windows vault failure, locked-vault, reparse/symlink, process-kill
and assistive-technology verification are still release gates; see
`evidence/0.0.0-dev/m2-all-local-data-deletion.md`.

The file publisher uses a held-directory capability, mode 0600 on Unix where
supported, an exact generated byte limit, a private sibling staging directory,
and a no-clobber hard-link commit. Unsupported filesystems fail closed. A crash or
post-write cleanup error can leave an encrypted `.ort-export-*` staging directory
in the selected folder; the UI reports that possibility.

Use **Select and check encrypted backup** to choose an existing `.ort-backup`
through the native Open dialog. The backend holds the selected parent directory,
refuses a symlink or non-regular final entry, bounds the file before allocation,
then authenticates and validates the complete archive on a blocking worker. The
passphrase field clears at dispatch. A successful result shows only format,
schema, creation time, application version, byte count, and bounded record counts;
the filename, path, passphrase, resume text, settings values, hashes, and native
errors never return to the webview. Wrong passphrases, damaged archives, and
unsupported encrypted content deliberately share one failure result.

Validation is read-only. To replace saved profile data, enter the backup
passphrase under **Replace saved profile from backup**, type
`REPLACE SAVED PROFILE`, and select the archive again. ORT authenticates it,
creates a fresh OS-vault database key, imports it into a private encrypted staging
profile, verifies integrity, and writes a content-free restart marker. The current
profile remains active and unchanged until the app is quit and reopened.

At the next startup, ORT renames the current encrypted profile to its fixed local
safety-copy slot, promotes the separately keyed staged profile, verifies it, and
then removes the marker. Startup resumes the handoff after interruption between
renames, or restores the old directory if the staged profile disappeared or the
promoted profile cannot open. The safety copy and its old vault key are retained;
external exports/backups are untouched. Restart promptly after staging and use
only synthetic data in development.

After activation, **Local recovery safety copy** reports only whether a retained
copy, restart operation, or cleanup operation exists. To return to the retained
profile, type `ROLL BACK SAVED PROFILE`. ORT verifies it, stages an encrypted
checkpoint, and swaps it in at the next restart; the current profile becomes the
new safety copy. To free the fixed safety slot, type `DELETE SAFETY COPY`. This
permanently removes only that encrypted directory and its OS-vault key. The active
profile and user-selected exports/backups are never included. An interrupted
confirmed cleanup resumes before the active database opens on the next startup.
See `evidence/0.0.0-dev/m2-portable-backup-export.md` and
`evidence/0.0.0-dev/m2-backup-validation.md`, plus the replacement checkpoint in
`evidence/0.0.0-dev/m2-replace-restore.md` and safety-copy management in
`evidence/0.0.0-dev/m2-safety-copy-management.md`.

## Encrypted profile storage usage

The **Encrypted profile storage** panel automatically reads a content-free
inventory after startup and after completed editor/file operations. Use
**Refresh usage** for an explicit reread. It shows the database schema and counts
for the master draft, immutable published snapshots, portable settings, PDF
render manifests, and bounded diagnostic records. It also shows exact bytes for
the encrypted database, encrypted write-ahead log, SQLite shared-memory file,
non-secret profile manifest, and transient manifest-recovery metadata.

The total covers only those fixed known files in the active profile. It does not
scan or count user-controlled exports/backups, OS-vault items, unrelated files,
or the in-memory PDF preview. WAL/shared-memory sizes can change after saves or
SQLite maintenance, so the panel labels the reading as refreshable rather than a
quota. The command accepts no path, profile identifier, category selector,
cleanup flag, or content-bearing value. This checkpoint is read-only: deletion,
vacuum/cleanup and all-local-data removal remain gated. Retained restore safety
copies are reported and managed through their separate content-free recovery
boundary, and their bytes are not included in this active-profile total. See
`evidence/0.0.0-dev/m2-storage-usage.md`.

## No-AI import core (backend-only checkpoint)

The deterministic mapper and in-memory review engine now have synthetic tests,
including an encrypted database commit/restart test. They do not enable a new
button or file import in the app yet. A constrained DOCX parser is linked only
into the worker library. A pinned PDFium text adapter is now also linked only
there, while the worker executable remains inert until native containment is
proven. No standalone `.txt` importer or AI transmission was added.

Run this checkpoint without starting the desktop or accessing an OS vault:

```sh
cargo test --locked -p ort-documents -p ort-application -p ort-document-worker
```

The storage integration test uses temporary synthetic profiles and an in-memory
vault. It checks explicit review, preservation of published snapshots, stale
commit/replay refusal, and restart. Mapping tests preserve unknown/multilingual
source text and reject malformed/oversized worker responses. Review changes are
not automatically written: the prepared result must use the existing storage
revision check. See `evidence/0.0.0-dev/m2-import-core.md` for limits and remaining
UI/parser integration work. No new installer or preview package is needed to
exercise this backend checkpoint.

The following transport-policy checkpoint adds bounded output collection,
discarded/capped stderr, deadline/cancellation checks and successful-exit/EOF
gating. The subsequent production coordinator in
`ort-documents::worker_supervisor` composes that policy with a native-adapter
contract for macOS XPC/App Sandbox and Windows AppContainer/Job containment.
It always requests whole-containment termination and accepts valid extraction
only after verified reaping, empty-tree, pipe/handle closure and teardown.
Adversarial mocks cover missing launch controls, weak resource ceilings,
cancellation, silence, floods, malformed/failed output, native event failure,
termination failure and every missing cleanup fact on both platform profiles.
The following bounded source-envelope checkpoint also adds
`ort-platform::read_native_document` and `ort-documents::import_source`: a held,
no-follow, 10 MiB PDF/DOCX snapshot plus signature/container preflight over the
same bytes. DOCX metadata inspection does not decompress or parse XML and rejects
unsafe/duplicate paths, encryption, known active parts, ZIP64/multidisk packages
and declared expansion over 100:1. The real synthetic DOCX output corpus and
adversarial malformed envelopes are covered. At that checkpoint it did not yet
implement private worker staging, native pipes, OS containment or a content parser.
See `evidence/0.0.0-dev/m2-import-transport.md` and
`evidence/0.0.0-dev/m2-parser-supervision-core.md`,
`evidence/0.0.0-dev/m2-import-source-envelope.md`, plus
`Implementation Plans/System Documentation/Document_Worker_Containment.md`.

The next private-staging subset adds Unix `0700` operation directories, a `0600`
fixed source, one transferred read-only handle, exact cleanup and a conservative
24-hour/128-entry startup scavenger. The application binds preflight and staging
to the same owned bytes and defines future launch/supervision/cleanup ordering;
the public path still cleans and returns disabled before any launcher call. A
parser-side builder now enforces the extraction protocol limits before encoding.
Windows staging deliberately fails closed pending ACL/reparse implementation.
See `evidence/0.0.0-dev/m2-import-private-staging.md` and ADR 0009.

The next disabled worker-parser checkpoint adds bounded ZIP32 store/deflate and
streaming WordprocessingML extraction in `ort-document-worker`. It independently
checks package identity, CRC/local metadata, content types, root/document
relationships, XML complexity, active content and extraction limits. It parses
the shipping fixed DOCX export in a round-trip integration test, reports one
logical page without claiming DOCX layout fidelity, and returns the existing OCR-
unavailable result for image-only/empty content. The executable still exits 78;
see `evidence/0.0.0-dev/m2-docx-worker-parser.md` and ADR 0010.

The following disabled PDF checkpoint pins `pdfium-render` 0.9.3 to the
`pdfium_7881` API and records exact non-V8/non-XFA PDFium 151.0.7881.0 archive
and extracted-library identities for macOS ARM64/x64 and Windows ARM64/x64.
The adapter loads only an absolute target-matching library whose size and hash
match, with no system fallback. It parses only bounded in-memory input, caps
pages, top-level objects and text, maps literal lines conservatively, and rejects
image-only or partially scanned content without OCR. Pure adversarial tests and
an opt-in pinned macOS ARM64 native smoke pass. The binary is not packaged and
the worker remains inert; see `evidence/0.0.0-dev/m2-pdf-worker-parser.md` and
ADR 0011.

The next checkpoint adds a separate synthetic macOS App Sandbox/XPC test:

```sh
just probe-document-sandbox-macos
```

Run from an ordinary macOS terminal with Command Line Tools, outside an enclosing
agent sandbox so its restrictions cannot contaminate the unsandboxed positive
control. It builds and ad-hoc signs only a test bundle, uses temporary synthetic
files and its own loopback listener, and never reads your profile or Keychain.
Generated bundles and reports remain in `target/native-probes`; the runner
removes its fresh fixture directory. `--build-only` can be passed to the Node
script to compile/sign without executing the probe.

Locally, read-only input transfer, seeded sibling/symlink restrictions and
loopback denial passed. Plain App Sandbox allowed direct children; the extended
probe now compares that baseline with helper-only hard limits (NPROC zero,
64 descriptors, core-file size zero). Direct spawn/fork and hard-limit increases
were denied, descriptor exhaustion/recovery passed, and the parent was unaffected.
Do not run this test as root: it is deliberately refused. It never changes the
desktop, shell or account-wide limits. The helper exits cooperatively; forced
cleanup and memory/CPU/thread/Mach-port/credential/broker limits are unproven.
A green probe invocation or CI job is **not** a full containment pass. Import is
still disabled. See `evidence/0.0.0-dev/m2-macos-hard-limits.md`. The report printed
in CI includes the synthetic measurements and hashes, not document content.

The subsequent lifecycle checkpoint runs a separate signed XPC supervisor and
its own fixed, sandbox-inheriting child:

```sh
just probe-document-lifecycle-macos
```

It tests nine synthetic cases: normal completion, cancellation, silent timeout,
stdout/stderr floods, nonzero exit, malformed output, complete output without
exit, and both pipe EOFs without exit. Stop cases require actual SIGKILL status
and `waitpid` reaping; no reused XPC PID or unrelated process is signaled.
The test uses small per-child limits, temporary marker files and no OS vault or
network. It does not launch the desktop or alter your shell/system limits.
Run outside an enclosing agent sandbox; `--build-only` is also supported by the
Node script. Generated signed test bundles/reports remain in `target/native-probes`;
the runner removes its own fresh input fixture directory.

This is a test candidate, not the production parser driver. Parent/supervisor
death, broker-created descendants, the child's complete authority boundary and
the remaining resource ceilings are still gated. Import is still disabled.
See `evidence/0.0.0-dev/m2-macos-lifecycle.md`. The user subsequently confirmed
all four CI jobs passing for `e978cfe`; this is user-reported, not an independently
retrieved run. The DOCX checkpoint subsequently passed three of four jobs;
its Windows golden-byte failure and subsequently confirmed repair are recorded above.

The preceding Windows log showed both isolated startup and import-storage tests crash
while opening the encrypted profile. A matching pinned SQLCipher logging defect
is now mitigated by `.cargo/config.toml`: its native diagnostic logger is compiled
out for local, CI and package builds. Run Cargo from the repository root (or a
subdirectory) so this policy is loaded; profile opening fails closed if the
required native configuration is missing. A clean/rebuilt native dependency is
expected the first time. App-level sanitized errors remain available, and memory
security/encryption remain on. The user reports all CI checks passed after this
mitigation (`bdc3e10`), including Windows; a run URL/log was not retrieved here.
Windows native UI/vault/installer verification remains a separate requirement.
Details: `evidence/0.0.0-dev/windows-sqlcipher-logging.md`.

On Windows, retain the last stage label if either command crashes:

```sh
cargo test --locked -p ort-storage --test native_startup -- --nocapture
cargo test --locked -p ort-application --test import_storage -- --nocapture
```

These probes use only temporary synthetic profiles and an in-memory vault.
No installer or Keychain/Credential Manager approval is required.

**Current limitations:** macOS Dock Quit and system shutdown are not protected
by this guard because the pinned runtime does not expose those termination
requests through its usual exit callback. Wait for **Saved** before using them.
Force Quit, crashes, renderer reloads, and power loss can also lose unsaved edits;
undo history is not crash recovery. Windows native interaction still needs VM
testing. Keep using synthetic data. On a failed save, autosave pauses; revision conflicts or
uncertain transport results require an explicit reload. Reloading asks before
discarding edited unsaved content, so copy anything needed before confirming.
If the quit connection fails, Keep editing leaves the editor available to copy
unsaved text while reconnecting. No missing response authorizes native quit.

The native vault proof is opt-in because it writes one randomized temporary
credential to macOS Keychain or Windows Credential Manager and then deletes it:

```sh
just test-platform-vault
```

An interruption can leave only a credential in the
`com.openresumetoolkit.platform-test.database` service namespace; no application
profile or real database key is used.

## Browser extension skeleton

Generate either development package with:

```sh
just dev-extension chrome
just dev-extension edge
```

The generated folders are `apps/extension/dist/chrome` and `apps/extension/dist/edge`. The M0 extension is intentionally inert and requests no host access or native-messaging permission.
