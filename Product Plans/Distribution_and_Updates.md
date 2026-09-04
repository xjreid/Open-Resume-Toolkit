# Distribution and updates

## Current platform scope

The active development and preview-qualification channel is macOS Apple Silicon.
The Windows channel design and Intel-Mac packaging requirements below are
retained for later platform expansion and do not gate M0-M2 macOS-arm64
completion. A shared CI build is not a supported distribution.

## Distribution principles

- Every official binary maps to a tagged public source revision and documented build workflow.
- Official channels, publisher identity, checksums, signatures, and installation instructions are published consistently with each release.
- Only project-controlled release channels, signing identities, package identifiers, and update metadata may represent a build as official. Modified and third-party builds use distinct identity and update infrastructure in accordance with `TRADEMARKS.md`.
- Direct-download and fallback Store editions identify their update channel and never overwrite one another using a different channel's updater.
- Unsigned preview builds are labeled prominently and never presented as equivalent to trusted signed releases.
- The initial ORT package does not bundle Codex. A release that enables Codex subscription mode identifies the separately installed compatible runtime requirement, supported version/protocol window, installation guidance, and failure behavior. ORT compatibility-checks the runtime and never installs, upgrades, or modifies it silently.

## Windows

### Preferred GitHub/direct channel

- The preferred stable Windows package is a direct build published through canonical GitHub Releases and signed through SignPath Foundation if the project qualifies and is accepted.
- The project first establishes documented functionality, an initial release in the form to be signed, reproducible and verifiable source-to-binary CI, MFA, review and signing-approval roles, privacy and uninstall documentation, a public code-signing policy, and the other current SignPath eligibility conditions. Acceptance is discretionary and is never represented as guaranteed.
- Every direct release includes checksums, source revision, build provenance, signature identity, SBOM/third-party notices, and release notes.
- Until trusted signing is available, an unsigned Windows build may be offered only as a development preview with SmartScreen limitations and verification instructions explained.
- A signed direct-build updater may query the latest compatible canonical GitHub Release, compare versions, show release notes, download the correct package, verify the trusted package signature, independently authenticated update metadata, and expected integrity, and hand off to a minimal updater.

### Microsoft Store fallback channel

- If SignPath signing is declined, unavailable, or cannot support the required release form, the planned fallback stable Windows package is MSIX distributed through Microsoft Store.
- Current Microsoft developer-account cost, identity verification, package policy, certification, and native-messaging feasibility are rechecked before enrollment; no plan assumes that current zero-registration-fee terms are permanent.
- Store installations normally receive background updates through Microsoft Store.
- The desktop **Check for updates** action uses the supported Store APIs to discover and request applicable Store package updates.
- Native-messaging registration and repair must work with MSIX isolation and survive package updates. This is a required implementation proof, not an assumption.

## macOS

- Initial macOS packages may be published through canonical GitHub Releases only as clearly labeled unsigned previews. Each preview includes checksums, source/build provenance, authenticated release metadata, accurate Gatekeeper and manual-opening instructions, and documented native-messaging or update limitations.
- An unsigned macOS artifact is not a stable broadly trusted release, even when it is project-controlled and its checksum is valid.
- Stable direct macOS distribution requires Apple Developer ID signing, hardened runtime, and Apple notarization. The project adopts the required paid Apple Developer Program membership when sustained macOS use, recurring Gatekeeper support burden, organizational adoption, or available project funding justifies the ongoing cost.
- Automatic application updating must not be enabled for unsigned previews unless an independently secure signature system is implemented, threat-reviewed, and clearly explained. Manual update notification may still point to the canonical release page.

## Update behavior

- The settings area contains **Check for updates** and displays the current version, channel, last check, and release notes.
- Automatic background application updating may be enabled by default only for a trusted signed/notarized channel. Unsigned previews may perform a bounded version check and direct the user to the canonical release page, but do not install an update automatically unless the independently authenticated design required above has been implemented and reviewed. Signed pricing/model-catalog checking may be enabled by default with a clear preference to disable it. Checks disclose only ordinary request metadata required to reach the update service; no resume/application content, AI credential, or stable cross-site tracking identifier is sent.
- Updates are downloaded only from channel-approved HTTPS locations and verified before execution.
- Content-only catalog updates are downloaded only from the canonical signed catalog channel, verified before activation, schema/sequence checked against rollback or freeze attacks, and never treated as executable code or permission to enable an untested model.
- Downgrades require explicit developer/testing workflows and must not silently run against a newer incompatible data schema.
- An application update that changes local data first creates or verifies a migration safety backup.
- Desktop, native host, extension, document schema, renderer, backup, provider catalog, and external Codex app-server protocol/runtime compatibility versions are evaluated together before release.

## Browser-extension stores

- Chrome is distributed through Chrome Web Store.
- Edge is distributed through Microsoft Edge Add-ons.
- Store listings explain permissions, local-first behavior, provider transmission, desktop-app dependency, and the exact official project links.
- Extension updates are handled by their stores. The desktop and extension negotiate protocol compatibility during staggered rollouts.
- Sideload instructions are available for developers and advanced users but are not the default mainstream installation path.

## Release documentation

Every public release provides:

- Version and release date
- User-visible changes and known issues
- Minimum supported OS/browser versions
- Download assets and checksums
- Signature/notarization status per asset
- Source tag/commit and build-provenance link
- GPLv3, copyright, Section 7 attribution, third-party notice, and trademark-policy links
- A visible official/preview/third-party status that agrees with the publisher and signing identity
- Data-schema or backup compatibility notes
- Supported direct-provider presets/pricing-catalog effective date and, when enabled, external Codex runtime installation and compatibility requirements, tested model intersection, and known token/quota limitations
- Security fixes without prematurely disclosing exploitable detail
