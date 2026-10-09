# Codex overlay controls finish review

## Disposition

Ship. The independent reviewer closed both findings: missing labels in earlier
captures were a text-paint timing issue, and renaming ChatGPT account OAuth copy
would exceed the requested ChatGPT plan title rename. Final busy and unknown
usage captures show the provider and reasoning labels.

## Scope

Preserves the existing Open Folio interface. API key and Codex provider labels
appear above model selectors. Codex supports model and reasoning selection,
known account-wide quota windows, and settings synchronization with the main
app. Unknown quota is omitted; active AI work locks selectors and retains Stop.
Reasoning-label clicks do not begin header dragging.

## Verification

- 285 desktop UI tests passed. The 66 affected overlay/connection tests were
  rerun after the final synchronization and pointer changes.
- 127 native tests passed; eight explicit runtime qualifications were ignored.
- 13 tailoring tests passed, including final-review issues and page overflow.
- TypeScript, production web build, Rust Clippy with warnings denied, formatting,
  security checks, and whitespace checks passed.
- Synthetic browser fixtures exercised model/reasoning selection and known,
  unknown, and busy overlay states, plus 1080x760 and 720x520 main-app layouts.

## Limits

No live account authentication or provider inference was performed. The main
app bundle was not installed or relaunched as part of this source-code request.

## Documentation

See codex-controls-documentation-check.md. Existing design authorities were
preserved; behavior references document the new provider controls and final
revision acceptance policy.
