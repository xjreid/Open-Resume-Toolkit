# M2 native output reader and resource measurements

Date: 2026-09-06. Local macOS arm64, Darwin 25.6.0. Uncommitted Step 5 work
on qualified M1 baseline `65518eb`. M1 remains complete; M2 remains incomplete.

## Bounded native output component

`crates/ort-platform/native/macos/worker_output.{h,c}` adds an adapter component
using owned, nonblocking, close-on-exec read ends of ordinary private pipes.
It performs at most one 8 KiB read per call, clamps poll waits to 25 ms, alternates
ready streams, and returns control after interruption or a readiness race.
The caller supplies the central supervisor's stdout/stderr byte limits.
The first byte beyond a limit causes sticky failure and closes both owned readers.
Stderr events retain only byte counts; returned byte buffers contain no stderr.
Buffered data precedes EOF, each EOF is reported once, and close is idempotent.

The component is **not linked into the app or a production XPC adapter**. It
does not launch a worker, prove containment, reap processes, verify descriptor
closure, or establish cleanup completion. Trusted wiring and the existing Rust
supervisor must still enforce the full launch, output and cleanup protocol.

`node tools/check-worker-output-macos.mjs` passed real-pipe tests with AddressSanitizer
and UndefinedBehaviorSanitizer: silent waits, descriptor flags, partial reads,
stderr fairness/redaction, exact byte limits, over-limit failure, ownership,
buffered EOF, interruption and fixed-size burst reads. Clang static analysis of
the reader and both resource probes completed without diagnostics. CI now runs
the reader regression on macOS; hosted results for these changes are pending.

Local report: `target/native-probes/worker-output-7JOS8m/report.json`.
Report fields deliberately retain `fullContainmentProven: false` and
`importEnabled: false`.
The full `CI=true pnpm check` also passed: 91 desktop, 24 contract, 30 tooling
and two extension tests, formatting, lint, frontend builds, security checks and
dependency licenses. Log: `target/m2-native-output-web-check.log`.

## Resource mechanisms did not qualify

`node tools/check-worker-resources-macos.mjs` compiles and runs isolated synthetic
helpers as an ordinary user. It changes only helper/owned-child limits. No parser,
document, profile, Keychain, network or installed application is involved.
No sanitizers are used for memory measurements because their reserved virtual
address space would contaminate the result. This runner measures behavior; its
successful exit is **not a resource-limit pass** and it is not a CI qualification gate.

The final local run outside the enclosing agent sandbox produced
`target/native-probes/worker-resources-hXJpMZ/report.json`:

- A 512 MiB-plus-one-page `PROT_NONE` mapping succeeded before limiting and was
  unmapped without touching those pages. The helper then reported
  445,746,184,192 virtual bytes and 1,343,488 resident bytes.
- Setting both `RLIMIT_AS` values to 512 MiB failed with `EINVAL` (22).
  Consequently, this run did not establish or test enforcement of that limit.
- The SDK-declared `task_set_phys_footprint_limit` call on the helper's own task
  at 512 MiB returned `KERN_NO_ACCESS` (8). It did not establish a footprint cap.
- CPU limit setup/readback at one second, hard-limit increase denial and the
  normal exit control passed. A spinning control with default signal handling
  terminated with `SIGXCPU`.
- A spinning child that ignored `SIGXCPU` continued for 7.999861 CPU seconds
  despite both CPU limit values being one second. The owning probe stopped it
  at its eight-second wall deadline and reaped it. That supervisor kill is
  explicitly excluded from the hard-CPU-limit success field.

Apple's upstream [VM map implementation](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/vm/vm_map.c)
rejects an address-space limit below the existing map size. That is consistent
with the local memory result; upstream source is not proof of this installed
kernel's exact implementation or of a resident-memory cap. Apple's archived
[setrlimit documentation](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/setrlimit.2.html)
describes CPU-limit signaling. The adversarial local result establishes why
signal delivery alone cannot qualify the intended hostile-parser CPU boundary.
The SDK exposes the footprint setter, but Apple's upstream
[task implementation](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/kern/task.c)
checks caller privilege and can reject access. Its presence in headers alone
does not establish availability to ORT's ordinary-user helper.

## Consequence and next gate

Do not translate successful limit readback, cooperative child behavior, a
baseline-relative address-space allowance, or periodic resident-memory polling
into a claim that the required hard 512 MiB memory / 30 CPU-second bounds exist.
The production adapter needs a separately justified, enforceable mechanism and
adversarial native measurements before it can return a valid launch receipt.
These probes do not establish that every macOS containment approach is impossible.

Import remains disabled. Full XPC/App Sandbox integration, hard resource bounds,
parent death/tree cleanup, credential/broker denial and the remaining native
qualification matrix are still open. No M2 build was installed by these checks.
