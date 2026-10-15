# Browser extension implementation

- [`Browser_Extension_and_IPC_Plan.md`](Browser_Extension_and_IPC_Plan.md) defines the shared Manifest V3 extension, native host, authenticated desktop IPC, packaging identities, repair, and version-skew behavior.
- The extension is optional and never performs AI work or persistently stores captured job content.
- The Chrome store-source package and an explicit unsigned macOS development
  BETA bridge are implemented. Real Chrome → native host → encrypted desktop
  review is tested using disposable profiles. See
  [Chrome development setup](../../packaging/extension/chrome/development-testing.md) for setup and the development
  trust boundary. The signed production channel remains gated; Edge is deferred.

Chrome is version 0.3.0. Default native builds retain fail-closed production
behavior; macOS development-feature tests run separately in CI. See the
[extension README](../../apps/extension/README.md) and
[CI map](../../docs/development/ci.md). Functional Edge capture remains unfinished.
