# M3 native walkthrough

Date: September 16, 2026. Scope: macOS Apple Silicon development, locally
signed current-source candidate; no notarization or wider native-platform claim.

Final bundle: `target/m3-final-20260916/Open Resume Toolkit Dev.app`.
Its package manifest records desktop SHA-256
`860e1eda51adfa65cf306fd4a40b5d7282c23bae87723524a6fb9401c4b20cba`,
parser executable SHA-256
`6754da9b8a3471eafbb77fbeac57b433cba77a0dc3b566e691600fb97ec40152`,
and pinned parser CDHash `9eec219ef5d1c142ba1c17bc4c56b95b1e919266`.
The outer app uses the existing ORT Local Test Signing certificate. Deep/strict
signature verification and nested helper hash verification passed during packaging.

## Observed runtime evidence

- Main and companion windows report **Encrypted storage ready**.
- Existing saved resume data renders in the original editor, with its section
  navigation, Technical style, publication control, and saved status intact.
  No resume content was edited, published, removed, or exported during this check.
- View retains saved/published version selection and PDF/Word export controls.
- Import is enabled in the complete candidate; its native PDF/DOCX Open dialog
  opens and Cancel returns cleanly without importing or changing data.
- The final bundled helper also passed the native `parser_helper_smoke` example
  with synthetic PDF/DOCX extraction, wrong running-code identity rejection,
  mid-job cancellation, and verified child reaping. The harness requires an
  absolute helper path; an initial relative-path invocation was correctly refused.
- Settings retains backup/recovery and storage/deletion routes. The final backup
  disclosure includes content-free AI activity/pricing provenance and excludes
  keys, credentials, and active spending-cap authority.
- AI & monitoring renders in the final signed app with No AI, an empty secure
  key field, disabled credential-dependent dispatch/cap controls, the signed
  `2026-09-15.1` entry, ORT limits, price completeness, and provider applicability.
- Provider selector changes the displayed signed OpenAI, Anthropic, and Gemini
  model/rate entries without saving a connection or making a provider request.
- Week and All time controls change the authoritative empty monitoring period.
  Aggregate export opens the native Save dialog with the fixed JSON filename;
  Cancel leaves no export file and returns to monitoring.
- The current native AI UI was visually inspected at the normal window size;
  navigation and catalog content render without clipping. Component/axe tests
  separately cover keyboard accessibility, streaming, cancellation, failures,
  missing usage, cap baseline reload, and aggregate consistency.

The generic M0 qualification bundle initially omitted the parser helper and
therefore disabled import. It is not the M3 handoff artifact. The complete
candidate was assembled using the existing pinned-helper packaging path:
`python3 tools/package-m2-macos.py --identity <existing fingerprint> --output target/m3-final-20260916`.
Parallel copies of the same bundle identifier also caused a transient blank
main window; this did not recur after closing the earlier qualification copy
and cleanly launching the complete candidate. Normal menu Quit exited each
prior candidate successfully.

No real or fabricated API key was entered, and no provider call was made.
The roadmap explicitly permits documenting missing live-provider evidence
when no provider credential is configured. Credential-vault round trips and
provider transport/accounting behavior have separate automated evidence.
The installed application was not replaced.
