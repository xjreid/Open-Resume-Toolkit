# Security and threat model

## Status and scope

- Status: implementation baseline; security review required before stable release
- Owner: security maintainer plus component owners
- Applies to: desktop, local data, provider/Codex integrations, document processing, extension/IPC, installer, updater, and website claims
- Product authority: `../../Product Plans/Security_Privacy_and_Open_Source.md`

The product stores unusually sensitive employment material and user-supplied credentials. The design therefore assumes job pages, imports, AI output, local IPC peers, backup files, and update infrastructure may be malicious or compromised.

## Protected assets

1. Resume/profile content, job descriptions, application answers, tracker history, and exports.
2. Provider API keys, database key, native-IPC secret, and Codex authentication state.
3. Direct-provider spending caps and Codex quota-control state.
4. Application integrity: executable, templates, catalogs, update keys, and installer registration.
5. User trust in factual boundaries, change summaries, qualification alerts, and privacy claims.

## Trust boundaries

| Boundary | Untrusted side | Trusted receiving side | Required control |
|---|---|---|---|
| Web page to extension | page DOM/content script | service worker | explicit gesture, minimum data, normalization, size limit |
| Extension to native host | extension/runtime | host | browser origin allowlist, framed schema, one request per process |
| Host to desktop | local process | desktop IPC server | per-user endpoint ACL, mutual challenge, nonce and expiry |
| UI to Rust | bundled webview | Tauri command layer | narrow capabilities, runtime validation, no arbitrary shell/network |
| Import to document service | PDF/DOCX/text | parser | type sniffing, resource limits, isolated staging, hostile-parser tests |
| Job/resume text to AI | untrusted content | prompt builder/provider | delimiter isolation, least data, structured output, no tools |
| AI response to state | provider/Codex | validator | schema, size, reference, factual and policy validation |
| Backup to restore | arbitrary file | staging restore | authenticated decrypt, limits, checksums, schema validation, no path trust |
| Release service to updater | network/GitHub | installed app | TLS plus independent signed update metadata and artifact signature |

## Core controls

### At-rest data

- SQLCipher encrypts the entire database, indexes, journal, and WAL with a random 256-bit database key.
- The database key is generated locally and stored only in the operating-system credential vault. There is no plaintext fallback. If the vault is unavailable, ORT stops before opening user records and offers recovery guidance.
- Database, IPC socket/pipe, temporary directory, and diagnostic files receive per-user permissions at creation and are rechecked at startup.
- Sensitive temporary material uses a private application directory, randomized names, bounded lifetime, and best-effort deletion. The UI does not claim cryptographic erasure on SSDs.
- User-created exports are outside ORT's encryption boundary, and the export flow states that clearly.

### Secrets

Provider keys, the database key, and the native-IPC secret use separate vault entries with an install/profile identifier. Secret values never enter the UI event log, SQLite, command-line arguments, environment variables, backup, crash diagnostics, or clipboard automatically.

The vault is not treated as an application-identity sandbox on every platform. Windows Credential Manager generic secrets are protected from other accounts and offline access but are readable to processes in the same user context; tests and documentation state this limitation. On macOS, use the narrowest supported Keychain accessibility plus an explicit trusted-application/access-control design for the desktop and native host. Signed builds must bind access to the intended code requirements/access group. Unsigned previews must not silently fall back to plaintext or pretend to provide signed-app isolation; their move/update/repair prompts and limitations require a platform proof.

Only the vault adapter can obtain secret bytes. Its public application ports expose presence, creation, replacement, operation-scoped use, and deletion—not general read or enumeration. Provider and database secrets never cross into the webview or native host. The native host can obtain only the IPC secret. Buffers are short-lived, excluded from core dumps where the platform permits, memory-locked where practical, and zeroized after use.

Codex authentication is owned by the separately installed Codex runtime in an isolated ORT-specific configuration/keyring namespace. ORT stores only a connection record and safe account/status metadata.

### Application authority

Tauri capabilities are assigned per window and command. No window gets an unrestricted filesystem, HTTP, opener, process, or shell capability. Remote navigation and new-window creation are denied except for explicit, allowlisted links opened through the operating system after confirmation.

The CSP permits only bundled scripts/styles/fonts/images and the app's internal protocol. Dynamic code evaluation and remote source maps are prohibited in production.

All user/provider/imported text reaches React as text nodes or typed form values. Production lint/build policy rejects `dangerouslySetInnerHTML`, raw HTML rendering, string-to-code APIs, and unreviewed custom-protocol URLs. The internal document resource protocol accepts an opaque one-use or session-scoped handle plus a fixed operation; it never accepts a filesystem path, host, query-selected file, traversal segment, or arbitrary MIME type. A compromised webview therefore still cannot turn display data into filesystem, opener, shell, process, or network authority.

### Network

All direct-provider and catalog/update calls originate from Rust adapters with rustls certificate validation, explicit hosts, bounded redirects, timeouts, body limits, and proxy behavior documented to the user. The webviews and extension do not call AI providers.

Update keys and catalog keys are independent. Compromise or rotation of one cannot authorize the other.

### Prompt injection and factual integrity

- Job descriptions and imported text are labeled as untrusted data, not instructions.
- Direct-provider tools/function execution is disabled except the single structured-output schema mechanism.
- Codex tool, command, file, patch, browser, or approval events are prohibited and abort the attempt.
- AI outputs can propose content only inside the requested operation schema.
- Resume facts must reference stable input field IDs; introduced claims without evidence cause validation failure.
- Required Qualification Alerts must quote or span a mandatory requirement and cite the resume evidence used for a confirmed mismatch. Unsupported categories and speculative personal attributes are dropped.
- Alert types are only `confirmed_mismatch` and `not_found`; absence is not converted into a claim about citizenship, disability, authorization, sponsorship, or other sensitive status.

### Spending and quota abuse

Direct-provider caps reserve estimated maximum cost in the same transaction that creates an attempt. Unknown outcomes remain reserved. The adapter cannot dispatch when the enabled cap has insufficient capacity or pricing is unavailable.

Codex quota thresholds are based only on app-server account/rate-limit data with provenance and age. Stale or missing quota data is shown as unavailable and never fabricated from local token estimates.

## Codex containment gate

Application-level prompting is not sufficient containment. Before stable Codex support, a platform-specific proof must demonstrate:

1. only a tested external Codex executable and version is launched;
2. the child sees an empty, ORT-owned working directory and no resume files;
3. environment and inherited handles are reduced to an allowlist;
4. filesystem and subprocess access outside the sandbox are denied;
5. outbound network access is limited to the required Codex authentication/service endpoints at process level;
6. app-server tool and approval requests are rejected and treated as a security event;
7. termination kills the full child process tree and removes temporary state;
8. bypass attempts pass on supported Windows and macOS versions.

The executable gate is independent of the protocol/version gate. Resolve the canonical file and every parent, reject symlink/reparse redirection and unsafe ownership/write permissions, and verify the expected official distribution provenance using the strongest stable platform evidence available (code-signing/notarization identity, package receipt, and/or release-manifest digest). A manual picker can locate a runtime but cannot waive these checks. Test a counterfeit executable that prints the expected version and protocol handshake; it must never be launched beyond a non-executing identity check, or—when a bounded version probe is unavoidable—must already be inside the external sandbox with no user-data access.

The official app-server surface includes command, filesystem, process, tool, approval, permission, MCP, skill, collaboration, and experimental operations. ORT initializes without experimental capabilities, sends only an allowlisted method set, never calls `thread/shellCommand`, `command/*`, `process/*`, or `fs/*`, and terminates on any related server request, item, or notification. `approvalPolicy: never` is defense in depth, not the containment boundary. ORT never returns approval, elicitation, permission, or tool output.

If supported public OS mechanisms cannot enforce these properties without administrative installation, unstable private APIs, or a misleading user promise, stable builds ship with Codex mode disabled. This outcome does not block No AI or Direct API modes.

## Native IPC protocol controls

- Endpoint name contains a random installation identifier, not user data.
- Windows named pipe uses the current user's SID ACL and rejects remote clients.
- macOS Unix-domain socket resides in a `0700` directory and the socket is `0600`.
- A 256-bit vault secret authenticates an HMAC challenge-response; requests include protocol version, nonce, monotonic sequence, and a 60-second expiry.
- The desktop maintains a bounded replay cache.
- Origin metadata from the native-messaging launch must match configured Chrome/Edge production or development extension IDs.
- Input is UTF-8, schema-valid, and at most 256 KiB framed / 128 KiB captured content.
- The host never accepts an arbitrary path, executable, URL scheme, or command from the extension.

## Import, rendering, and archive controls

- File type is determined from magic bytes/container structure, not extension alone.
- PDF/DOCX parsing runs in a new disposable worker process for each import. The OS sandbox denies network, subprocess creation, vault/keychain/credential-manager access, database/application-data access, browser/native IPC, and reads outside the already-open staged input. The worker can write only to a randomized private result directory and returns a bounded versioned extraction message over an inherited pipe.
- The parent opens and validates the staged file before worker launch, passes a handle rather than trusting a worker-supplied path where supported, revalidates all returned data, and kills the entire worker process tree after success, timeout, crash, protocol violation, or cancellation. Worker exit cannot commit or mutate canonical data.
- PDF/DOCX parsers also enforce page, relationship, nesting, decompression, image, object, handle, memory, CPU, and wall-time limits. Fuzzing/resource limits do not substitute for the sandbox.
- Native implementation candidates and the required positive-control/access-denial
  test matrix are in `Document_Worker_Containment.md`. The implemented bounded
  transport policy has only synthetic-event evidence; it cannot enforce OS
  isolation, wake blocked I/O, kill a process or prove cleanup. A separate native
  macOS probe verifies read-only descriptor transfer, seeded sibling/symlink
  restrictions and loopback denial locally, but **allows child creation** and
  observes only cooperative disconnect. It does not establish credential/broker
  isolation, resource ceilings or forced process-tree cleanup. Windows containment
  is unproven. Import stays disabled; probe completion is not full containment.
- External DOCX relationships, macros, embedded packages, scripts, and active content are never executed or fetched.
- Render templates are bundled and addressed by known IDs; user content cannot inject Typst source.
- Resume/cover-letter file drags expose only validated PDFs materialized in a random private ORT session directory. Paths cannot be supplied by web content; Finish/discard and startup recovery remove only containment-verified ORT-owned files. Download uses a separate one-use native dialog token.
- Links are parsed as data, allow only approved schemes, and are escaped by the renderer.
- Backup payload entries use logical IDs rather than paths. Restore never joins an archive-provided path to disk.
- Backup clear headers are bounded and canonicalized before any allocation or KDF work. Argon2id parameters outside the accepted memory/iteration/lane policy fail before derivation. Authentication succeeds before decompression, manifest parsing, or archive entry allocation.
- Fuzzing covers parser panics, decompression bombs, malformed UTF-8, integer overflow, and partial files.

## Update and supply-chain controls

- CI actions are pinned to commit digests and receive minimum permissions.
- Pull-request jobs cannot access signing or publishing secrets.
- Release artifacts are built once, scanned/tested, then signed and promoted.
- Windows code signing, Tauri updater signing, GitHub artifact attestations, checksums, and SBOMs are all verified independently.
- The updater rejects rollback below the installed security floor, cross-channel metadata, expired metadata, incompatible database migrations, and bad signatures.
- Compromised-key runbooks revoke publication, rotate the affected key, ship a manually verified recovery build if needed, and clearly distinguish code-signing from updater/catalog keys.

## Privacy verification

Automated tests intercept all process network destinations for critical offline journeys. Stable release requires proof that:

- offline authoring/import/render/export causes no network request;
- the website and extension receive no desktop content;
- API calls contain only the fields declared by the operation minimizer;
- clearing AI Monitoring history does not reset guardrails;
- backups exclude credentials and device-bound secrets;
- diagnostic bundles contain none of the seeded marker strings from synthetic content/credentials.

## Security test matrix

| Test class | Examples | Gate |
|---|---|---|
| Unit/property | URL sanitizer, cap arithmetic, schema references, HMAC expiry | every PR |
| Fuzz | PDF/DOCX, backup, IPC, AI JSON, catalog parser | scheduled and release candidate |
| Integration | vault unavailable, database tamper, replayed capture, invalid update signature | release candidate |
| Platform | vault access matrix, parser sandbox escape attempts, permissions, process-tree kill, native-host registration, updater rollback | every supported OS/channel |
| Adversarial AI | page prompt injection, fabricated facts, required/preferred confusion | every prompt/schema release |
| Manual review | CSP/capabilities, release permissions, Codex sandbox evidence | stable release |

## Required evidence

- threat-to-test traceability table;
- dependency and binary review;
- redacted network capture for offline and AI journeys;
- Codex containment report per supported platform/version;
- installer/update signature verification output;
- restore-fuzz and corrupted-database results;
- accessibility/security interaction review for warnings and confirmations.

## Remaining bounded technical questions

- Which public Windows sandbox/network-control primitive can meet the Codex constraint without elevation?
- Which supported macOS mechanism can meet the same constraint without relying on deprecated/private APIs?
- Which supported public Windows and macOS primitives enforce the hostile-document worker boundary on every advertised OS version?
- Does the selected SQLCipher build configuration produce the same encryption and recovery behavior on all architectures?
- Can the unsigned macOS preview authorize both the desktop and native host to one IPC Keychain item without weakening access or creating misleading prompts? If not, browser integration remains disabled for that preview form.
- Can the Microsoft Store fallback retain native-messaging registration under its final packaging identity?

These are implementation gates, not permission to reduce the user-facing promise.
