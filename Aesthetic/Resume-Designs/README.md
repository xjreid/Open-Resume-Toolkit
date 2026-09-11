# Professional resume designs — M2.5

These are outputs from the application's actual local PDF and DOCX renderers,
using the same synthetic source. They contain no real employment history.

| Style | PDF | Word | Direction |
| --- | --- | --- | --- |
| Technical / Engineering | [PDF](technical.pdf) | [DOCX](technical.docx) | Centered name, compact serif typography, section rules, aligned dates |
| Professional / Business | [PDF](professional.pdf) | [DOCX](professional.docx) | Left-aligned name, more breathing room, formal heading hierarchy |
| Modern / Marketing and Sales | [PDF](modern.pdf) | [DOCX](modern.docx) | Larger name, restrained teal accent, clear chronology |

All three representative PDFs have one page. The [long Technical PDF](technical-long.pdf)
and [Word sample](technical-long.docx) have two pages. PDF and Word pages were
rendered and visually inspected for clipping, headings, alignment, and legibility.
Word uses a reader-provided Times New Roman face; PDF embeds Libertinus Serif.
Normal reader-specific font and pagination differences remain possible.

The Technical layout is an independent implementation of common professional
resume conventions. No Jake's Resume source or assets were copied. Documents
are independent of ORT branding and application-theme tokens.

Regenerate this small review set with:

```sh
CARGO_BUILD_JOBS=1 cargo run --locked -p ort-render --example m25_designs -- Aesthetic/Resume-Designs
```

The example never opens a profile, credential vault, or user-supplied resume.
