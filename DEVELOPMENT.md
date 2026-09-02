# Development

Open Resume Toolkit implements the M0 architecture skeleton, the local M1
encrypted-storage slice, and an M2 offline editor checkpoint. The development
app can autosave synthetic resume drafts and publish immutable snapshots through
its OS-vault-backed encrypted database, and export a saved draft or published
snapshot as unencrypted UTF-8 text through a native Save dialog. PDF/DOCX import
and output, AI, updater, and browser-native messaging remain gated or unimplemented.

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
