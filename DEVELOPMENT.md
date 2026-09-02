# Development

Open Resume Toolkit implements the M0 architecture skeleton, the local M1
encrypted-storage slice, and the first M2 offline editor slice. The development
app can save synthetic resume drafts and publish immutable snapshots through its
OS-vault-backed encrypted database. Import, rendering/export, AI, updater, and
browser-native messaging remain gated or unimplemented.

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

For the initial M2 manual check, enter synthetic contact information, add a
section, entry, and bullet, then choose **Save draft** followed by **Publish
snapshot**. Restart the app and confirm the saved draft and published revision
return. Publishing is disabled while edits remain unsaved.

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
