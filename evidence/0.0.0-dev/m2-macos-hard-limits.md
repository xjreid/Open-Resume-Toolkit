# M2 macOS helper hard-limit checkpoint

Date: 2026-09-02. Base commit: `bdc3e10`. Local platform: macOS 26.6.2 arm64.
Status: expanded native subset passed locally; user subsequently reported all
four CI jobs passing after `723a97f`. The run log was not independently retrieved.
Full containment remains gated. The next independent supervision experiment is
recorded in [M2 macOS lifecycle](m2-macos-lifecycle.md).

## Previous CI result

The user reported all four CI jobs passing following `bdc3e10`, including the
previously failing Windows storage tests. This records user-provided evidence;
no successful-run URL/log was independently retrieved. It does not establish
Windows native vault/UI behavior or hostile-document containment. This new
hard-limit checkpoint later received the passing CI report noted above.

## Implemented experiment

The existing separate, ad-hoc signed App Sandbox/XPC test helper now performs
two phases: a plain sandbox baseline, then a helper-only hard-limit phase.
The unchanged baseline reproduces allowed direct child creation. After that
baseline, the same synthetic helper lowers both soft and hard limits to:

- `RLIMIT_NPROC = 0`;
- `RLIMIT_NOFILE = 64`;
- `RLIMIT_CORE = 0`.

Both parent and helper refuse root execution. Only the helper calls `setrlimit`;
no user-wide, launchd, desktop-app or shell configuration changes. The parent
records its limits before launching and verifies they are unchanged afterwards,
then successfully creates/reaps another fixed child as a positive control.

Apple's [setrlimit documentation](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/setrlimit.2.html)
defines hard-limit behavior. Its published [XNU process creation code](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_fork.c)
checks a non-root caller's NPROC ceiling. These sources inform this experiment;
they do not replace testing supported OS versions or broker-mediated launches.

No parser is linked or enabled. In production, limits and isolation must be
established before any parser code or untrusted input is processed. The synthetic
before/after phase is not a product lifecycle to copy into the importer.

## Native results

Two local executions passed. The retained final report is
[macos-hard-limits-report.json](macos-hard-limits-report.json), originally emitted
at `target/native-probes/macos-document-pLbujA/report.json`.

| Check | Plain App Sandbox | With helper-only hard limits |
| --- | --- | --- |
| Read transferred input; reject writes to descriptor | Pass | Pass |
| Deny seeded sibling read/write-open and symlink read | Pass | Pass |
| Deny parent's loopback TCP listener | Pass | Pass |
| Direct `posix_spawn` of fixed `/usr/bin/true` | Allowed | Denied |
| Direct `fork` with child immediately calling `_exit` | Allowed | Denied |
| Raise hard NPROC/NOFILE/CORE limits | Not applied | Denied; values unchanged |
| Exhaust helper descriptor budget; release and retry | Not applied | `EMFILE`; recovery passed |

Child `EAGAIN` is counted only when the non-root helper's zero soft/hard NPROC
limit is read back. Both child APIs must first succeed in that same helper.
Unexplained `EAGAIN`, missing files and other environmental errors are not denial
evidence. Filesystem/network tests continue to require `EACCES`/`EPERM`.

Descriptor exhaustion uses at most 65 duplicates of a known-good synthetic input,
not unbounded file opens. Every obtained duplicate is closed before another
successful duplication is required. Only the helper's small descriptor budget
is exhausted. Limit-raise attempts require `EPERM`; mere setter success/readback
does not stand in for these enforcement tests.
The tested ceiling concerns new descriptor allocation. Lowering NOFILE does not
close existing descriptors, and this probe does not inventory all inherited or
broker-transferred handles; a production adapter still needs that allowlist proof.

The core-file limit is verified as zero; no crash is deliberately induced, and
absence of all OS diagnostic artifacts is **not** proven. The helper still exits
cooperatively and the parent observes XPC disconnection; this is not forced
termination or parent-death cleanup.

## Validation and remaining work

- C builds with warnings-as-errors, hardened runtime and strict nested-signature
  verification. Final helper/host hashes and effective limits are in the report.
- Nine Node tests cover report shape/type/version, positive controls, baseline
  regressions, missing/soft-only limits, limit-raise and exhaustion failures,
  descriptor recovery, and the invariant that no result enables import.
- Both macOS CI jobs now require the expanded denial/limit checks. Synthetic
  metadata, hashes and measured limits are printed to CI logs for retention.
- Full local `just check` passed; generated contracts have no drift.
- No dependency, product UI, OS credential or personal resume changes. Synthetic
  fixtures were removed by the runner; ignored test bundles/reports remain.
  The previous desktop preview package was not rebuilt.

Still required: safe forced termination/cancellation/parent death and fresh-worker
identity, memory/CPU/thread/Mach-port ceilings, broker/native-IPC/credential denial,
executable replacement, broader filesystem/network coverage, hostile-code cleanup,
release signing and the supported platform matrix. Windows containment is still
unimplemented. Neither `fork` denial nor NPROC proves that an external service
cannot create a process on the helper's behalf.

The parser worker remains inert (exit 78), `IMPORT_ENABLED` stays false, and M2
is not complete. Next containment work should address supervision and remaining
authority paths before enabling PDF/DOCX parsing; do not treat this subset as a
general sandbox security guarantee.
