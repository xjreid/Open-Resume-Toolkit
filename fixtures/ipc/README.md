# Synthetic IPC fixtures

Protocol, authentication, capture-session, bounds, replay, origin, disconnect,
and timeout cases are exercised by `ort-ipc`, `ort-native-host`, desktop bridge
tests, and `apps/extension/tests/`. macOS feature-enabled development transport
tests run separately from production fail-closed defaults.

Use only synthetic identifiers, current-user temporary capabilities, and
invented captures. Never commit a real installation secret, credential, or page
containing personal data. See the [CI map](../../docs/development/ci.md).
