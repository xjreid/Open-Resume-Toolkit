# Step 4: M1 macOS Apple Silicon storage qualification

Date: 2026-09-05. Source baseline: `4bd25940e1a344fdf1ac984ab0a47fbd1a6b60dc`.
Status: **Step 4 local macOS-arm64 qualification complete; formal M1 signoff
awaits hosted CI for the containing commit**. Synthetic data only. The changes
in this checkpoint are uncommitted. No commit or push was performed.

## M0 baseline

M0 is complete for macOS-arm64 development at `4bd2594`; both hosted workflows
were independently checked through the GitHub API. See
`m0-macos-qualification.md` for the exact runs, qualified source/artifact hashes,
signer, and preserved platform/distribution limitations. Later source changes
do not inherit that artifact's native qualification automatically.

## Exclusive database-key creation repair

Review found a check-then-upsert race in `OsDatabaseKeyVault::store_new`.
Its mutex belonged to one adapter instance. Two independent callers could both
observe absence and then call `keyring::Entry::set_secret`, whose macOS backend
updates an existing item. The later write could therefore replace a key that
the earlier caller had successfully created.

macOS creation now uses the safe `security-framework 3.7.0` wrapper around
Keychain's add-only operation in the same User-domain Keychain. A native duplicate
maps to `AlreadyExists`; other native errors map to `Unavailable`. There is no
lookup, update, delete/recreate, or fallback in this creation path. Loading and
deletion remain unchanged. The dependency was already pinned transitively;
no package version, database format, namespace, ACL, signing policy, or trust
setting changed. Other platforms retain their existing compatibility path and
need an exclusive-create design before later native qualification.

`just test-platform-vault-concurrency` passed in the developer session outside
the agent sandbox:

- The negative control schedules two missing-item reads before two upserts.
  Both writes succeed and the second replaces the first synthetic value. This
  demonstrates that the old primitive cannot guarantee exclusive creation.
- Four native races each start eight independent adapters at a barrier. Exactly
  one creation succeeds, all other calls return `AlreadyExists`, and the loaded
  value matches the winning caller. Exact cleanup and subsequent absence pass.
- Each separate test process disables Keychain UI. Only five randomized items
  in `com.openresumetoolkit.platform-test.database` are created and deleted by
  the recipe. No developer profile key, signing key, trust state, or account
  configuration is touched. The earlier focused concurrency run also passed.

These are real native Keychain operations from a development test executable,
using concurrent threads in one process. They do not qualify signed application
ACLs, another process, another user, moves, updates, or a locked login Keychain.
Log: `target/m1-native-vault-concurrency.log` (ignored local evidence).

## Hostile backup and restore coverage

The public backup reader is exercised with every possible truncation of a valid
fixture; appended/concatenated containers; invalid magic/version/reserved/length
fields; KDF underflow/overflow/out-of-policy values; and an actual file buffer
above the 64 MiB payload ceiling. Valid positive controls still restore.

An independent test encryptor constructs correctly authenticated containers with
unsupported schema/version tuples, wrong inventory/hash, invalid metadata,
invalid revisions/documents, a prohibited credential field, secret-named and
oversized settings, malformed JSON, invalid UTF-8, duplicate struct fields,
excessively nested setting values, and trailing JSON. Where records deserialize,
the test preserves their canonical hash and inventory so a hash mismatch cannot
mask missing domain validation. All hostile cases must return `InvalidBackup`;
a valid independently encrypted container must reproduce the original records.
The existing 1.0 and 1.1 golden format vectors remain unchanged.

Storage integration checks preserve the active database/WAL/shared-memory/
manifest bytes, records, and memory-vault key across invalid restore attempts,
then reopen the profile. A separate test injects a SQLite trigger failure after
draft/publication insertion and verifies the entire restore transaction rolls
back; removing the trigger allows an intact restore. These use actual native
SQLCipher with temporary synthetic files and memory vaults. The trigger is not
ENOSPC, native Keychain denial, forced termination, or power-loss evidence.

Focused gate passed: `cargo test --locked -p ort-backup -p ort-storage -p ort-vault`
(9 backup tests, 38 storage tests, 1 isolated native-cipher startup test, 3 vault
unit tests; the 3 opt-in vault tests are ignored by this command). Focused strict
Clippy passed. Logs: `target/m1-focused-tests.log`.

The canonical `CI=true just check` passed on the final implementation: formatting,
TypeScript and JavaScript tests/builds, static security and dependency-license
checks, clean contract regeneration, Rust formatting, strict workspace Clippy,
and all workspace/all-target Rust tests. Opt-in native tests remain ignored by
that gate and are recorded separately above. Log:
`target/m1-canonical-check.log`. The normal host context was required because
pnpm resolved its virtual-store setting differently inside the agent sandbox;
an offline frozen-lockfile install reported the existing dependencies up to date.

Before updating the app, the installed M0 main window showed encrypted storage
ready, saved draft revision 20, and published snapshot 2. Three synthetic text
markers visible in the editor were absent in UTF-8, UTF-16LE, and UTF-16BE from
both the nonempty database (4,096 bytes) and WAL (193,672 bytes). Both files were
mode `0600`. This is marker-scan evidence, not proof of all-content confidentiality
or Keychain ACLs. Report:
`target/m1-qualification/installed-marker-scan-before-update.json`.

## Signed rebuild, update, and relocated-copy observations

`just package-qualification "ORT Local Test Signing"` passed on the final
implementation, followed by installation into `/Applications` and strict
installed-app verification. The previous installation is preserved at
`target/m1-qualification/previous-installed.app`; the M0 machine reports were
preserved under `target/m1-qualification/m0-baseline/` before rebuilding.

- Source implementation SHA-256:
  `c3d5f0348df9489e3009ce0b01af8d3b22f4bf28aea9e96d8426563a37fbffdf`.
- Updated executable SHA-256:
  `fc4332524553800c06b01eef070dd91d69e2249567d26ce8f2ec50c007e77e65`.
- Certificate SHA-256:
  `d4fdc787da71f95d9f5455c1a733e9d942d76409a72b92a27dff48da66c2cc49`.
- Designated requirement:
  `identifier "com.openresumetoolkit.dev" and certificate root = H"314932fe09cb08143ddc66fed25ba732fd20cfe5"`.
- Native arm64, hardened runtime, empty entitlements, and strict sealed-bundle
  verification passed. The code requirement and signer match the M0 build;
  the executable digest changed as expected. No private key was exported and
  no signing or Keychain prompt needed user interaction during these checks.

The updated installed app opened both routes with **Encrypted storage ready**.
Its main window showed the same saved draft revision 20, published snapshot 2,
synthetic title/contact/sections, and no pending recovery operation. Its process
path was independently checked as `/Applications/.../Contents/MacOS/ort-desktop`.

After a normal Command-Q, the same signed bytes were launched from
`target/m1-qualification/moved-location/Open Resume Toolkit Dev.app`. The
relocated copy passed the full verifier. Process-path inspection confirmed that
the relocated executable was running, not the `/Applications` copy. Both routes
reported encrypted storage ready and the main window retained draft revision 20
and published snapshot 2. The relocated copy was quit normally afterward.

This qualifies reopening this existing synthetic profile across this specific
same-identity update and relocated-copy launch. It does not establish the
profile key's original ACL, clean key creation by the installed app, cross-process
denial, another account, or behavior after denying a Keychain prompt. The original
installed path remained present while the relocated copy ran. No source profile
was reset, replaced, or deleted for these checks.

Reports for this earlier build are preserved under
`target/m1-qualification/before-empty-profile-fix/`: `build.json`,
`installed-app.json`, and `moved-app.json`.
Build log: `target/m1-package-qualification.log`. The relocation observation
applies to this earlier build, not the later UI repair below.

## Installed portable-backup creation and authentication

The user created `target/m1-qualification/developer.ort-backup` through the
installed app's native Save dialog, choosing and entering the passphrase locally.
An initial validation selection was canceled and was not counted as a pass.
After retrying, the user reported success and native accessibility inspection
independently observed the **Authenticated backup summary**:

- Format 1.1; 3,250 bytes; created `2026-09-05T20:34:23.550575Z`.
- Application `0.0.0-dev`; database schema 2; document schema 1.
- One draft, two published snapshots, zero settings, zero render records.
- The interface reports authentication and structural validation passed without
  changing the active profile. The editor still shows saved draft revision 20
  and published snapshot 2; no replacement or recovery operation is pending.

A bounded no-follow read confirmed the local file is regular, its size/header
agree, and its SHA-256 is
`534b839c574fc97677b665c2fd99b875533179f4d3d74169b012a427418c02b4`.
Authentication evidence comes from the installed app; the independent file read
checks identity and framing only and does not obtain the passphrase or decrypt
the archive. Content-free receipt:
`target/m1-qualification/developer-backup-receipt.json`.

Automatic approval review initially rejected the proposed shared copy and
`orttest` read/search access before execution. The user subsequently explicitly
approved that exact payload, recipient, and access change. The transfer then
completed to:

`/Users/Shared/ORT-M1-transfer-xnk_lzto/developer.ort-backup`

The developer-owned directory is mode `0700` and the encrypted file is `0600`,
with explicit ACLs granting only `orttest` directory list/search and file read
access (plus read-only attributes/security metadata). No write/delete grant was
added for `orttest`. The recipient resolves to UID 502. Source and destination
both match the 3,250-byte size and SHA-256 recorded above; the original is
preserved. No passphrase, device key, signing key, or login Keychain was copied.
The transfer receipt is appended to the local backup receipt. At this transfer checkpoint, actual access and restore from the real `orttest`
login were still pending; the later user-observed results are recorded below.
Developer-session file verification alone is not cross-account access evidence.

## Standard-account initial startup — user-observed

On 2026-09-05, after instructions to use Fast User Switching into the real
standard `orttest` login and open the installed app before restoring, the user
reported completing the check:

- **Encrypted storage ready** was displayed.
- The resume was empty, with none of the developer's resume or synthetic test
  data visible.
- No Keychain prompt appeared when opening the app or checking the resume.

This is user-observed native evidence, not a tool observation from the developer
session. It establishes successful isolated initial UI startup in this account.
It does not independently establish when the profile/key was created, compare
vault identities, prove developer-file/Keychain access denial, or qualify restart
and restore behavior. No account impersonation or elevated cross-account read
was used as evidence. Those separate gates remain open.

## Empty-profile backup controls repair

During the real `orttest` check, the user reported that backup fields and buttons
could not be used. No restore was completed. The loaded empty profile creates an
unsaved placeholder, which the ordinary editor dirty check intentionally treats
as dirty. The backup panel inherited that check and therefore blocked recovery
before the first draft existed.

The backup-specific guard now permits the untouched placeholder only: there is
no stored draft and the edit epoch is zero. Any edit, including an edit followed
by undo, restores the save-before-backup guard. Ordinary save, publication, and
quit behavior retain their existing dirty semantics. Busy-state serialization,
the exact replacement confirmation, and staged-until-restart recovery remain
required. No profile reset or dummy draft is needed.

Live UI regressions cover enabled empty-profile validation, canceled validation,
explicitly confirmed restore, disabled controls during the pending operation,
cleared passphrases, unchanged empty editor after staging, and no implicit save
or publication. Separate cases cover blocking after editing a new or stored
draft and after undo on the new draft. All 65 desktop tests and TypeScript checks
passed (`target/m1-empty-backup-tests.log`). The native retry subsequently passed
as reported below.

The user confirmed the `orttest` app was closed before replacement of the build.
The shared encrypted backup remains the same file and digest.

The full `CI=true just check` passed after the repair (log:
`target/m1-empty-backup-canonical.log`). The corrected arm64 app was rebuilt,
signed with the same certificate and designated requirement, and installed at
`/Applications/Open Resume Toolkit Dev.app` after confirming no ORT process
remained running. Its deep/strict signature and source digest checks passed:

- Implementation SHA-256:
  `d67087b8846c2c01a7f5a0d6d724748b04b0c5daefa0cabcb01257d66903bf7c`.
- Executable SHA-256:
  `25ff270a453a867708874f3b7f435ec46db1733e3ceedda373dec0157aa6feaa`.
- Current reports: `target/m1-qualification/build.json` and `installed-app.json`;
  build log: `target/m1-empty-backup-package.log`.
- Previous installed app retained at
  `target/m1-qualification/before-empty-profile-fix/previous-installed.app`.

Native accessibility inspection of the corrected installed app found encrypted
storage ready in both windows, with the existing developer draft revision 20,
published snapshot 2, and saved status intact. The developer app was quit
normally afterward. The first automation launch request timed out; a subsequent
inspection succeeded. This observation is developer-profile reopen evidence;
the separate standard-account result follows.

## Standard-account restore retry — user-observed

After installation of the corrected build, the user reported that all supplied
retry steps worked and passed: empty-profile backup controls, authentication of
the shared backup with one draft and two publications, confirmed replacement,
and quit/reopen recovery of draft revision 20, published snapshot 2, and synthetic
content. This is user-observed evidence from the real `orttest` login, not an
independent developer-session inspection of that account.

The user also reported that the first launch required their password for
Keychain access. This qualifies the earlier no-prompt observation: it applied
to the earlier startup, and must not be generalized to this corrected build.
The user clarified that they selected **Always Allow**, and the prompt did not
return for the remaining tests, including quit/reopen. The exact prompt text and
referenced Keychain item were not captured. No password was requested or recorded.
Successful recovery and nonrecurrence after explicit authorization are recorded
separately from the still-open Keychain access-policy qualification; these
observations do not establish the initial prompt's cause or a storage defect.
Retained safety-copy status and direct vault/file isolation were still unverified
at this point; the subsequent real-account helper checks below address them.

## Signed native failure harness

`node tools/qualify-storage-macos.mjs "ORT Local Test Signing" --installed-key-probe`
passed on macOS arm64. Final report and logs:
`target/m1-qualification/native-storage-ha2WQB/`; combined log:
`target/m1-native-final.log`. Signed helper SHA-256:
`18f7eab194330619b1b796519a821a4e0e2c2289a412426c0067c9ba404ade43`.
The helper uses the same local signing certificate with the distinct identifier
`com.openresumetoolkit.qualification.storage`, hardened runtime, and strict
signature verification. It is not the installed desktop app.

- Real native `platform-test` Keychain items and disposable SQLCipher profiles
  survive independent-process reopening. The parent SIGKILLs and reaps only its
  own child after a committed write remains in the WAL. Revision 2 and immutable
  publication 1 recover, with UTF-8/UTF-16 synthetic markers absent from database
  and nonempty WAL, both mode 0600.
- Deleting or substituting only a disposable profile's native key causes the
  expected missing-key or key-mismatch failure. All profile files remain exactly
  unchanged. Restoring the original synthetic key permits independent reopening.
- A ciphertext bit flip causes integrity failure without rewriting database or
  manifest. No successful store/plaintext fallback is returned.
- Real process termination occurs inside migration before COMMIT, after COMMIT,
  and after the manifest handoff rename. Each restart preserves draft/publication,
  reaches schema 2, validates checksums/integrity, and removes interrupted manifest
  bookkeeping. The v1 starting fixture is seeded deliberately; the migration and
  interrupted recovery use production functions. Hooks exist only in macOS test
  builds and have a 30-second child timeout.
- A separate native destination gets a distinct Keychain reference and key.
  Backup payload inspection finds no source profile/install IDs or raw/hex device
  key; secret-shaped settings make export fail closed. Restore succeeds with the
  destination key. Deletion removes that exact destination directory/key while
  preserving the other native profile and its key. Provider credentials are not
  implemented yet; this is a synthetic secret-shaped-setting exclusion check.
- The developer app key's exact metadata is present, but the distinct signed
  helper cannot load its secret with Keychain UI disabled (`Unavailable`). The
  user explicitly approved this read-denial probe after automatic approval review
  rejected the initial combined run. No key bytes were printed or permission
  changes made. This proves this caller's denial, not universal same-user malware
  isolation or native-host access (the host remains gated).
- A private 64 MiB HFS+ image reaches native ENOSPC. A bounded SQLCipher write
  eventually fails, committed records remain intact, the failed setting is absent
  after reopen, and writing succeeds after removing the filler. The image was
  detached successfully. This qualifies the storage primitive on bounded HFS+;
  it does not claim every installed UI operation or APFS-specific low-space path.

Initial harness attempts exposed test assumptions, not production fixes: export
rejects secret-shaped settings (saving encrypted settings does not), hdiutil uses
UDIF for a blank read/write image, and a failed 1 MiB allocation can leave smaller
usable extents. The corrected disk test requires an actual failed database write,
with an 80 MiB filler cap and at most 64 further bounded writes. Failed-attempt
reports are retained alongside the final pass. No developer or `orttest` profile
was modified by this harness.

## Bounded backup mutation campaign

`just test-backup-mutations` passed (`target/m1-backup-mutations.log`): 10,000
fixed-seed header mutations, of which 8,464 are rejected before KDF derivation;
128 fixed-policy ciphertext/tag mutations all rejected; valid authenticated
controls pass before and after. Header mutations which remain structurally valid
are inspected only, avoiding attacker-selected KDF resource costs in this bounded
campaign. This supplements the independently authenticated hostile-payload corpus;
it is reproducible mutation coverage, not sustained coverage-guided fuzzing or
release-candidate resource qualification.

## Real standard-account boundary checks — user-observed

The user ran the staged `m1-account-check.py before` in Terminal in the real
`orttest` login and reported **ALL CHECKS PASSED**. The helper requires UID/effective
UID 502, `/Users/orttest`, and active-console ownership. Its checks establish:

- Opening the developer database and login Keychain file fails with EACCES/EPERM;
  a missing path is not accepted as denial.
- The developer key address is absent from the test account's Keychain metadata
  search. The test account's active and retained safety keys are present, have
  distinct identities, and the active identity differs from the developer's.
- The restored database/WAL have no known synthetic markers in UTF-8/UTF-16 and
  do not have a plaintext SQLite header; scanned files have mode 0600. The shared
  encrypted backup still matches its recorded SHA-256.
- The retained pre-restore safety profile is present. Its key differs from the
  restored active key. No secret read is performed by this helper.

The content-free before receipt remains in `orttest`'s home. This is user-reported
execution of the reviewed helper, not developer-session impersonation or an
independent read of that account's files.

The user also performed Force Quit on the installed app in `orttest` after Saved,
then reopened and confirmed **Encrypted storage ready**, draft revision 20,
published snapshot 2, and unchanged synthetic content. The locked-Keychain
nonmutation and all-local-data deletion results follow below.

## Locked-Keychain denial and installed-account deletion — user-observed

The user completed the real `orttest` lock/deny test and reported that encrypted
storage was disabled when Keychain access was denied, and the read-only
`m1-locked-check.py after` comparison printed **ALL CHECKS PASSED**. The baseline
was taken after normal app quit. The comparison requires an identical inventory
and SHA-256 for every known file in the active/safety profile directories, so
this demonstrates refusal without a plaintext fallback or profile-file mutation.
The user subsequently completed the unlock/reopen prerequisite and deletion flow.

An earlier screenshot showed two command-entry mistakes: rerunning the already
completed account-baseline script was refused because its receipt existed, and
`usr/bin/security` lacked its leading slash. Neither was counted as a lock test.
The commands were corrected without replacing the successful account receipt.

The user then performed **Delete all local ORT data**, reopened the installed
app, and reported an empty working profile plus **ALL CHECKS PASSED (after)**
from the real-console account helper. The helper requires:

- Both old active and retained safety Keychain item addresses are now absent.
- The fresh active key exists and has a different identity.
- Old recovery directories and pending deletion/restore markers are gone.
- Developer database/Keychain-file access remains denied, and the developer key
  is absent from the test account search.
- The encrypted shared backup digest remains unchanged.

The user confirmed the test-account app was closed and returned to the developer
account. Independent developer-session comparison then found the developer
`profile.db` and `profile.json` byte-identical to their pre-deletion SHA-256
baseline. Reports: `target/m1-qualification/developer-before-account-deletion.json`
and `developer-after-account-deletion.json`. This adds positive developer-profile
preservation evidence to the standard-account deletion result.

## Final signed artifact

The final `CI=true just check` passed (`target/m1-step4-final-canonical.log`).
The final app build also passed (`target/m1-step4-final-package.log`), using the
same certificate and designated requirement recorded above:

- Source implementation SHA-256:
  `26f21ce4d8c9a84e6d15914eb45f480c60b8787d65d660a544dbd0cdca30fca2`.
- Executable SHA-256:
  `bf152b1d24c6bd82b7d74be661a5fb7fda12a62a8bf248dc5756f6fb2da5c512`.
- Installed at `/Applications/Open Resume Toolkit Dev.app`; arm64, hardened
  runtime, empty entitlements, strict signature verification. Local self-signing
  only, not notarized; DMG not refreshed.

After confirming both account apps were closed, the previous installed build was
retained under `target/m1-qualification/before-native-matrix-final-build/` and the
final app installed and verified. Native inspection found both windows ready,
with developer revision 20/snapshot 2 still saved. The app was quit, moved to
`target/m1-qualification/final-moved-location/Open Resume Toolkit Dev.app`, and
verified/launched while the original Applications path was absent. Process-path
inspection confirmed the relocated executable. Both windows were ready and the
same saved draft/publication remained available, without a Keychain prompt.
The app was quit, moved back to Applications, and signature/digests reverified.

Current reports: `target/m1-qualification/build.json`, `installed-app.json`,
`moved-app.json`, and `final-move-observation.json`. The earlier user restore,
lock/deny, Force Quit and deletion observations belong to the preceding signed
Step 4 artifact; the final build adds native qualification tools/test-only hooks
and preserves the same frontend. On the exact final installed build, the user
confirmed encrypted storage enabled with an empty resume in `orttest`. The
updated build requested the test-account Keychain password on first launch;
therefore this update is explicitly qualified with user authorization, not as a
prompt-free update. The user chose **Always Allow**, quit and reopened once,
and confirmed the empty profile opened successfully without another Keychain
prompt. This closes the final-artifact standard-account update/reopen check.

## Native matrix status

| Gate | Current evidence / remaining work |
| --- | --- |
| Signing consistency and installed update | Same signer/requirement and verified installed Step 4 app reopen passed |
| Keychain create/reopen | Native race and signed-helper create/reopen pass; real-console helper confirms active/safety key presence and distinct identities; user-authorized first-launch prompt does not recur |
| Moved app and same-identity rebuild/update | Final app actual relocation and saved-profile reopen pass with original Applications path absent; app restored and reverified |
| Cross-process access and explicit trust policy | Signed helper independent-process reopen and unapproved-helper denial of installed key pass; native host remains gated; local signer only |
| Denial and locked/unavailable Keychain | User-observed locked-Keychain denial disables storage and exact encrypted-file inventory/digests remain unchanged; unlock/reopen succeeds |
| Separate `orttest` profile and Keychain item | User reports reviewed real-console helper passed distinct developer/active/safety identities and native item metadata presence |
| `orttest` denial of developer profile/key | User reports real-console helper passed EACCES/EPERM for developer database/Keychain file and absent developer item in test-account search |
| Database/WAL plaintext markers | Three known synthetic markers absent in nonempty installed database/WAL before update; synthetic suite also passes |
| Normal restart and forced termination | Installed normal restart and user-observed Force Quit/reopen pass; signed-helper committed-WAL SIGKILL recovery passes |
| Migration interruption, corruption, wrong/missing key | Signed native helper passes SIGKILL at three migration boundaries, corruption, wrong/missing native keys and nonmutation/recovery |
| Low disk | Signed native SQLCipher helper passes actual ENOSPC in private 64 MiB HFS+ image, rollback, reopen and recovery; APFS/UI-wide matrix not claimed |
| Portable developer-to-`orttest` backup | Installed developer creation/authentication and shared-copy digest verification passed; user reports real-account validation, confirmed restore and recovered revision20/snapshot2 after restart passed |
| Credentials/device-key exclusion | Native helper proves independent device keys, payload identity/key exclusion and secret-shaped export rejection; real-account active/safety/developer identities differ |
| Active-account-only all-local-data deletion | User-observed installed orttest deletion/reopen and after-helper pass old-key/recovery removal, fresh key, preserved backup; developer database/manifest independently byte-identical |
| Hostile restore/fuzz | Authenticated hostile reader/integration corpus and bounded 10000-header/128-ciphertext mutation campaign pass; sustained release fuzz/resource qualification not claimed |

## Step 4 closeout and limits

All local Step 4 checks above are complete for macOS Apple Silicon development.
The canonical gate, signed installed/update/move checks, native storage helper,
bounded hostile-backup campaign, and real-account manual/helper checks passed.
Evidence deliberately distinguishes the helper, installed app, user observations,
and exact build checkpoints. One-time Keychain authorization on the tested
updates is recorded, not hidden as a prompt-free result.

The containing commit has not been created or pushed; its hosted CI is pending.
M0 remains complete at `4bd2594`. Formal M1 signoff waits for that CI result;
M2 remains incomplete and is separate work. Windows/Intel native qualification,
Developer ID/notarization, broader distribution/preview/native-host tests,
sustained release fuzzing and APFS/UI-wide low-space qualification are not
claimed. Hostile-file import remains disabled. The signing private key and login
Keychains were never copied, and user passwords/passphrases were not recorded.
The validated shared encrypted backup and local evidence remain available.

Suggested commit: `test(m1): qualify macOS vault backup and recovery boundaries`.

