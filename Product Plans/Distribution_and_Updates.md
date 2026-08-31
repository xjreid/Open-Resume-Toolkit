# Distribution and updates

## Distribution principles

- Every official binary maps to a tagged public source revision and documented build workflow.
- Official channels, publisher identity, checksums, signatures, and installation instructions are published consistently with each release.
- Only project-controlled release channels, signing identities, package identifiers, and update metadata may represent a build as official. Modified and third-party builds use distinct identity and update infrastructure in accordance with `TRADEMARKS.md`.
- Store and direct-download editions identify their update channel and never overwrite one another using a different channel's updater.
- Unsigned preview builds are labeled prominently and never presented as equivalent to trusted signed releases.
- A release that enables Codex subscription mode identifies whether it bundles a verified app-server component or requires a separately installed compatible runtime. Bundled binaries are pinned to reviewed public source, included in the SBOM/notices, signed with the containing package, and updated only through the ORT release channel. External runtimes are compatibility-checked and never modified silently by ORT.

## Windows

### Microsoft Store channel

- The planned mainstream Windows package is MSIX distributed through Microsoft Store.
- Microsoft Store signing and delivery are the preferred zero-certificate-cost path.
- Store installations normally receive background updates through Microsoft Store.
- The desktop **Check for updates** action uses the supported Store APIs to discover and request applicable Store package updates.
- Native-messaging registration and repair must work with MSIX isolation and survive package updates. This is a required implementation proof, not an assumption.

### GitHub/direct channel

- GitHub Releases may offer a direct Windows build with checksums and provenance.
- Trusted direct distribution should use SignPath Foundation if the project qualifies and is accepted, or another approved signing path.
- Until trusted signing is available, an unsigned build may be offered only as a developer preview with SmartScreen limitations explained.
- A direct-build updater may query the latest compatible GitHub Release, compare versions, show release notes, download the correct package, verify its trusted signature and expected integrity, and hand off to a minimal updater.

## macOS

- A polished public macOS build should use Apple Developer ID signing, hardened runtime, and Apple notarization for direct distribution.
- Developer ID signing and notarization require the appropriate Apple Developer Program membership and release credentials.
- A clearly labeled unsigned macOS preview may be distributed before paid signing, but users must be told about Gatekeeper and manual opening requirements.
- Automatic updating must not be enabled for unsigned builds unless an independently secure signature system is implemented and its user experience is reviewed.

## Update behavior

- The settings area contains **Check for updates** and displays the current version, channel, last check, and release notes.
- Automatic background application and signed pricing/model-catalog checking may be enabled by default with a clear preference to disable it. Checks disclose only ordinary request metadata required to reach the update service; no resume/application content, AI credential, or stable cross-site tracking identifier is sent.
- Updates are downloaded only from channel-approved HTTPS locations and verified before execution.
- Content-only catalog updates are downloaded only from the canonical signed catalog channel, verified before activation, schema/sequence checked against rollback or freeze attacks, and never treated as executable code or permission to enable an untested model.
- Downgrades require explicit developer/testing workflows and must not silently run against a newer incompatible data schema.
- An application update that changes local data first creates or verifies a migration safety backup.
- Desktop, native host, extension, document schema, renderer, backup, provider catalog, and Codex app-server protocol/runtime compatibility versions are evaluated together before release.

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
- Supported direct-provider presets/pricing-catalog effective date and, when enabled, Codex app-server distribution/runtime requirement, tested model intersection, and known token/quota limitations
- Security fixes without prematurely disclosing exploitable detail
