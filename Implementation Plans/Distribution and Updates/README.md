# Distribution and updates implementation plans — reserved

Future plans in this folder should cover:

- Preferred GitHub Windows packages, checksums, SBOM/provenance, public code-signing policy, SignPath eligibility/application, signed direct updater, and unsigned-preview limitations
- Fallback Microsoft Store MSIX packaging, identity, signing, certification, native-messaging feasibility, and Store update APIs when SignPath is unavailable
- Unsigned macOS preview packaging, checksums/provenance, authenticated update notification, Gatekeeper guidance, and measurable triggers for later hardened runtime, Developer ID, notarization, and stapling
- External Codex-runtime installation guidance, executable verification, protocol compatibility window, release compatibility matrix, and safe disablement; the initial ORT package does not bundle Codex
- Channel identity and rules preventing Store/direct updater crossover
- Trusted update metadata, signature verification, download handoff, restart, failure recovery, and schema-safe ordering
- Independently signed pricing/model-catalog publication, official-source review, effective/expiry metadata, rollback/freeze resistance, and content-only activation
- GitHub release automation, protected environments, signing-secret custody, SBOM, provenance, and reproducible-build goals
- Extension Store submissions and compatibility sequencing
- Install, update, repair, downgrade, and uninstall test matrix
- Compatibility coordination across desktop, native host, extensions, schema, renderer, backup format, provider/model catalog, and Codex app-server
