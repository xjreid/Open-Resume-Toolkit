# M0–M2 development milestone closure — 2026-09-09

**M0, M1 and M2 are complete for macOS Apple Silicon development.** The user
explicitly authorized closure after the final focused runtime checks. M3 is next;
this decision does not claim production distribution or other-platform support.

| Milestone | Completion evidence |
| --- | --- |
| M0 — architecture and contracts | [Qualified checkpoint](m0-macos-qualification.md), commit `4bd2594` |
| M1 — encrypted local core | [Qualified checkpoint](m1-macos-storage-qualification.md), commit `65518eb` |
| M2 — offline resume path | [Final account report](m2-round4-results.md), [quit repair and local verification](m2-quit-fix.md), earlier evidence linked from [acceptance history](m2-final-native-acceptance.md) |

## Final observed result

The exact candidate `/Users/Shared/ORT-M2-acceptance-rn13guag/Open Resume Toolkit Dev.app`
matched desktop SHA-256 `34b674a0f3061d1e1c8b9d02fb713862396fcf57434cb17180b3c95d19f94b9b`.
On the real `orttest` login, menu Quit with the overlay frontmost and an unapplied
17-block review exited in about two seconds. Reopening retained title
`M2 final quit check`, saved revision 2 and zero published snapshots, with no review
pending. Command-Q also exited in about two seconds. All focused runtime criteria passed.

## Accepted limitations and follow-up

- The test account's strict signature check returned `CSSMERR_TP_NOT_TRUSTED`;
  the tester launched the exact candidate directly. Developer-host strict signing
  verification passed. Test-account trust setup remains unresolved, not a passing
  launcher check. Track it with development signing/clean-machine packaging before
  relying on that launcher for later acceptance or claiming distribution readiness.
- Remaining round-three manual checks were waived by the user and remain unrun.
  This includes unsampled Keychain/metadata, filesystem/lifecycle, accessibility,
  reversed-date warning and full offline-journey evidence. Earlier observed and
  user-reported results retain their original attribution.
- Restore archive selection was inconsistent with the recorded backup A identity;
  that replacement observation remains inconclusive, not a proven restore defect.
- Future features must extend cleanup inventories for newly stored credentials,
  AI ledger records, workspace data or native IPC state as their plans require.
- M2's baseline CI pass at `295e3b2` was user-reported. Later local fixes and tests
  are documented separately; these working-tree changes are not yet committed and
  no new hosted CI pass is claimed. Commit/CI integration is a separate next action.

The supplied report's earlier pending-signoff conclusion is preserved verbatim in
its copied evidence. This later user-authorized scope decision supersedes it and
all earlier M2 pending/full-matrix instructions. Technical requirements and
historical failures are preserved; waived evidence is never relabeled as passed.
