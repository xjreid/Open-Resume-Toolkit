# Security, privacy, and open-source governance

## Privacy promise

ORT may accurately describe itself as local-first when all canonical user content remains on the user's device. It must not imply that AI-assisted content never leaves the device.

The project must distinguish:

- Local editing, storage, search, rendering, backup, and export.
- Deliberate transmission to a user-selected direct AI provider or to OpenAI through a user-authorized ChatGPT/Codex subscription.
- Update checks and browser-store communication.

Project maintainers cannot access local resumes, job descriptions, tracker entries, generated materials, or AI keys unless a user deliberately shares them outside the application.

## Protected assets

- Master draft, published master, imported content, application workspace, tracker, retained snapshots, AI activity ledger, AI guardrail state, Codex usage snapshots, backups, and exports.
- Direct-provider credentials, Codex authentication tokens, and provider account information.
- Local database encryption keys when implemented.
- Native-messaging and local-IPC secrets.
- Release signing/notarization credentials, Store accounts, repository administration, CI workflows, and update metadata.

## Main threats and required controls

### Malicious web content and prompt injection

- Captured content is untrusted and never becomes an instruction or executable action.
- Every extension/bridge/desktop boundary validates origin, schema, size, freshness, and action.
- AI output is validated structured data and never executed.

### Local data exposure

- Restrictive per-user permissions protect application storage.
- The operating-system user session is the primary access boundary. ORT must warn that people sharing the same unlocked OS account may be able to access the local profile.
- Credentials use the operating-system vault with separate database, provider, IPC, and Codex namespaces. The implementation documents the exact Windows credential type/persistence and macOS accessibility/access-control settings and tests access from the desktop, native host, unrelated same-user process, other user, moved app, and upgraded app.
- ORT does not claim that Windows generic credentials or an unlocked user Keychain defeat malware already running as that user. The desktop and native host run without elevation, minimize secret lifetime, zeroize copied buffers where supported, and never expose a secret-reading UI/IPC command. Optional user-presence protection may be added only if recovery, native-host startup, accessibility, and background-operation behavior remain understandable.
- Logs and diagnostic exports exclude content and secrets by default.
- Backups are encrypted and require a user-controlled passphrase.
- Clipboard use, temporary preview files, drag-and-drop exports, and recent-file behavior receive explicit implementation review because they can create copies outside ORT.

### AI credential, cost, and quota abuse

- API keys and Codex tokens use the OS credential store and never enter content records, activity rows, guardrail state, logs, backups, exports, extension messages, or diagnostics.
- Direct keys receive random local identities; ORT does not persist key prefixes or hashes as identifiers.
- Spending caps use atomic reservations and durable counters so concurrent dispatch, retry, crash recovery, activity deletion, clock changes, and migration cannot bypass them.
- A configured hard cap fails closed when price, currency, usage, or reservation state cannot be evaluated. An unknown provider outcome is never treated as free.
- ORT explains that local direct-API caps cannot see use of the same key elsewhere and that Codex quota values can reflect other clients. Provider-side billing/usage controls remain authoritative.

### Codex app-server containment

- ORT uses managed ChatGPT authentication in an ORT-specific configuration/authentication root and keyring namespace. It never asks for a password, scrapes cookies, imports a general-purpose Codex authentication file, or changes another Codex client's model, plugin, permission, or login configuration.
- Each operation runs in an empty ORT-owned scratch directory under a process-level sandbox with restricted read-only filesystem access and provider-only network egress. ORT configures no dynamic tools, web, MCP, app, plugin, skill, or collaboration access, sets approval policy to never, and interrupts/fails on any command, file-change, tool, web, connector, or permission event.
- The separately installed app-server executable and protocol are identity/version/capability checked before use; the supported version window is explicit, and unexpected model rerouting or tool activity fails the operation rather than broadening access.
- The integration accepts only reviewed input and validated structured final output. Codex threads and scratch content follow bounded cleanup rules and do not become a second resume store.

### Malicious imports and archives

- PDF, DOCX, backup, and other imported files are untrusted.
- Use operating-system quarantine, reputation, or malware-scanning facilities when available and appropriate, but never treat a missing alert as proof that a file is safe.
- PDF and DOCX parsing is performed in a dedicated, disposable, least-privileged worker process. The worker has no provider/database/IPC secrets, no network, no child-process authority, no writable path except its private staging/output directory, and read access only to the staged input. Operating-system sandboxing is mandatory for stable builds; resource limits and fuzzing supplement rather than replace process isolation.
- The parent accepts only a bounded, versioned extraction result, validates it again, kills the full worker tree on completion/timeout/error, and treats a crash or sandbox violation as an import failure. A platform that cannot enforce the worker boundary does not advertise document import until an equivalent reviewed containment design exists.
- Reject unsafe paths, archive traversal, excessive expansion, unsupported embedded content, malformed relationships, and executable payloads.
- Do not run document macros, scripts, external links, or embedded objects.

### Rendering, links, and structured snapshots

- Escape user and AI content before HTML, PDF, DOCX, and preview rendering.
- Allow only explicitly approved web/email link schemes; block script, data, local-file, executable, and unsafe custom schemes.
- Never place resume, job, provider, or imported strings into `innerHTML`, executable template source, CSS, script, custom-protocol paths, or webview navigation. The internal resource protocol serves only opaque, single-purpose handles for already validated bytes and cannot translate a URL path into an arbitrary filesystem read.
- Generated documents contain no hidden prompts, credentials, internal identifiers, temporary paths, or unnecessary metadata.
- Structured snapshots cannot select executable code, arbitrary local paths, unapproved templates, remote fonts, or unsafe renderer options.
- Validate nesting, strings, collections, decompression ratio, schema version, integrity checksum, and renderer compatibility before opening historical material.

### Update and supply-chain compromise

- Protect repository and Store accounts with MFA; use hardware-backed or phishing-resistant factors for release maintainers where possible.
- Restrict CI release permissions, pin or verify build actions/dependencies, review build-script changes carefully, and keep signing credentials outside the repository.
- Verify update signatures and channel identity before installation.
- Unsigned previews are not a stable trust channel. Checksums and provenance help expert verification but do not replace Windows code signing or macOS Developer ID/notarization; preview documentation must warn users that bypassing platform reputation controls increases installation risk.
- Verify signed pricing/model-catalog source, schema, sequence/effective dates, and rollback resistance; catalog data cannot carry executable code or broaden model/tool permissions.
- Generate software bills of materials and provenance when the implementation toolchain supports them.
- Maintain an embargoed security-reporting path and a documented revocation/recovery procedure for compromised releases.

### Local IPC abuse

- Allowlist exact extensions and restrict IPC to the current OS user.
- The native host exposes a narrow typed protocol, not a general command or filesystem interface.
- Browser messages cannot select arbitrary local paths or invoke arbitrary binaries.

## Telemetry and diagnostics

- No resume, job, answer, credential, full URL, filename, or document content is collected automatically.
- Initial releases should operate without centralized telemetry.
- The AI accounting ledger is encrypted local product data recording calls made by ORT and supplying aggregate AI Monitoring; it is not maintainer telemetry and is never uploaded automatically.
- Activity records exclude prompts, responses, document content, API keys, provider-account credentials, and full URLs. Raw provider-reported measurements remain distinguishable from ORT estimates.
- Codex account-level token/quota snapshots remain local, are labeled separately from ORT-only activity, and are never used as maintainer analytics.
- The application may generate a local diagnostic bundle that the user previews and deliberately attaches to a GitHub issue or support request.
- A future opt-in crash service requires a separate product decision, published data fields, retention, processor review, and a true off switch.

## Data deletion

There is no remote account-deletion workflow because ORT holds no product account or cloud copy. ORT provides local deletion and clear instructions for removing:

- Current application workspace
- Individual tracker artifacts or entries
- Direct provider credentials and the local Codex sign-in/session
- Selected or all AI activity records
- AI guardrail policies and counters through a separate confirmed reset/removal action; clearing activity alone never resets them
- Local database and settings
- Backups and exported documents, which may reside outside the app's control
- Browser-extension data and native-host registrations

The application cannot delete copies the user saved elsewhere or transmitted to an AI provider. Documentation points users to the provider's own data/account controls.

## License

The repository license is **GNU General Public License v3.0 only (`GPL-3.0-only`)**. GitHub labels its matching license template **GNU General Public License v3.0**. This permits commercial and noncommercial use, study, modification, sale, and redistribution while requiring distributed derivative versions to provide corresponding source under the GPL.

The official GPL text remains unmodified in `LICENSE`. For original project material whose copyright holder applies it, `ADDITIONAL_TERMS.md` adds a narrow author-attribution requirement permitted by GPLv3 Section 7(b). The required notice identifies Open Resume Toolkit and its canonical source repository. It must be preserved in a source-distribution notice and in an existing About, Credits, or legal-notices view. It does not prohibit commercial activity or otherwise narrow the GPL's granted freedoms.

`NOTICE` records project copyright attribution and the canonical repository. `TRADEMARKS.md` governs use of the project name and branding so that modified or third-party distributions do not imply official status. Trademark rules must not be used to prevent accurate attribution, discussion, compatibility statements, or the exercise of GPL rights in the code.

Before publication:

- Keep the unmodified official license text as `LICENSE`.
- Keep `NOTICE`, `ADDITIONAL_TERMS.md`, and `TRADEMARKS.md` public and internally consistent with application and website legal notices.
- Add SPDX identifiers, copyright notices, and an `ADDITIONAL_TERMS.md` applicability pointer to original source files where appropriate. Do not imply that the Section 7 term applies to third-party material or a contribution whose copyright holder has not accepted it.
- Audit all runtime, build, font, template, icon, and asset licenses for GPL compatibility and redistribution rights.
- Avoid proprietary bundled components that would make the published build non-reproducible or undermine SignPath eligibility.
- Document separately licensed assets and required attribution.
- Require the contribution process to state the inbound license clearly, including whether submitted material is provided under the applicable GPLv3 Section 7 attribution term. A Developer Certificate of Origin alone does not transfer contributor copyright.
- Obtain qualified review of the exact Section 7 term, contributor treatment, automated license identification, Store rules, and signing-program eligibility before the first stable release.

Using remote proprietary AI APIs or a user-authorized Codex subscription does not make those providers part of ORT, but bundled provider SDKs and the external Codex/app-server integration's licenses, trademarks, distribution terms, executable verification, authentication isolation, and client-identification requirements still require review. Bundling Codex later would require a new product, license, signing, SBOM, and update review.

## Contributions and governance

The repository should include:

- `README.md`
- `CONTRIBUTING.md`
- `CODE_OF_CONDUCT.md`
- `SECURITY.md`
- Privacy and code-signing policies
- Build and release documentation
- Issue and pull-request templates
- Maintainer, reviewer, and release-approver roles
- A dependency and license policy
- Public license, notice, additional-attribution, and trademark documents

Contributors certify that they have the right to submit their work. A Developer Certificate of Origin/sign-off workflow is preferred initially over a broad contributor license agreement unless legal advice identifies a specific need.

## Distributed documentation and claim boundaries

- Distributed application documentation must explain local storage, update checks, extension communication, AI-provider transmission, aggregate local AI Monitoring and estimate limitations, diagnostics, backup limits, and deletion behavior.
- Do not claim legal, ATS, hiring, accuracy, or security guarantees.
- Verify all third-party names, logos, templates, fonts, and icons before distribution.
- Review encryption export and software-distribution obligations applicable to official releases.
- Open source and free do not mean public domain or eliminate applicable software-distribution obligations.
- ORT does not need to collect a birth date or age category. It is not marketed as child-directed, and users of optional AI providers must satisfy the provider's own minimum-age, account, consent, and payment terms.
