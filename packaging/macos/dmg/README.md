# macOS packaging

`just package-preview` builds an explicitly ad-hoc-signed development-identity
`.app` and DMG through `tauri.preview.conf.json`. It is for local
install-boundary testing only: ad-hoc signing verifies bundle integrity on the
current machine, but it does not establish a trusted publisher identity.

The preview is not Developer ID signed or notarized, does not carry the release
identity, and must not be distributed to users. macOS may still require the
tester to approve opening it in Privacy & Security. Developer ID signing,
notarization, hardened runtime, final DMG layout, and release verification
remain M7 gates.
