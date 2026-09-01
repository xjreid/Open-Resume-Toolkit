# Technical reference baseline

## Purpose

This is the dated source register behind the initial technical selections. It is not a replacement for pinned dependency versions, license files, or release-time verification. Upstream behavior, API versions, platform rules, and program eligibility can change.

Last reviewed: 2026-09-01.

## Desktop and packaging

- [Tauri 2 documentation](https://v2.tauri.app/) — desktop architecture and platform prerequisites.
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/) and [Content Security Policy](https://v2.tauri.app/security/csp/) — webview permission and bundled-content boundaries.
- [Tauri Windows installer documentation](https://v2.tauri.app/distribute/windows-installer/) — NSIS/MSI packaging and WebView2 choices.
- [Tauri DMG documentation](https://v2.tauri.app/distribute/dmg/) — macOS disk-image packaging.
- [Tauri updater documentation](https://v2.tauri.app/plugin/updater/) — mandatory updater signatures, platform artifacts, and static update JSON.

Release-time checks must confirm supported OS versions/architectures, installer/update behavior, action versions, and current platform signing rules.

## Storage and cryptography

- [SQLite atomic commit](https://www.sqlite.org/atomiccommit.html) and [file format/WAL documentation](https://www.sqlite.org/fileformat.html) — transaction, recovery, and journal assumptions.
- [SQLCipher documentation](https://www.zetetic.net/sqlcipher/documentation/) and [license information](https://www.zetetic.net/sqlcipher/license/) — encrypted SQLite integration and Community Edition attribution.
- [Apple Keychain Services](https://developer.apple.com/documentation/security/keychain-services) — macOS secret storage.
- [Apple Keychain access-control lists](https://developer.apple.com/documentation/security/access-control-lists) and [distribution-signed code](https://developer.apple.com/documentation/xcode/creating-distribution-signed-code-for-the-mac) — trusted-application/code-requirement behavior and entitlement authorization for shared Keychain access.
- [Microsoft generic credentials](https://learn.microsoft.com/en-us/windows/win32/secauthn/kinds-of-credentials) — Generic Credential values are readable to processes inside the same user boundary; the vault is not a same-user malware sandbox.
- [RFC 9106](https://www.rfc-editor.org/rfc/rfc9106.html) — Argon2id parameters; the backup v1 writer adopts the 64 MiB, three-iteration, four-lane memory-constrained recommendation.
- [SQLCipher API](https://www.zetetic.net/sqlcipher/sqlcipher-api/) and [design](https://www.zetetic.net/sqlcipher/design/) — per-page HMAC, compatibility/KDF/HMAC/page settings, integrity checks, and optional full allocation memory wiping.

The implementation security review must add the final Rust crate/build documentation, selected Windows/macOS vault API details, XChaCha20-Poly1305 specification/library evidence, generated backup test vectors, and exact SQLCipher compile options before format freeze.

## Documents

- [Typst PDF reference](https://typst.app/docs/reference/pdf/) — PDF options, standards support, and tagged-PDF behavior.

The PDFium/Open XML adapters, fonts, templates, PDF.js viewer, and their licenses/hashes are not accepted merely by appearing in a plan; add their official sources and review evidence during M2.

## Browser integration

- [Chrome native messaging](https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging) — manifest, allowed origins, platform registration, stdio framing, and browser size behavior.
- [Chrome extension messaging security guidance](https://developer.chrome.com/docs/extensions/develop/concepts/messaging) — trust treatment for content-script messages.

Microsoft Edge documentation must be rechecked before packaging even where Chromium behavior is shared, especially Add-ons identity and native-host registry locations.

## Codex

- [Codex app-server](https://developers.openai.com/codex/app-server) — `stdio` lifecycle plus command, process, filesystem, tool, approval, permission, and experimental surfaces that ORT must not expose or accept.
- [Codex authentication](https://developers.openai.com/codex/auth) — ChatGPT/device-code login, credentials, keyring/configuration behavior.

ORT supports only a tested subset and version range. Official protocol availability does not by itself prove the OS containment requirements; that remains a separate blocking gate.

## Release provenance and website

- [GitHub artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations) — build provenance publication and verification.
- [GitHub Actions artifacts](https://docs.github.com/en/actions/how-tos/writing-workflows/choosing-what-your-workflow-does/storing-and-sharing-data-from-a-workflow) — CI artifact flow; release assets remain a separate publication step.
- [Cloudflare Pages Astro guide](https://developers.cloudflare.com/pages/framework-guides/deploy-an-astro-site/) — static build output, Git integration, and preview deployments.
- [Astro documentation](https://docs.astro.build/) — static content architecture and content collections.

SignPath eligibility/process, Microsoft Store packaging/submission, Chrome Web Store, Edge Add-ons, Apple signing/notarization, provider endpoints/model catalogs/prices, and Cloudflare behavior are all release-time facts and require current official-source review.
