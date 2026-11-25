# 13. RAG visualization

**Status: done.** `components/insights/AiPipeline.tsx`.

## Layout

Four lanes tell the story left to right. Empty lanes are dropped.

| Lane | Title | Roles |
|---|---|---|
| 0 | Ingest: once, offline | loader, splitter, embedder, vector store, keyword index |
| 1 | Retrieve: every question | retriever, fusion, reranker |
| 2 | Generate and decide | prompt, memory, cache, guardrail, LLM, router, agent, tool, evaluator |
| 3 | Inside the model | model server, tokenizer, neural layers, trainer |

Cards show the role, the class name as written, and up to two literal settings
("chunk size 800", "top-k 8"). The layout is deterministic (no force layout) so the same
repo always looks the same.

## Key numbers strip

These are pulled from all stages and shown above the diagram:

- chunk size and overlap;
- embedding model;
- top-k per retriever and fusion weights;
- reranker model and top-n;
- LLM model and context window.

**How many chunks?** The real count needs the documents. We show an estimate with the
formula visible: for a P-page document at ~1,800 characters a page,
`chunks ≈ ⌈(chars − overlap) / (size − overlap)⌉`. P is an input, so a student can try
their own book. Doc 07 replaces the estimate with the traced count.

## Play a request

A tour orders the stages topologically, ignoring loop-back edges and breaking cycles at
the earliest lane. A packet travels along each edge (CSS `offset-path`). The caption
reads "Embed receives chunks. Turns each chunk into a vector …". Stages not yet reached
are dimmed.

## Detail panel

Clicking a card shows:

- the full explanation;
- every setting found in the code (with human labels);
- the rule id;
- "Show me the code, line N", which jumps to the function in the Architecture view.
