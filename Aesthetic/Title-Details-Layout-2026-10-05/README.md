# Resume title and details spacing

The Edit resume title bubble uses its content width, and Skills/details follows it across a 7px gap. Neither resting field grows into unused row space. An open title/details editor has at least 180px of available width (bounded by its container) for the existing four formatting controls.

Checked the actual App component in a temporary local browser fixture with synthetic resume data, with no native persistence or provider calls. At 1080×760 and 720×520, all five sample subsection rows had exactly 7px between their title and details controls. Short titles matched text width plus the existing 2px border. At the minimum size, longer details wrapped without overflow, and the active editor kept all four formatting controls on one row. Temporary preview files and browser viewport override were removed after checking.

Validation: desktop TypeScript, desktop tests, web security, Prettier, production native build, and bundle signature verification. Installation receipt is recorded beside this report. The installed application is left closed.

## Separator follow-up

Added the same decorative `|` used in View between the Edit title and Skills/details, including when the Add skills/details affordance is shown. It uses the existing separator styling and is hidden from assistive technology. The existing 7px spacing now sits on each side of the separator. Browser checks at 1080×760 and 720×520 confirmed all five sample rows remained aligned without overflow. The follow-up installation receipt is `separator-installation.json`.
