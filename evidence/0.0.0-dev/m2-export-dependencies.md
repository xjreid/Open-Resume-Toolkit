# M2 text-export dependency review

Date: 2026-09-02. This is an implementation review, not a vulnerability or
legal certification. Cargo.lock SHA-256:
`2a6ca5964081b551abcfb481372af7e1cb0d1aaccedc528232b4197c1bd2fb2f`.

## Additions and authority

- `tauri-plugin-dialog =2.7.2` (Apache-2.0 OR MIT) supplies the native Save dialog.
  Its upstream [Rust API](https://docs.rs/tauri-plugin-dialog/2.7.2/tauri_plugin_dialog/)
  requires blocking dialogs off the main thread; this implementation uses
  Tauri's blocking worker pool. Its selected `FilePath` is consumed only in Rust.
- `cap-std =4.0.3` (Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT)
  supplies held-directory operations. Its
  [directory API](https://docs.rs/cap-std/4.0.3/cap_std/fs/struct.Dir.html)
  enables relative staging and hard-link publication so later pathname changes
  cannot redirect output. No command gives the renderer ambient filesystem
  authority. Filesystems without hard links are unsupported in this checkpoint.
- Relevant new transitives include `cap-primitives 4.0.3`, `rfd 0.16.0` (MIT),
  `tauri-plugin-fs 2.5.2` and the plugin build helper. Registry versions and
  checksums are locked. No external renderer, parser, font, model, shell process,
  or executable download was added.

Inspected the downloaded dialog plugin's `init` and desktop implementation:
the dialog plugin registers its own command handler and managed native dialog
object; it does **not** initialize the filesystem plugin. `tauri-plugin-fs` is
a transitive Rust dependency, not an enabled frontend filesystem capability.
Both webview capability files still grant only `core:default`; regression tests
check this exact boundary and the existing restrictive CSP. No JavaScript
dialog/filesystem package was installed. The dialog plugin's injected bridge
does not itself grant its IPC permissions. The app's export command independently
checks the invoking window label and rejects unknown payload fields.

The exact dependency pins were selected from the inspected API/source versions,
not asserted to be the newest available releases. Updating either requires
rerunning source/permission review and cross-platform export tests.

## Evidence and remaining gates

- Local production JavaScript dependency audit: no known vulnerabilities found
  by `pnpm audit --audit-level high --prod` on this date. This does not audit Rust.
- Local `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  and workspace tests pass on macOS arm64.
- The desktop CI matrix now runs domain, text formatter, and filesystem writer
  tests on macOS arm64, macOS Intel, and Windows, beyond compilation alone.
  The modified workflow has not run remotely yet.
- `cargo-audit` is not installed here. A current RustSec audit, full transitive
  license/SBOM/notice generation, signed-package checks, and Windows native
  dialog/ACL/reparse-point verification remain release gates. The listed
  SPDX strings come from the downloaded registry manifests; they are not a
  complete distribution notice bundle or a legal compatibility opinion.

No dependency change makes this development checkpoint release-eligible.
