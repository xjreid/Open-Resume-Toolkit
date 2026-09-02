# M2 import transport and Windows CI diagnosis

Date: 2026-09-02. Base commit: `1554540`. Verification platform: macOS arm64.

## Windows failure

The supplied windows-2025 log ends with a stack overflow in
`confirmed_candidate_saves_once_without_changing_published_snapshot_and_survives_restart`,
running in `ort-application --test import_storage`:
`0xc00000fd / STATUS_STACK_OVERFLOW`. The OpenSSL LNK4099 missing-debug-symbol
warnings preceded it but are not the reported failure. This was a runtime test
failure, not an expected consequence of M2 being unfinished.

The log contains no native backtrace or stage marker, so it does **not** establish
the responsible function/library or distinguish recursion from excessive stack
use. The same tests passed locally with the default stack and `RUST_MIN_STACK=262144`;
that is not Windows verification. No speculative dependency upgrade, encryption
relaxation, blanket stack increase, or skipped failing test is included here.
**The Windows failure remains unresolved pending native diagnostic evidence.**

CI now first runs an independent `ort-storage --test native_startup` executable
with static progress labels around native initialization, cipher availability,
synthetic encrypted profile creation, save/publish, integrity and reopen. This
uses a temporary profile and memory vault; it does not touch OS credentials.
The original import-storage executable retains cold/concurrent initialization,
now with content-free progress labels and uncaptured output. It runs even if
the standalone startup probe fails (provided the build succeeded). Neither
test failure is ignored. All three desktop OS jobs run both probes.

On the next Windows run, compare the last startup/import labels. A crash before
the first label suggests test-entry/setup stack use; during `Connection` opening
narrows investigation to native startup. A passing independent startup but a
failing import process requires a native debugger or further targeted probes of
the marked stage, including concurrent initialization. Do not call this fixed
until Windows execution passes and the cause/repair is understood.

## Transport checkpoint

Implemented a backend-only one-result transport policy with 8 KiB chunks,
512 KiB stdout, 16 KiB discarded stderr, both-EOF/successful-OS-exit gating,
terminal errors, cancellation and a 60-second monotonic absolute deadline.
It never launches a worker or writes a profile; it grants no parser authority.

Nine tests cover chunk splits, Unicode byte boundaries, both exit/drain orders,
exact/over limits, stderr floods, partial/multiple/trailing JSON, missing/duplicate
events, failure/signal exits, silent and trickle timeouts, late completion,
cancellation, clock regression, I/O failure, format mismatch and debug redaction.
These are synthetic transport-policy tests, not actual pipe/kill/sandbox tests.

Full check results are recorded in `manifest.json`. Native containment remains
pending. No app bundle was rebuilt, no GUI changed, and no Keychain access was
needed. The last packaged text-export app remains the prior artifact.

Implementation candidates and native proof checklist:
[Document worker containment](../../Implementation%20Plans/System%20Documentation/Document_Worker_Containment.md).
M2 remains incomplete; imports and public release remain disabled.
