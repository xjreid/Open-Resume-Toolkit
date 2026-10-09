# Codex readiness and signed-out overlay

October 9, 2026 · Operate mode · narrow refinement of Open Folio.

The Codex page checks the installed executable independently of runtime
permission and account status. Verified, disabled installations expose Enable
Codex without installation guidance or account controls. Missing or rejected
installations offer explicit installation and optional details. Loading and
failed checks do not assume installation is missing. An enabled preference can
still be disabled if the executable becomes unavailable.

The overlay retains its Codex provider label while signed out and replaces both
model selectors with “AI is disabled until an account is connected.” Usage stays
hidden. Controls return after connection; connected model/reasoning synchronization
and API-key selection remain covered by regression tests.

Headless fixture checks passed at 1080×760, 720×520 and 360×760. Reviewed captures
include verified/disabled readiness, missing installation, download, administrator
approval, retry, connected overlay and signed-out overlay. No horizontal overflow
or page errors were found. The teal rail, Hanken type, muted copy, existing control
styles and restrained borders remain intact. The new overlay message uses the
existing compact 13px body scale; detector type-ramp advisories for that size and
other incumbent overlay styles are intentionally preserved. Setup source scan
returned no findings. No live authentication, runtime installation or inference
was performed, and the installed ORT app was not launched.

Validation: 299 desktop UI tests, 43 contract tests, 142 native tests passed
(8 live/qualification tests ignored); TypeScript, Clippy with warnings denied,
Rust formatting, web-security, secret-file and whitespace checks passed.

Installation guidance is conditionally hidden, not removed; users regain it if
verification fails. This removes the previous confusion between a stopped
runtime and a missing executable without removing recovery controls.
