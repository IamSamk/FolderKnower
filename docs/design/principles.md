# Design principles (soft neo-brutalism)

Rules every UI change must follow.

- Nested rounded corners: inner radius = outer radius minus padding.
- Borders that blend (alpha, for example `rgb(0 0 0 / 0.12)`) for quiet lines; the solid ink border stays only on cards and buttons, as the style needs.
- Palettes in OKLCH, not HSL. Gradients use `in oklch`.
- Icon stroke width matches the weight of the text next to it.
- Carousels fade at the edges: `mask: linear-gradient(90deg, transparent, black 15% 85%, transparent)`.
- Icon swaps: blur the outgoing icon while it fades (`filter: blur(4px)`).
- Dark mode shows depth with lighter surfaces, not shadows.
- Full-height sections use `100dvh`.
- Product shots on white: `mix-blend-mode: multiply`.
- No purple gradient, radial glows, gradient text, extra badges, or emoji. Two calls to action at most.
- One accent colour. 4-point spacing. A clear type hierarchy and visual priority. Scannable cards. Show what can be clicked.
- Glass effects only where they help. Premium assets (React Bits, v0, Uiverse and similar) only when they fit the style.
- Write text in Simplified Technical English (see the skill).

## Ideas from gitingest and gitdiagram

- Same-address entry: change one word in a GitHub URL to open it. Plan: `fknow` and the site accept a repo URL.
- Show stats first (files, size, token estimate) and let people scope to one folder.
- Click a node to jump to its file; check every link against the real file tree (we already do this: each box carries `path:line`).
- Stream the overview while the diagram builds; cache results so reopening is instant (we cache parsed results).
- Export: a copy-ready digest for a prompt, and PNG or Mermaid export of a diagram.
