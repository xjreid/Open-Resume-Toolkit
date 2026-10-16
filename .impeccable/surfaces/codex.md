# Main application / AI & monitoring / ChatGPT

Mode: Operate. Scope: the main-window Codex page only. Existing runtime,
authentication, installer, model, and quota contracts remain authoritative.

## Direction contract

THESIS: The user's runtime-first composition makes connection state immediately
legible, then reveals account controls only after login.

OWN-WORLD: Inherit Open Folio: Paper #ffffff, Canvas Mist #f2f6f7, Ink #1c3135,
Folio Teal #28676c, Success Moss #2d624f, Danger Red #a13b3b. Hanken typography,
40px controls, 12px panel corners, restrained rules. Reuse the sourced OpenAI mark.

STORY: Check installation, launch, log in with OpenAI, choose model/reasoning,
read remaining quota, and optionally reserve usage. Quit and sign out stay distinct.

FIRST VIEWPORT: A separate sticky header has the OpenAI/ChatGPT lockup at left,
installation state below, runtime status/refresh/launch-or-quit at right (Quit has no icon).
Below it, a centered large white login button is replaced by model controls and
an account section. Account plan and sign-out lead; quota meters precede reserve.
At small window sizes the controls wrap, and usage/reserve stack.

FORM: Precisely specified local replacement within an established visual system;
user-pinned composition, no concept roll. Code-led implementation of the brief.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

Layout:

    [ OpenAI mark + ChatGPT                 • Running  ↻  Quit ]
    [ ✓ Codex Runtime is Installed                            ]

    [ Model                              Reasoning            ]

    [ Account / Plus plan                           Sign out  ]
    [ 5-hour limit + weekly limit        Usage reserve         ]

Review against the brief: the header is the one distinctive brand gesture;
the remaining sections use familiar controls and only task-relevant copy.
No unrelated shell, overlay, API-key, or backend changes.

User follow-up: the navigation tabs are My Keys, ChatGPT, and Data. Remove
the Codex subtitle from the header; ChatGPT is the heading. Quit is text-only.

All sections fill the same 960px maximum-width track, bounded by the window.
Their width is independent of authentication and reserve state. Reserve scrollbar
space so tall account content does not change the available width. Launch is text-only.
