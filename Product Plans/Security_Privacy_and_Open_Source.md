# Security, privacy, and open-source governance

## Privacy promise

ORT may accurately describe itself as local-first when all canonical user content remains on the user's device. It must not imply that AI-assisted content never leaves the device.

The project must distinguish:

- Local editing, storage, search, rendering, backup, and export.
- Deliberate transmission to a user-selected AI provider.
- Update checks and browser-store communication.

Project maintainers cannot access local resumes, job descriptions, tracker entries, generated materials, or AI keys unless a user deliberately shares them outside the application.

## Protected assets

- Master draft, published master, imported content, application workspace, tracker, retained snapshots, backups, and exports.
- AI-provider credentials and provider account information.
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
- Credentials use the operating-system vault.
- Logs and diagnostic exports exclude content and secrets by default.
- Backups are encrypted and require a user-controlled passphrase.
- Clipboard use, temporary preview files, drag-and-drop exports, and recent-file behavior receive explicit implementation review because they can create copies outside ORT.

### Malicious imports and archives

- PDF, DOCX, backup, and other imported files are untrusted.
- Use operating-system quarantine, reputation, or malware-scanning facilities when available and appropriate, but never treat a missing alert as proof that a file is safe.
- Parse with memory/time/size limits and isolated libraries or processes where justified.
- Reject unsafe paths, archive traversal, excessive expansion, unsupported embedded content, malformed relationships, and executable payloads.
- Do not run document macros, scripts, external links, or embedded objects.

### Rendering, links, and structured snapshots

- Escape user and AI content before HTML, PDF, DOCX, and preview rendering.
- Allow only explicitly approved web/email link schemes; block script, data, local-file, executable, and unsafe custom schemes.
- Generated documents contain no hidden prompts, credentials, internal identifiers, temporary paths, or unnecessary metadata.
- Structured snapshots cannot select executable code, arbitrary local paths, unapproved templates, remote fonts, or unsafe renderer options.
- Validate nesting, strings, collections, decompression ratio, schema version, integrity checksum, and renderer compatibility before opening historical material.

### Update and supply-chain compromise

- Protect repository and Store accounts with MFA; use hardware-backed or phishing-resistant factors for release maintainers where possible.
- Restrict CI release permissions, pin or verify build actions/dependencies, review build-script changes carefully, and keep signing credentials outside the repository.
- Verify update signatures and channel identity before installation.
- Generate software bills of materials and provenance when the implementation toolchain supports them.
- Maintain an embargoed security-reporting path and a documented revocation/recovery procedure for compromised releases.

### Local IPC abuse

- Allowlist exact extensions and restrict IPC to the current OS user.
- The native host exposes a narrow typed protocol, not a general command or filesystem interface.
- Browser messages cannot select arbitrary local paths or invoke arbitrary binaries.

## Telemetry and diagnostics

- No resume, job, answer, credential, full URL, filename, or document content is collected automatically.
- Initial releases should operate without centralized telemetry.
- The application may generate a local diagnostic bundle that the user previews and deliberately attaches to a GitHub issue or support request.
- A future opt-in crash service requires a separate product decision, published data fields, retention, processor review, and a true off switch.

## Data deletion

There is no remote account-deletion workflow because ORT holds no product account or cloud copy. ORT provides local deletion and clear instructions for removing:

- Current application workspace
- Individual tracker artifacts or entries
- Provider credentials
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

Using remote proprietary AI APIs selected by the user does not make those providers part of ORT, but bundled provider SDK licenses and terms still require review.

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

- Distributed application documentation must explain local storage, update checks, extension communication, AI-provider transmission, diagnostics, backup limits, and deletion behavior.
- Do not claim legal, ATS, hiring, accuracy, or security guarantees.
- Verify all third-party names, logos, templates, fonts, and icons before distribution.
- Review encryption export and software-distribution obligations applicable to official releases.
- Open source and free do not mean public domain or eliminate applicable software-distribution obligations.
- ORT does not need to collect a birth date or age category. It is not marketed as child-directed, and users of optional AI providers must satisfy the provider's own minimum-age, account, consent, and payment terms.
