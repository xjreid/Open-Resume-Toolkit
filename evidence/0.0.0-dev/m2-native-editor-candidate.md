# M2 native editor qualification candidate

Date: 2026-09-07. This separately staged candidate contains the current
uncommitted Step 5 implementation on M1-complete `65518eb`. It is not an M2
signoff, hosted CI result, or replacement of the installed M1 application.

## Build and staging evidence

- Workspace strict Clippy and `cargo test --workspace --all-targets --locked`
  passed. Opt-in credential tests remain ignored. The immediately preceding full
  JavaScript gate passed with 102 desktop tests; see
  [entry guidance](m2-entry-guidance.md).
- `node tools/qualify-macos.mjs build 'ORT Local Test Signing'` passed the exact
  development configuration, release build, local certificate signing,
  hardened-runtime/no-entitlement inspection and deep/strict signature checks.
  The app is locally signed, not notarized.
- The copied app passed `qualify-macos.mjs verify` against the build's source,
  certificate and executable hashes. Only the app, a content-free build receipt
  and a launch helper were placed in a fresh shared folder. No profile or
  credential was copied. `/Applications/Open Resume Toolkit Dev.app` was not
  replaced or launched.
- The helper requires both the current user and active console login to be
  `orttest`, rejects an already-running ORT process in that account, and verifies
  the app's signature and exact executable hash before launching the explicit
  staged path. Shell syntax passed; a developer-account negative control exited
  with the switch-account instruction before reaching launch.

Candidate: `/Users/Shared/ORT-M2-native-pznun5tx/Open Resume Toolkit Dev.app`.
Launcher: `/Users/Shared/ORT-M2-native-pznun5tx/Open M2 Test.command`.

Implementation SHA-256:
`4000ebb53b821067d4639b33b986642614c87579e42e893f6de630593f18f1cb`.
Executable SHA-256:
`4ab058c517c3a9b9f5849a62640c455765cc5dda52a1b52b6c0ae23e2ec199c3`.
Signing certificate SHA-256:
`d4fdc787da71f95d9f5455c1a733e9d942d76409a72b92a27dff48da66c2cc49`.

Logs: `target/m2-native-qualification-clippy.log`,
`target/m2-native-qualification-tests.log`,
`target/m2-native-qualification-build.log`, and
`target/m2-native-staged-verify.log`. Full source/asset/signature receipt:
`/Users/Shared/ORT-M2-native-pznun5tx/build-receipt.json`.

## User-observed focused implementation check

1. In the real `orttest` login, close any running ORT app and run the launcher.
   Confirm encrypted storage is ready. Use only the synthetic test profile.
2. Build a Custom resume if needed. Set the resume title to
   `Synthetic M2 native quit` and wait for Saved.
3. Clear the resume title so it is invalid, then choose **Quit** from the app's
   Dock icon menu. Expect a close decision, disabled Save and quit, and an
   enabled Keep editing action. Keep editing must retain the empty title.
4. Repeat Dock Quit and explicitly discard the invalid edit. Expect exit.
   Reopen through the same launcher and confirm the saved synthetic title remains.

The user reported all four checks passed, including preservation after Keep
editing and restoration of the saved title after discard/reopen. They reported
a Keychain password prompt on initial launch. The choice made in that prompt and
repeat-prompt behavior were not separately reported; no broader Keychain result
is inferred. These are user-observed results for the exact staged candidate,
not agent-observed UI evidence or full Step 6 qualification.

No test account was switched and no Keychain approval, Dock quit, system
logout/shutdown or production editor launch was performed by the agent for this
candidate. Further checks include PDF pages,
overlay focus, repeated requests, in-flight work, native failure/interruption,
VoiceOver and the complete offline journey. Import remains disabled.
