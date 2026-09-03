# M2 native macOS direct-child supervision checkpoint

Date: 2026-09-02 local time (final report timestamp uses UTC).
Base commit: `723a97f`. Local platform: macOS 26.6.2 arm64.
Status: native lifecycle subset passed locally; full containment remains gated.

The user confirmed all four CI jobs passed for the preceding hard-limit
checkpoint. That report was not independently retrieved from GitHub. The new
lifecycle probe has not yet run on the two macOS CI runners.

Follow-up at the next checkpoint: the user reported all four jobs passing for
`e978cfe` before further implementation. That report was not independently
retrieved; it does not change the containment limitations below.

## Test candidate and security scope

`just probe-document-lifecycle-macos` builds three separate executables from
`tools/native/macos-lifecycle-probe/probe.c`: a test client, an embedded XPC
supervisor, and the supervisor's fixed direct child. The supervisor has only
App Sandbox entitlement; the child has exactly App Sandbox plus sandbox
inheritance. Nested binaries use ad-hoc hardened-runtime signatures, verified
strictly before execution. No parser is linked and nothing is bundled in ORT.

This is a new candidate alongside the earlier XPC probe, not a declaration that
the old cooperative-disconnect test proves forced termination. The supervisor
is trusted, owns child creation/reaping and adds security-sensitive code that
will require review before production integration. Inheritance is from that
minimal supervisor, not the unsandboxed desktop.

Apple documents [sandbox inheritance](https://developer.apple.com/library/archive/documentation/Miscellaneous/Reference/EntitlementKeyReference/Chapters/EnablingAppSandbox.html)
and [waiting for a child process](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/waitpid.2.html).
The prototype uses public spawn, signal, pipe, poll and wait APIs. It does not
use private audit-token APIs, an XPC-supplied PID or a process-group kill.
One supervisor thread owns `waitpid` and signals, SIGCHLD is default, and no
signal is sent after reaping or an unexpected wait failure. The direct-child
ownership model avoids using a PID that could have been reaped/recycled elsewhere.

Spawn uses a fixed absolute bundled executable, explicit stdin/stdout/stderr
file actions and close-on-exec-by-default. It clears the child's signal mask,
resets the relevant dispositions and passes only a fixed PATH environment.
There is no arbitrary executable, path, PID, shell or general operation supplied
by child output. Only the validated read-only synthetic input is transferred.
Production executable-identity and inherited-authority verification remain gated.

Before reading that input the child verifies its sandbox entitlement and sets
hard NPROC zero, NOFILE 64 and CORE zero. It checks the exact marker and that a
write through the descriptor fails. The stalled/flooding child ignores SIGTERM.
Its 12-second alarm is a bounded test fallback, not accepted as supervisor proof.
The test never uses real documents, a database, Keychain, external network or
account-wide settings. Fixture cleanup removes only the newly created private
synthetic directory; generated bundles/reports remain under ignored target/.

## Native cases and final measurement

All nine cases passed, including repeated native runs of the expanded suite.
The final-source report is retained in
[macos-lifecycle-report.json](macos-lifecycle-report.json), originally generated
at `target/native-probes/macos-lifecycle-sgX17J/report.json`. It includes hashes,
OS/architecture, per-case exit/signal/EOF/reaping results and byte counters.

| Synthetic case | Required result |
| --- | --- |
| Normal fixed output and exit zero | Accept only after reaping and both EOFs |
| Explicit cancellation after readiness | SIGKILL, reaped, rejected |
| Silent stalled child | Deadline, SIGKILL, reaped, rejected |
| Stdout flood | Output ceiling, SIGKILL, reaped, rejected |
| Stderr flood | Output ceiling, SIGKILL, reaped, rejected |
| Valid output followed by exit 65 | Reaped, rejected |
| Malformed output followed by exit zero | Reaped, rejected |
| Complete output without exit | Deadline, SIGKILL, reaped, rejected |
| Complete output and both EOFs without exit | Deadline, SIGKILL, reaped, rejected |

The synthetic supervisor retains at most a 64-byte stdout prefix, discards
stderr and drains each stream in fair nonblocking 1 KiB chunks. Test ceilings
are 4 KiB per stream with a one-second monotonic deadline, independent of pipe
activity. Over-limit counters saturate at 4097; they do not claim to count every
emitted byte. Cleanup observation is bounded at four seconds; failure cannot
become a successful result. These small test limits do not change the Rust
production-policy ceilings, and this is not its native adapter yet.

A sent kill, a ready message or a disconnected XPC connection cannot pass the
stop cases: the report requires SIGKILL OS status, successful reaping and both
EOFs. Signal 14 from the alarm fallback or signal 15 does not pass. Valid bytes
with failed exit and EOFs without exit remain negative cases.

## Verification and remaining gates

- Native code compiles with `-Wall -Wextra -Werror`; strict nested-signature
  verification passes. Clang static analysis is clean for all three roles.
- Five new report-validation tests reject missing/extra/replayed cases, wrong
  types, missing reaping/EOF evidence, wrong signals, incorrect deadlines/byte
  limits and unexpected success. They keep import/full-containment flags false.
- Full local `just check` passes. Contract regeneration has no drift. Both
  macOS CI jobs now run this new probe in addition to the existing sandbox test.
- No dependency, production Rust/UI, installer or desktop preview package change.

Still unproven: client or supervisor death/crash, broker-created descendants and
full process-tree termination, the inherited child's filesystem/network/vault/
native-IPC boundary, memory/CPU/thread/Mach-port ceilings, launch and cleanup fault
injection, signed-release identity and supported platform matrices. The supervisor
can create its fixed child by design; arbitrary child-triggered broker operations
must remain impossible and require their own adversarial tests.

Production parsing/review integration remains disabled. `ort-document-worker`
still exits 78 and `IMPORT_ENABLED` is false. M2 is not complete. Next work should
prove supervisor/client-death behavior and the remaining authority/resource
boundaries before integrating a PDF/DOCX parser.
