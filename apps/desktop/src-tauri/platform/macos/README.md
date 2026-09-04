# macOS boundary

This directory is reserved for reviewed macOS composition, entitlements,
packaging hooks, and platform tests. M0 grants no additional macOS authority.

The M2 document-import adapter remains unimplemented. Its production boundary is
a separately signed minimal App Sandbox XPC supervisor and fixed
sandbox-inheriting child. Before parser execution it must establish the exact
descriptor allowlist, read-only staged input, private output, minimal environment,
no shared app/Keychain entitlements, network/filesystem/credential/IPC denial,
child denial and hard resource ceilings required by
`ort_documents::worker_supervisor`. Its bounded event driver must implement
`ContainedWorker`, terminate the owned direct child on every path, and report
cleanup only after reaping, both pipe EOFs, handle closure, empty-tree checks and
XPC teardown. A receipt is not a substitute for the native adversarial matrix.

Do not add import commands, entitlements or parser packaging until the remaining
gates in `Implementation Plans/System Documentation/Document_Worker_Containment.md`
pass for every supported macOS version, CPU and release signing identity.
