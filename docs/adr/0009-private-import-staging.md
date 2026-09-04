# ADR 0009: Operation-owned private import staging

- Status: accepted for Unix; Windows ACL implementation remains gated
- Target milestone: M2

## Decision

Stage a preflighted document beneath the fixed `imports` child of the trusted
native application-data directory. Each operation receives a UUIDv7 directory
containing exactly two fixed names: a bounded content-free ownership marker and
`source.bin`. Unix implementations require `0700` directories and a `0600`
source, use capability-relative operations, and transfer the held read-only file
exactly once to the native containment adapter. No selected path or
caller-controlled stage filename crosses that boundary.

The application layer owns the composition: inspect the selected snapshot's
format envelope, stage those same owned bytes, transfer the input once, launch a
contained worker, run common supervision, close adapter-owned handles, and remove
the exact stage. Cleanup failure overrides otherwise valid extraction. The
public extraction path remains guarded by `IMPORT_ENABLED`; while false it
removes the prepared stage without calling a launcher.

On startup, a bounded scavenger may remove only stages older than 24 hours whose
UUIDv7 name, directory type/mode, exact two-entry inventory, regular-file types,
bounded marker, declared format/length, and actual source length all match.
Unknown, fresh, symlinked, malformed, over-limit, or additional content is
preserved for explicit repair. Recursive deletion and scanning outside the fixed
private root are forbidden.

## Consequences

The source selected from a user path is not reread after preflight, and a parser
adapter receives a handle rather than a path. Drop performs best-effort exact
cleanup, but only explicit successful cleanup can satisfy later supervision and
release evidence. Source bytes remain ordinary sensitive memory and are not
claimed to be securely erased.

The current implementation fails closed on Windows because current-user-only
ACL creation and verification is not yet implemented. It also does not provide
an XPC/App Sandbox or AppContainer/Job adapter, parser, native pipe driver,
desktop import command, or permission to enable import.
