# Reviewed resume templates

The renderer compiles these fixed Typst templates from the application binary.
Resume content is supplied as JSON data only; templates must not read files,
import packages, load fonts, or evaluate resume text as Typst source.

- `technical_pdf_v1.typ` is a compact, single-column engineering layout.
- `professional_pdf_v1.typ` uses more space and a formal hierarchy.
- `modern_pdf_v1.typ` adds a restrained teal accent while preserving chronology.
- `plain_pdf_v1.typ` remains the historical compatibility presentation.

All three styled templates receive the same normalized content stream. They may
change presentation while preserving every factual value and section/entry order.
Dates share the role row; blank optional fields disappear. The frame audit permits
only text, links, and bounded thin horizontal rules from these bundled templates;
images, arbitrary shapes, clipping, and transforms remain rejected.

See [rendered review samples](../../Aesthetic/Resume-Designs/README.md).
