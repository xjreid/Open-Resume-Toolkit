# M2 final quit acceptance results — 2026-09-09

## Candidate identity

- Candidate: `/Users/Shared/ORT-M2-acceptance-rn13guag/Open Resume Toolkit Dev.app`
- Desktop executable: `Contents/MacOS/ort-desktop`
- SHA-256 observed: `34b674a0f3061d1e1c8b9d02fb713862396fcf57434cb17180b3c95d19f94b9b`
- Manifest SHA-256: `34b674a0f3061d1e1c8b9d02fb713862396fcf57434cb17180b3c95d19f94b9b`
- Identity comparison: matched.
- `codesign --verify --deep --strict` result on this login: failed with `CSSMERR_TP_NOT_TRUSTED` (arm64). The supplied `Open M2 Test.command` would consequently stop before launch. The candidate was launched directly by its exact tested path for the focused runtime checks below; this signature-trust result is not represented as a pass.

## Focused observed acceptance

1. The supplied candidate launched in the `orttest` desktop login. Both the main editor and overlay reported **Encrypted storage ready**.
2. The existing synthetic draft title was set to `M2 final quit check`. Autosave completed, reporting **Draft revision 2 saved securely** and **Saved**.
3. Saved state before the menu-quit check: title `M2 final quit check`; revision `2`; published snapshots `0` / **No snapshot**.
4. Imported `/Users/Shared/ORT-M2-acceptance-rn13guag/Synthetic resume.docx`. The review displayed 17 undecided blocks and 0 kept. It was deliberately left unapplied.
5. With **Open Resume Toolkit Overlay Dev** frontmost, opened the app menu and selected the exact native item **Quit Open Resume Toolkit**. The candidate exited within about two seconds: both candidate windows disappeared and the desktop app inventory reported it not running. **PASS.**
6. Relaunched the same candidate. It again reported **Encrypted storage ready**. The main editor showed title `M2 final quit check`, revision `2`, **Saved**, and published snapshots `0` / **No snapshot**. The normal **Import an existing resume** button was present and no import-review UI was pending. **Persistence and discarded unapplied review: PASS.**
7. Used **Command-Q** from the relaunched candidate. Within about two seconds both windows disappeared and the desktop app inventory reported it not running. **PASS.**

## Result

The focused menu-quit, Command-Q, and persistence criteria passed. However, final scoped acceptance remains pending resolution of the observed local development-signing trust failure (`CSSMERR_TP_NOT_TRUSTED`) when running the supplied strict launcher verification.

The session tested only the repaired application-menu quit regression, Command-Q, and saved-data persistence. It does not claim release/distribution qualification, other-platform qualification, a new hosted CI result, or outcomes for the skipped round-three checks. The prior restore/archive-selection ambiguity remains inconclusive.
