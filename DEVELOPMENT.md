# Development

Open Resume Toolkit implements the M0 architecture skeleton, the local M1
encrypted-storage slice, and an M2 offline editor checkpoint. The development
app can autosave synthetic resume drafts and publish immutable snapshots through
its OS-vault-backed encrypted database, and export a saved draft or published
snapshot as unencrypted UTF-8 text, constrained DOCX, or the exact locally
previewed PDF through a native Save dialog. PDF/DOCX import, AI, updater, and browser-native
messaging remain gated or unimplemented.

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
keeps editing. A running save/publication/text export is allowed to finish before quit;
failed saves keep the editor open. Closing the main window currently quits the
whole development workspace; closing the overlay alone does not quit the editor.
See `evidence/0.0.0-dev/m2-close-guard-smoke.md` for verified paths and limitations.

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
on Windows). All four CI jobs now independently check the generated package,
CRC, XML, source-text parity, heading/list semantics and hyperlink relationships.
Local headless-render and manual/platform limits are recorded in
`evidence/0.0.0-dev/m2-docx-export.md`. There is no production Python or
LibreOffice dependency; the exporter is Rust-only.

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
line breaks, in addition to document limits. Overflow, uncovered glyphs (including
the current CJK/emoji fixture), compile warnings, and unsupported layouts are
explicit failures, never silent truncation or font substitution. Tabs become four
spaces. Text and DOCX remain alternatives. This original plain fixture is not the
final template catalogue. The fixed typography/language options and full bundled
licenses are visible in the panel. Each successful preview stores a content-free
receipt in the encrypted profile before the PDF is exposed to the UI. Repeated
identical source/revision/PDF identities increment a counter instead of adding
duplicate rows. Storage keeps the newest 100 identities and the UI shows the
newest 20; resume text, PDF bytes, paths and preview tickets are not retained.
This metadata does not provide an old-renderer replay guarantee and is not yet
included in the portable-backup prototype.

Headless structural verification, without launching the app or OS vault:

```sh
cargo run --locked -p ort-render --example pdf_fixtures -- target/pdf-review-fixtures
node tools/verify-pdf-fixtures.mjs target/pdf-review-fixtures
# Optional second independent parser (requires pypdf):
python3 tools/verify-pdf-fonts.py target/pdf-review-fixtures
```

Every CI runner now checks PDF golden bytes, visible text/order, page geometry,
tags and safe links. New template sources also retain LF bytes under Windows
checkout conversion. `tools/smoke-pdf-browser.mjs` performs optional headless
Chromium QA against a loopback production frontend with synthetic mocked native
commands; supply `ORT_PLAYWRIGHT_MODULE` / `ORT_BROWSER_EXECUTABLE` if needed.
It never proves native IPC, WKWebView/WebView2, save dialogs, vaults or ACLs.
See `evidence/0.0.0-dev/m2-pdf-preview.md` for actual checks and manual gates.

## No-AI import core (backend-only checkpoint)

The deterministic mapper and in-memory review engine now have synthetic tests,
including an encrypted database commit/restart test. They do not enable a new
button or file import in the app yet. No PDF/DOCX parser is running: the worker
remains disabled until native containment is proven. No standalone `.txt`
importer or AI transmission was added.

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
gating. It does not yet implement native pipes, process termination or a sandbox.
See `evidence/0.0.0-dev/m2-import-transport.md` and
`Implementation Plans/System Documentation/Document_Worker_Containment.md`.

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
