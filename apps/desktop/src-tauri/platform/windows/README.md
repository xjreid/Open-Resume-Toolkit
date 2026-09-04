# Windows boundary

This directory is reserved for reviewed Windows composition, installer hooks,
registration, and platform tests. M0 grants no additional Windows authority.

The M2 document-import adapter remains unimplemented. Its production boundary is
a zero-capability AppContainer token plus an explicit inherited-handle list,
child-process mitigation and a kill-on-close/no-breakaway Job Object assigned
before parser execution. The job must enforce one active process and the memory,
CPU and handle ceilings required by `ort_documents::worker_supervisor`. A bounded
overlapped pipe/event driver must implement `ContainedWorker`, terminate the
whole job on every path, and report cleanup only after worker wait/accounting,
both pipe closures, source/result-handle closure, an empty job and Job teardown.
A receipt is not a substitute for native AppContainer token/ACL/Job evidence.

Do not add import commands, package capabilities or parser installation until the
remaining gates in `Implementation Plans/System Documentation/Document_Worker_Containment.md`
pass on every supported Windows version, architecture and package identity.
