# Distribution and update implementation

- [`Distribution_and_Updates_Plan.md`](Distribution_and_Updates_Plan.md) defines channel identities, packaging, SignPath-first Windows signing, unsigned macOS preview handling, GitHub Releases, updater trust, release automation, Store fallback, and recovery.
- Browser Store sequencing and native-host registration are also constrained by the browser/IPC plan.

Local development app/helper packaging is implemented, including an explicit
verified Codex-runtime installer. Production Developer ID signing/notarization,
updater metadata/publication, final native-host identity, and extension Store
qualification remain M7 work. The repository currently has CI and dependency-scan
workflows; it does not have a completed release/publish workflow. See
[development setup](../../DEVELOPMENT.md) and the
[current implementation map](../../docs/development/current-state.md).
