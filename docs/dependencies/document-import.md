# Document-import parser dependencies

The disabled M2 worker parsers pin:

- `quick-xml` 0.41.0, MIT, from the crates.io release associated with
  <https://github.com/tafia/quick-xml>;
- `flate2` 1.1.10, MIT OR Apache-2.0, from the crates.io release associated with
  <https://github.com/rust-lang/flate2-rs>;
- `pdfium-render` 0.9.3, MIT OR Apache-2.0, from the crates.io release associated
  with <https://github.com/ajrcarey/pdfium-render>, with default features off and
  only `pdfium_7881` enabled;
- non-V8/non-XFA `PDFium` 151.0.7881.0, BSD-style upstream license, distributed
  by the MIT-licensed <https://github.com/bblanchon/pdfium-binaries> immutable
  `chromium/7881` release.

`crates/ort-document-worker/pdfium-manifest.json` records the authenticated
GitHub release metadata plus exact archive and extracted-library sizes and
SHA-256 digests for macOS ARM64/x64 and Windows ARM64/x64. The selected archive
digests are:

| Target | Archive SHA-256 | Extracted library SHA-256 |
| --- | --- | --- |
| macOS ARM64 | `52e94ca5aa8847934330daf3f8150c190682c5ca93831468794f8b90d4392e40` | `1bc45b15466b34cef96641ce25c77a876e70010c6b114f909dda2f5325fc5bd7` |
| macOS x64 | `6dedf83990e0e3d6b7c93c9e7589c5a126b0ae14b7464d76120cff7a26afb18b` | `4eaad6c3e8d786cf6f66a45d7d014edf5c65f372f98c3070e66595ebb50e43d9` |
| Windows ARM64 | `d3035d4d2cacac6ecd1a2ece197a3d702a1b2a58466276b9f870b8cb278a9d84` | `267a6f08a9c854d9949754a53b7630f23c0b67de5c7ca273b6abf178b49158c2` |
| Windows x64 | `73cc0de638ac2095e7445bf56a38200a5b7c7ca0e9f4ba144598f2457377ac08` | `79d4676b656cfb1abcea88f9ade3b4b0826c5200382db5f4ec72a636c598c118` |

`Cargo.lock` records the registry checksums and exact resolved transitive graph.
These parser dependencies are linked only into the disposable document-worker
path; they do not establish containment and are not an authorization to enable
import. The application-layer import crates do not depend on
`ort-document-worker`.

This checkpoint reviewed API scope, fixed versions, declared licenses and the
release's no-V8/no-XFA build arguments. A native macOS ARM64 smoke test loaded an
exactly verified extracted library and parsed a synthetic in-memory PDF. Final
release work must still verify the release build-provenance attestation, run the
repository advisory/license/SBOM gates, retain all required license texts in
packaged notices, and reproduce the archive/extracted-library checks inside the
signed packaging pipeline. See ADR 0011.
