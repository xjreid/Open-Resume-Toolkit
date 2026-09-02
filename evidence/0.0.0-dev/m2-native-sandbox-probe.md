# M2 native macOS sandbox probe: partial containment evidence

Date: 2026-09-02. Base commit: `e643574`. macOS 26.6.2, arm64.
Status: synthetic native subset measured; full containment gate not satisfied.

## Implementation and scope

`just probe-document-sandbox-macos` builds a separate test app and embedded XPC
service with public C APIs, ad-hoc signatures and hardened runtime. The helper
has only `com.apple.security.app-sandbox = true`; signature verification is
strict and the helper inspects its effective entitlement. This is research/test
code under `tools/native/macos-document-probe`, not the product parser, desktop
bundle or an exception to the workspace Rust unsafe-code policy.

The parent transfers one read-only descriptor for a synthetic marker file.
Only newly created, private `/private/tmp/ort-document-sandbox-*` fixtures and
a parent-owned ephemeral IPv4 loopback TCP listener are tested. The same probes
in the unsandboxed parent establish accessible positive controls. The actual
run was outside the agent sandbox to avoid contaminating those controls.

## Measured result

Two local native executions agreed. The retained final measurement is
[macos-sandbox-probe-report.json](macos-sandbox-probe-report.json).

| Probe | Unsandboxed control | App Sandbox helper |
| --- | --- | --- |
| Read exact marker through preopened descriptor | Allowed | Allowed |
| Write through read-only descriptor | Denied (`EBADF`) | Denied (`EBADF`) |
| Open seeded sibling for read/write | Allowed | Denied |
| Follow symlink to seeded sibling | Allowed | Denied |
| Connect to parent's loopback TCP listener | Allowed | Denied |
| Create `/usr/bin/true` child and reap it | Allowed | **Allowed: no-child gate failed** |

Only `EACCES`/`EPERM` count as access denials; missing targets, dead listeners
and other OS errors are inconclusive and reject evidence. Write authority is
tested by opening for write without truncating or modifying the sibling file.
The fixed child uses no shell or inherited secrets. Creation itself fails the
planned prohibition, even if its later execution is restricted. This does not
demonstrate a sandbox escape.

The helper exits cooperatively after its response; the parent observes an XPC
connection interruption. A helper alarm and parent deadline are test failsafes,
not hostile-code resource limits or forced process-tree cleanup. No potentially
recycled XPC PID is used to kill a process.

## Verification and remaining gate

- C compilation uses `-Wall -Wextra -Werror`; nested code signatures pass
  `codesign --verify --deep --strict`.
- Five Node tests reject malformed/inconclusive measurements, failed positive
  controls and missing lifecycle/entitlement evidence. Even an all-denied fixture
  cannot set `fullContainmentProven` or `importEnabled` to true.
- Both macOS CI jobs now run the synthetic subset. A green step means its
  descriptor/filesystem/loopback/cooperative-disconnect assertions passed, not
  that the no-child requirement or full sandbox gate passed.
- Forced tree termination and parent death, memory/CPU/handle ceilings,
  credential/broker/native-IPC isolation, UDP/DNS/external-network denial, broader
  filesystem access, hostile-code/crash cleanup, release-signed packages and
  supported OS/architecture matrices remain unproven. Windows is still pending.

No real document, resume profile, Keychain item or external network endpoint was
accessed. The runner removed only its fresh synthetic fixture directory; those
disposable fixtures are not retained. Test bundles and their reports remain under
ignored `target/native-probes` for inspection. The OS may maintain its own test
service container; the runner does not remove user container directories.

The final local report originated at
`target/native-probes/macos-document-9cvMgD/report.json`. It records the host and
helper hashes, signing, OS/architecture and numeric measurement outcomes
(`0` allowed, `1` denied). No user content is present.

The production `ort-document-worker` still exits 78 and `IMPORT_ENABLED` remains
false. No app UI, installer or previous desktop preview package was rebuilt.
Next work must resolve or explicitly redesign the unsatisfied containment
requirements before integrating a PDF/DOCX parser. M2 remains incomplete.
