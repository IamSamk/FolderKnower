# 24. Testing and goldens

**Status: done.**

| Layer | Tests |
|---|---|
| `ark-parse` enrich | kwargs, idents and flow for Python; JS option objects and flow; Rust `?` and Go `if err != nil` flow |
| `ark-patterns` | catalog loads; stack, queue, heap, recursion, graph; class stack and tree; hybrid RAG params; LangGraph agent loop and Anthropic prompt cache; vLLM prefix caching and torch attention; JS top-k |
| `ark-core` goldens | `fixtures/expected/fullstack-todo.json`, `fixtures/expected/rag-agent.json`. Regenerate with `UPDATE_GOLDEN=1 cargo test -p ark-core` and review the diff |
| `ark-telemetry` | samples its own process (memory > 0, tree ≥ self, Linux breakdown present, threads ≥ 1); Prometheus, Ollama, vLLM and llama-server parsers on recorded payloads |
| Desktop | `tsc --noEmit`, `vite build`, `biome ci .`; a manual Playwright screenshot pass over each new view on the AI demo (automating it is M6.2) |

## The rag-agent fixture

`fixtures/repos/rag-agent` is written to hit every recognizer once:

- ingest (PDF loader, splitter 800/120, bge-small, Chroma, BM25);
- retrieve (k = 8, ensemble 0.6/0.4, cross-encoder, top-n 3);
- agent (ChatOllama qwen3:4b with num_ctx and keep_alive, a `@tool`, StateGraph with
  conditional edges);
- serving (vLLM with prefix caching and fp8 KV);
- `model/tiny_gpt.py` (attention, MLP, block);
- `algorithms/structures.py` (stack, BFS queue, heap, memoized fib, binary search,
  tree);
- an OKF `knowledge/` bundle.

Any change to a rule shows up as a reviewed golden diff.
