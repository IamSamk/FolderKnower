# 23. Frontend and student UX

**Status: done** for this slice. This is the hard requirement: the backend is useless if
a student cannot read the screen.

## Rules every view follows

1. **One sentence per number.** No metric without a plain explanation next to it
   (KV cache, PSS, top-k, overlap).
2. **Show, then tell.** An animation or a picture first (DSA, packet tour, token strip),
   the definition second, the evidence third.
3. **Always link back to code.** Every card has "line N →", which jumps to the
   function through the same reveal used across the app.
4. **Say how sure we are.** Confidence shows as "85% sure"; textbook animations say
   "textbook example"; simulated data says "Simulated numbers".
5. **Use the words of the field.** PagedAttention, prefix caching, attention sinks,
   BM25, reciprocal rank fusion: real names, each explained once.
6. **Deterministic layout.** The same repo looks the same every time, so students can
   share screenshots and teachers can prepare.

## New surfaces

| Surface | Where |
|---|---|
| **How it works** tab | AI pipeline · Data structures · Memory and caching · Libraries |
| **Workflow** button | top bar, every zoom level and every view except Live; Esc closes |
| **Live** tab | Memory, cores, processes with trends, one process explained, model servers, "Will a model fit?" |
| Inspector, "How it runs" | DSA chips for the function and "Show the clean path through this function" |
| Welcome | "Try the AI demo: RAG + agent + tiny GPT" (works in the browser too) |

## Visual language

Soft neo-brutalism, unchanged:

- ink outlines and offset shadows;
- one typeface for text and JetBrains Mono for code;
- role colours from `ROLE_COLOR` (AI) and tier colours (architecture);
- state colours, used everywhere: yellow is current, green is done or on the clean
  path, blue is next or frontier, red is an error exit.

## Accessibility

- All controls are buttons with titles.
- Colour is never the only signal: every state also has a label ("error exit",
  "decision").
- Keyboard: Esc closes the drawer, and the tabs are focusable.
- Next: reduced-motion switch and a screen-reader summary per animation frame.
