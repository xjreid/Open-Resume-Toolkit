# UI corrections — October 4, 2026

The user requested sticky navigation without a top gap, precise placement of entry fields, consistent empty-date styling, and clearer inline editing controls. Existing Open Folio identity, resume data, autosave, formatting, and document exports are preserved.

- Edit/View/Import and My Keys/Data stick to `top: 0` within the app content scroll area at default and minimum widths. The AI development preview has a 61px banner outside that scroll area; both its tabs and scroll area start at the same coordinate.
- Role is beneath Title; Skills/details is beside Title; Date, Location, and Extra form a right-aligned stack. Empty Date uses the same Hanken/muted treatment as the other add controls.
- The inline editor keeps a visible field label, Bold/Italic/Link controls, a Text layout selector for the existing bullet/paragraph modes, an explicit Done action, and separate Clear text. Link validation and Apply/Cancel are retained. Escape closes; Ctrl/Command B and I use the existing formatting model. Done closes without reverting autosaved edits and returns keyboard focus to the field.
- The active editor expands when needed and scrolls into view with space for the sticky toolbar.

## Evidence

[Corrected scrolled editor](after-editor-scrolled.png), [inline information editor](after-information-editor.png), [minimum-width focused editor](after-minimum-information-editor.png), [AI Keys](after-ai-keys-scrolled.png), and [AI Data](after-ai-data-scrolled.png).

The before/after captures use actual React components in local Chrome with synthetic fixtures. They do not open the installed native app or access a real profile. Each PNG carries embedded provenance. The scripts and geometry are retained for reproducibility.

237 desktop tests in 39 files, TypeScript, the production web build, and web-security checks pass. Eight grouped browser interaction checks pass with no page errors. [The independent layout review](layout-review.md) resolves both scoped residual findings. The installed development app was replaced after signature and all 25 bundle-file checks passed. The previous bundle is preserved for rollback. See [the installation receipt](installation.json). The app was not launched, and the profile and credentials were not modified.
