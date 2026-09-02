# M2 close-guard checkpoint

Date: 2026-09-02. Base commit: `b1605b8`; implementation changes are not yet
committed. Local platform: macOS arm64. Development identity:
`com.openresumetoolkit.dev`. Artifact: the ad-hoc-signed preview app in the
manifest, not a refreshed DMG or stable release.

## Native checks

Only the existing synthetic development resume was used. No real resume or
Keychain credential value was inspected. The separate installed app in
`/Applications` was not modified.

- Main window close with an invalid blank title opened a modal instead of
  exiting. Save and quit was disabled; Keep editing and explicit discard were
  available. Escape preserved the unsaved title. Undo returned to Saved.
- A clean main-window close exited through native approval.
- Initial Command-Q testing exposed an upstream macOS termination bypass. After
  replacing the predefined Quit action with a regular app-owned menu action,
  Command-Q and clicking Quit in the app menu both opened the guard.
- Repeating Command-Q while the modal was open did not create duplicate dialogs
  or authorize exit. Focus stayed on Keep editing.
- Command-Q with the overlay focused brought the main editor's confirmation
  forward. Escape preserved its invalid edit; Undo restored the saved draft.
- A valid title change followed immediately by Command-Q (before autosave)
  enabled Save and quit. On reopening, draft revision 10 contained
  `Synthetic M2 save-and-quit check`; published snapshot 2 stayed unchanged.
- Explicitly discarding a subsequent invalid title exited without saving it.
  Reopening still showed revision 10, the saved title, skill/link/section data,
  and snapshot 2. Only the intentionally unsaved synthetic title was discarded.
- A native screenshot showed the modal, readable warning, visible keyboard
  focus, and disabled save control without clipping at the default window size.

## Automated and build evidence

- Frontend cases cover clean/untouched/dirty/invalid close policy, pending saves
  and publication, save failure/conflict/uncertain transport, and a late older
  save that must not authorize quitting with newer edits.
- Subscription tests cover missed events, out-of-order reads, cancellation
  invalidation, StrictMode cleanup, listener failure, and untrusted event wakeups
  that cannot invent native authority.
- Rust cases deny unsolicited, stale, replayed, and overlay resolutions; repeated
  native close requests coalesce and cancellation invalidates their identifier.
- Generated contracts validate bounded attempt IDs and typed responses. No
  database migration, new dependency, credential access path, broad process
  capability, or frontend filesystem/network permission was added.
- Local `just check` and a macOS preview build are run for this checkpoint;
  current outcomes and the verified executable hash are in the manifest.

## Remaining gates

- macOS Dock Quit and logout/shutdown are NOT protected. The pinned Tauri 2.11.5
  / muda 0.19.3 native Quit action invokes `terminate:` outside the usual exit
  callback. The custom app menu fixes its own path, not the OS delegate.
  [Tauri issue #9198](https://github.com/tauri-apps/tauri/issues/9198) and
  [termination-hook request #12978](https://github.com/tauri-apps/tauri/issues/12978)
  document this boundary. Wait for Saved before using those system paths.
- Force Quit, crashes, renderer reloads, power loss, and unsaved invalid-form
  recovery are not covered. An unresponsive renderer cannot approve a guarded
  quit; no timeout silently discards its work. OS Force Quit remains available.
- Windows interaction, Alt-F4/logoff, and a full
  screen-reader/scaling matrix require further native checks. Save-failure and
  in-flight timing scenarios currently have automated, not native fault-injection,
  evidence.
- The user reported a green four-job GitHub matrix for the previous push
  (run `33591464799`). This unpushed checkpoint has not run remotely.
- M2 import containment, import review, PDF preview, exports, storage management,
  and the complete offline journey remain unfinished. This is not release-eligible.
