# Distribution and updates plan

## Status and requirements

- Status: approved baseline; SignPath application and Store/native-host probes outstanding
- Owner: release maintainer
- Milestone: M7, with preview packaging earlier
- Product authority: `../../Product Plans/Distribution_and_Updates.md`, `Security_Privacy_and_Open_Source.md`, `Quality_Accessibility_and_Verification.md`, and `Release_Scope_and_Open_Decisions.md`

Codex is not included in any ORT installer. Users who select Codex mode install a supported Codex runtime independently.

Cross-platform source ownership, build targets, environment identities, CI stages, desktop-before-extension deployment order, and the M0 handoff are centralized in `../System Documentation/Development_and_Deployment_Outline.md`. This plan remains authoritative for packaging, signing, update trust, publication, and rollback mechanics.

Only macOS Apple Silicon is an active development and preview-qualification
target through M2. Windows and Intel/universal Mac sections are retained as
future channel designs. They do not create present native-test obligations or
support claims, and their artifacts must not be published as qualified previews.

## Channel identities

| Channel | Application identity | Package/update owner |
|---|---|---|
| development | separate dev ID and data directory | local developer |
| preview-direct | preview ID/data and GitHub prerelease | GitHub; manual/preview updater rules |
| stable-direct | stable direct ID/data | GitHub Release + ORT signed updater |
| stable-store | Store package identity/data | Microsoft Store update service |

Store and direct builds never use each other's updater. Preview and stable data are separate by default so a preview migration cannot endanger stable user data. A deliberate import/backup path is used to move data.

## Artifact set

Example version `1.2.3` assets:

```text
OpenResumeToolkit_1.2.3_windows_x64-setup.exe
OpenResumeToolkit_1.2.3_windows_x64.nsis.zip       updater payload if required by Tauri
OpenResumeToolkit_1.2.3_windows_x64.nsis.zip.sig
OpenResumeToolkit_1.2.3_macos_universal.dmg
OpenResumeToolkit_1.2.3_macos_universal.app.tar.gz updater payload when eligible
OpenResumeToolkit_1.2.3_macos_universal.app.tar.gz.sig
latest.json                                        channel-specific updater metadata
checksums-sha256.txt
compatibility.json
sbom-spdx.json
sbom-cyclonedx.json
licenses.html
provenance.intoto.jsonl or GitHub attestation reference
```

Final names must be stable and machine-readable. Every release states OS/architecture, channel, signature status, Codex compatibility, extension versions, database/backup/renderer versions, known limitations, and verification instructions.

## Windows preferred direct channel

Tauri builds a per-user NSIS installer on a clean Windows runner. The installer includes the desktop executable, native host, bundled renderer/parser assets, licenses, and registration actions. It does not include Codex or provider credentials.

SignPath is the preferred free code-signing path for the open-source project. The application process should begin once the project satisfies SignPath's eligibility and public-release requirements. Signing configuration must deep-sign relevant PE files before the outer installer and preserve the exact artifact digest through promotion.

Stable-direct publication requires a valid trusted Windows signature. Before SignPath approval, GitHub artifacts may be published only as clearly labeled unsigned previews with SmartScreen guidance and no claim of stable signing. They must still carry updater signatures, checksums, SBOM, and provenance.

The native host uses per-user install/registration to avoid administrative requirements. Clean-machine tests cover WebView2 bootstrap mode, paths containing Unicode/spaces, repair, update while running, uninstall, and browser registration.

## Microsoft Store fallback

If SignPath approval is denied or unavailable long enough to block a trustworthy stable direct channel, implement the approved Store fallback.

First run a packaging spike for MSIX/Store identity using Microsoft-supported tooling. Prove:

- Tauri application and all runtime assets package correctly;
- local SQLCipher/vault/data paths behave under Store identity;
- native-messaging registry/manifest registration is permitted and persists through updates;
- file associations/launch activation work;
- Store update ownership can fully disable the direct updater;
- backup migration between direct and Store identities is documented and safe.

If the Microsoft Store supports an approved packaged Win32 submission that better preserves native messaging, document that as an architecture change before replacing the product plan's MSIX assumption. Do not improvise a hybrid channel.

The Store performs distribution signing. No stable-direct update metadata is embedded in Store builds.

## macOS initial channel

Build an Apple Silicon DMG for the active preview. When Intel-Mac work resumes,
prefer a universal DMG where dependencies support it; otherwise publish separate
Apple Silicon and Intel DMGs with explicit names. The initial preview is unsigned
and not notarized, distributed through GitHub Releases with SHA-256 checksums,
SBOM, provenance, and candid Gatekeeper/quarantine instructions.

Unsigned macOS preview builds do not silently auto-install updates. `Check for updates` verifies authenticated metadata and opens the exact GitHub release/download guidance. Tauri updater signatures may protect metadata/artifact integrity but are not represented as Apple code signing or notarization.

When the traction/cost trigger in the product plan is met, add Developer ID Application signing, hardened runtime, notarization, stapling, and signed in-app updates. That transition receives its own key-custody and entitlement review.

## Updater trust model

Tauri updater signing uses an offline/protected private key distinct from Windows/macOS code signing and the provider catalog key. Public key and channel endpoint are compiled into the application.

`latest.json` is generated from tested artifacts and contains version, notes URL, publication time, platform-specific URL, digest/signature, minimum compatible schema/runtime data, and channel identity. It is hosted as a GitHub Release asset or a static indirection file whose content is signed.

Update flow:

1. manual or scheduled check fetches bounded metadata over HTTPS;
2. verify channel, signature, chronology, version, OS/architecture, compatibility, and security floor;
3. show version, size, signing status, notes, and whether restart/migration is required;
4. download to a private staging path and verify signature/digest before install;
5. ensure no active operation and create a pre-migration safety copy when required;
6. hand off to platform installer/updater and restart;
7. verify new binary/data health and retain rollback evidence.

Bad/expired/unreachable metadata leaves the current app usable. The updater rejects cross-channel or older-than-security-floor releases. Automatic checks use only version/channel/platform metadata and record no user identity.

## Catalog publication

Provider/model/pricing catalogs are content-only signed assets, independently versioned from application releases. A protected workflow builds canonical JSON, validates sources/effective dates, signs it with the catalog key, and publishes it alongside a human-readable review record. Catalog activation never downloads executable code or prompt templates.

Emergency catalog disablement can prevent selection of a broken/retired model, but cannot change user documents or silently switch connection mode.

## GitHub Actions release design

Workflows are separated:

### `ci.yml`

Unprivileged pull-request build/test on the qualified macOS-arm64 target plus
retained deferred-platform portability targets. No secrets and no publishing
permissions; deferred-target success is not native qualification.

### `preview.yml`

Manual or prerelease-tag build. Produces unsigned packages, updater signatures through a protected preview key if approved, checksums, SBOM, compatibility manifest, and provenance. Uploads workflow artifacts, runs clean-install smoke tests, then creates/updates a draft prerelease.

### `release.yml`

Triggered from an approved signed semantic-version tag. Uses pinned action commit SHAs, minimal `contents`/attestation permissions, and protected environments. Flow:

1. validate tag, changelog, version consistency, clean lockfiles, catalog and contract versions;
2. build Windows and macOS artifacts once;
3. generate SBOM/licenses/checksums/provenance and scan;
4. send Windows artifacts through SignPath and retrieve signed results;
5. verify code/updater signatures and artifact digests;
6. run clean-machine install/update/repair/uninstall tests on the exact artifacts;
7. assemble signed update metadata;
8. create a draft GitHub Release with all assets;
9. human approval promotes draft to public stable release;
10. website build consumes the released compatibility/download manifest.

Jobs must not rebuild between testing and publication. GitHub artifact attestations complement, not replace, OS/updater signatures.

## Secrets and roles

- SignPath policy/project credentials, Tauri updater private key, catalog private key, Store credentials, and future Apple credentials are separately scoped.
- Pull-request workflows and forks cannot access them.
- Protected environment approval is required for production signing/publication.
- Hardware-backed or managed signing is preferred; exportable secrets are rotated and recovery-tested.
- Maintainer who changes release workflow should not be the sole approver of the same release.

## Release compatibility matrix

`compatibility.json` records:

- desktop version/channel/platform;
- minimum/maximum database and backup versions;
- renderer/template/font bundle versions;
- native-host and extension protocol range plus Store listing versions;
- provider catalog/prompt/schema range;
- external Codex runtime/protocol tested range and enabled/disabled gate status;
- update security floor.

Desktop exposes this data in diagnostics. Website/support pages render it rather than maintaining a divergent hand-written matrix.

## Rollback and incident behavior

- Withdraw a bad release asset/update manifest without deleting evidence; publish an incident note.
- If no schema migration occurred, republish the last known-good version only through new signed metadata that satisfies rollback policy.
- If data migrated incompatibly, ship a forward fix or guide restoration of the pre-migration copy; never encourage installation over an unreadable schema.
- A compromised updater/catalog key is revoked independently and replaced through a release signed by the remaining trust channels.
- A compromised code-signing identity triggers SignPath/Microsoft/Apple revocation procedures and a documented clean-install path.
- Store rollback follows Store controls and never re-enables direct updater.

## Test matrix

- clean install, upgrade from every supported predecessor, interrupted download/install, app running, low disk;
- tampered package, updater metadata, signature, checksum, provenance, and catalog;
- preview/stable/direct/Store crossover attempts;
- Windows per-user permissions, WebView2 variants, Unicode paths, native host repair/uninstall;
- macOS Intel/Apple Silicon, quarantine/Gatekeeper, drag-install, moved app, unsigned update guidance;
- schema migration failure and recovery copy;
- offline update check and GitHub rate/availability failures;
- external Codex absent/old/new/incompatible without affecting base install.

## Completion criteria

- A release is reproducibly associated with one commit and the exact tested artifacts.
- Stable Windows direct packages are SignPath-signed, or the approved Store fallback owns stable Windows distribution.
- macOS artifacts are accurately labeled unsigned until Developer ID/notarization exists.
- Update signatures, OS signing status, checksums, SBOM, provenance, and compatibility are independently verifiable.
- No installer bundles Codex, keys, user content, or hidden telemetry.
- Repair/update/uninstall preserve or remove local data exactly as documented.
