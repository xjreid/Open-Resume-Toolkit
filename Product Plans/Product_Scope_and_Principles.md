# Product scope and principles

## Product identity

**Open Resume Toolkit (ORT)** is a free, open-source, local-first desktop tool for preparing resumes and managing a person's own job applications. It combines a structured resume editor, optional user-authorized AI assistance through a personal API key or eligible ChatGPT/Codex subscription with local usage visibility and guardrails, required-qualification comparison against the published resume, local document rendering, an application tracker, a desktop overlay, and narrow Chrome/Edge capture extensions.

ORT is a tool, not an employment service, employer product, job board, or automatic application agent. It must not claim to guarantee interviews, employment, applicant-tracking-system performance, or legal compliance.

## Intended audience and initial platforms

- Individual job seekers.
- Windows and macOS desktop computers.
- Chrome and Microsoft Edge companion extensions.
- English initial interface and document tooling.
- Public source code and releases distributed through GitHub and supported stores.

The software may be used internationally, but documentation must not imply that every template, generated statement, export, or workflow satisfies every jurisdiction or employer requirement.

## Product surfaces

1. **Desktop application** — owns user data, resume editing, application workspaces, AI-provider configuration and activity history, rendering, exports, tracking, backup, restoration, updates, and the primary overlay.
2. **Desktop overlay** — a compact always-available view for the current application workflow. It is part of the desktop app, not browser-injected product UI.
3. **Chrome and Edge extensions** — capture user-selected job descriptions or application questions and send them to the local desktop app after review.
4. **GitHub repository** — canonical public source, issue tracker, release history, contribution process, security policy, and build provenance.

## Core product principles

### Local ownership

- Resume, application, generated, and configuration data are stored on the user's device.
- ORT does not require an ORT account or centrally operated backend.
- No cloud synchronization or recovery is implied. Backup and device migration are explicit user-controlled operations.

### Desktop-first experience

- The desktop application is a full application with an editable document surface, tracker, and real overlay.
- The extensions stay small and permission-constrained. They do not store the resume database, hold AI credentials, render documents, or make AI calls.

### User control over AI

- Manual editing, tracking, rendering, and export work without an AI provider.
- The user selects either a supported personal provider API key or an eligible personal ChatGPT/Codex subscription. ORT never supplies or resells inference.
- Direct API and Codex subscription modes are mutually exclusive, and provider/model changes never trigger a silent fallback.
- Before transmission, ORT identifies the selected provider and the content to be sent.
- AI output is always a proposal requiring review. It never silently replaces the master resume or submits an application.
- ORT makes its own AI calls visible through a local activity history and distinguishes estimated charges from the provider's authoritative bill.
- Optional direct-API estimated-spend caps and Codex provider-quota thresholds block future ORT calls when their measurable limit is reached, while clearly stating their local and provider-reporting limitations.

### Structured, portable documents

- Structured versioned data is the editable source of truth.
- Templates control presentation without owning or discarding content.
- PDF, DOCX, and plain text are reproducible local exports, not the only editable copy.
- Users can export their complete ORT data in a documented portable format.

### Free and non-gated

- All implemented capabilities are available to every user.
- ORT has no feature tiers, subscriptions, advertisements, artificial application quotas, or centrally sold AI credits.

### Honest and safe assistance

- ORT does not invent qualifications, employers, dates, achievements, credentials, metrics, or personal facts.
- It refuses to draft legal or personal attestations that the user must answer themselves.
- It treats captured web content as untrusted data and makes material AI changes visible.

## Initial non-goals

- Accounts, subscription billing, hosted user storage, cross-device sync, or web-based resume editing.
- Automatic application submission, automatic form completion, or continuous browsing monitoring.
- Multiple named master resumes or an unlimited career-profile database.
- Permanent storage of every intermediate AI draft.
- Employer/recruiter features, applicant ranking, or hiring decisions.
- Universal ATS scoring or employment predictions.
- Firefox, Safari, Linux, or mobile support at initial launch.
- OCR for scanned resumes, local-model bundles, job-board notifications, reminders, calendars, or direct job-board integrations at initial launch.

## Canonical terms

- **Local profile:** the local data boundary for one person's ORT records on an operating-system account. It is not an online account.
- **Master-resume draft:** the one autosaved editable master resume.
- **Published master resume:** the deliberate snapshot used as the factual source for AI tailoring.
- **Application workspace:** the one current job-specific working set containing captured context and temporary generated material.
- **Application tracker:** the local spreadsheet-like history of jobs and deliberately retained structured materials.
- **Structured snapshot:** an immutable saved JSON representation of a selected tailored resume, cover letter, or answer set.
- **Derived export:** a locally generated PDF, DOCX, or text file that is outside ORT's canonical database once saved.
- **Provider adapter:** local code that translates an ORT request into a user-selected AI provider's API and validates the response.
- **AI connection mode:** the single active choice of no AI, a direct provider API credential, or a managed ChatGPT/Codex subscription connection.
- **AI activity ledger:** durable, content-free local history of logical AI operations and their provider-call attempts, usage measurements, statuses, and contemporaneous cost estimates.
- **AI guardrail state:** durable local counters, reservations, period policies, and Codex quota thresholds used to block future ORT calls independently of activity-history retention.
- **Required Qualification Alert:** a temporary, non-blocking comparison result for an explicit mandatory job qualification that either conflicts with an explicit published-resume fact or was not found in the published master. It is not an eligibility decision, fit score, or statement about information outside the resume.
