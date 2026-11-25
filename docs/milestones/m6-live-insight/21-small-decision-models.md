# 21. Small decision models

**Status: later.** The static core is deterministic; models only help where rules run
out, and every model answer is labelled as such.

## Where a small model earns its place

| Decision | Today (rules) | With a small model |
|---|---|---|
| Ambiguous tier for a folder | ask the student ("I'm 30% sure …") | Qwen3-0.6B ranks the options from file names and imports; the student still confirms |
| Unknown class that looks like a RAG stage | not shown | an embedding similarity against catalog explanations proposes a role at low confidence, shown with a "guess" chip |
| Explain a function in plain words | built-in narrator (Qwen2.5-0.5B) | unchanged; grounded on the facts we extracted |
| Name the algorithm in a function | DSA rules | a classifier over the control-flow outline plus identifiers |

## Rules for using models

- Local only. Under 1 GB at Q4, CPU-friendly, so they run on doc 20's reference machine.
- Never overwrite a rule-based fact; models add suggestions with a confidence and a
  visible "model guess" label.
- Deterministic decoding (temperature 0) and cached by input hash, so the same repo gives
  the same suggestions.
