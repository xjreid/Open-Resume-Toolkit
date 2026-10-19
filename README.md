# Open Resume Toolkit

A free, open-source, local-first desktop application for maintaining a master
resume and preparing application materials. The desktop app owns editing, AI,
tracking, and exports; the Chrome extension captures job text only when you ask.
ORT operates no account service, hosted resume store, subscription, or funded AI
service.

## Current state

**Active development target: macOS on Apple Silicon.** This is a development
build, not a production-qualified stable release. Windows and Intel Mac CI jobs
check portability; they do not establish platform support.

The current app includes:

- A structured master-resume editor, reviewed local PDF/DOCX import, immutable
  publications, local PDF preview, and PDF/DOCX/text export. Technical,
  Professional, and Modern styles keep presentation separate from content.
- An application overlay for reviewing job descriptions, tailoring the published
  resume, generating cover letters and answers, editing results, reviewing
  qualification alerts, and downloading or dragging out final documents.
- Multiple saved OpenAI, Anthropic, or Gemini API keys with one active key,
  explicit model selection, local usage history, estimated spending, and
  optional spending caps.
- Optional Codex through a separately installed, verified official runtime.
  Launch explicitly and sign in with your ChatGPT account.
  Authentication lasts only in that process's memory; stopping or quitting ends
  the session. ORT never silently switches to an API key.
- A local application tracker with retained materials, inline metadata editing,
  search, status filters, paginated results, and visible-page CSV export.
- Chrome 0.3.0 capture and an authenticated macOS development bridge. Edge is
  still a capture-disabled scaffold. Production host identity and Store
  distribution qualification remain release work.
- SQLCipher-encrypted profiles with OS-vault database/API keys, exclusive profile
  ownership, passphrase-encrypted backups, restart-staged restore and rollback,
  and retryable deletion of active and retained profile credentials.

M0–M4 have recorded development acceptance. M5 remains open for browser delivery
and Edge qualification. Much of M6 Codex is implemented and locally tested; full
milestone acceptance is not recorded. M7 production signing, notarization,
release/update infrastructure, and final Store qualification remain planned.
See the [current implementation map](docs/development/current-state.md) and
[delivery roadmap](<Implementation Plans/System Documentation/Delivery_Roadmap.md>).

M5 also includes the approved, **not yet implemented**
[Application-answer autofill and reusable library](<Product Plans/Application_Answer_Autofill.md>).
Its [technical plan](<Implementation Plans/Browser Extensions/Application_Answer_Autofill_Plan.md>)
adds opt-in foreground learning/filling, a desktop library and extension popup,
with approval, Undo, and privacy controls. The current app/extension behavior
listed above has not changed.

## Develop and verify

Start with [DEVELOPMENT.md](DEVELOPMENT.md) for pinned prerequisites, local builds,
synthetic profiles, and checks. The [CI guide](docs/development/ci.md) maps the
checked-in workflows, test coverage, and platform limits. Complete development
packaging is separate from a bare frontend/Tauri build because import and Codex
installation require verified native helpers.

Developers changing a workflow should maintain its
[application behavior reference](docs/application-behavior/README.md) and the
relevant product and implementation plans.

## Documentation

- [Plan index](Plan_Index.md): document authority, reading order, and decisions.
- [Product scope](<Product Plans/Product_Scope_and_Principles.md>) and
  [core workflows](<Product Plans/Core_Workflows.md>): approved product behavior.
- [Application behavior](docs/application-behavior/README.md): how implemented
  workflows behave, including persistence, failures, and cleanup.
- [Implementation plans](<Implementation Plans/README.md>): architecture and
  component design; unfinished work is identified explicitly.
- [Open decisions and release work](<Product Plans/Release_Scope_and_Open_Decisions.md>).
- [Chrome development setup](packaging/extension/chrome/development-testing.md)
  and [Codex runtime guide](docs/ai/codex-plan-runtime.md).

Dated evidence, reviews, ADRs, and archived plans describe their recorded
snapshot. They are not current setup instructions or fresh test results.

## License and community

ORT uses GNU GPL version 3 only (`GPL-3.0-only`); commercial use, modification,
and redistribution are permitted subject to the GPL. See [LICENSE](LICENSE),
[NOTICE](NOTICE), the [attribution term](ADDITIONAL_TERMS.md), and
[trademark policy](TRADEMARKS.md).

The [canonical source repository](https://github.com/xjreid/Open-Resume-Toolkit)
is the public home for source, documentation, support, and release information.
Development ad-hoc signatures do not establish an official production publisher.

Read the [contribution guidelines](.github/CONTRIBUTING.md) and
[Code of Conduct](.github/CODE_OF_CONDUCT.md). Report vulnerabilities through the
[private security process](.github/SECURITY.md).
