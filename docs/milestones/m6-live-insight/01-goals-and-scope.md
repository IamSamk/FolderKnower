# 01. Goals and scope

## The student we design for

A second-year CS student, or a prompt engineer who never took an OS course, opens a
repository they did not write. In ten minutes they should be able to say:

1. "This is an agentic RAG app: it chunks a PDF into ~800-character pieces, embeds them
   with bge-small, retrieves the top 8 from Chroma and from BM25, fuses them, reranks
   with a cross-encoder, and a LangGraph loop decides when to call the search tool."
2. "`balanced()` uses a stack; `bfs()` uses a queue and a visited set; `fib` is
   memoized recursion." And they have *watched* each structure move.
3. "The clean path through `load_user` is these 6 steps; these 3 red steps are where it
   can leave with an error, and which one fires depends on the input."
4. "While it runs, the model server holds 3.3 GB, 2.6 GB of it on the GPU, and its KV
   cache is 57% full; with 4 parallel requests at 8k context it would not fit in 16 GB."

## Requirements (from the request)

| Request | Where it lands |
|---|---|
| Visualize memory use across apps | 03, 04, Live view |
| Detect DSA (stack, queue, tree, heap, graph) with animated pictures, call stack | 08, 09, 10 |
| Workflow button at every zoom level, ideal path, errors unknowable statically | 11 |
| Agent loops, decision model picks tool | 12, 14 |
| Hybrid RAG: embedder, chunks, chunk count, top-k/top-n, reranker | 12, 13 |
| Transformer internals and KV cache | 15 |
| Memory techniques by real names (rolling cache, prompt caching) | 16 |
| Named frameworks and formats such as Google's OKF | 17 |
| Domain order ML → networks → security → IoT | 22, 25 |
| Live stats: cores, threads, process memory, KV cache | 03, 06 |
| Huge trees (node_modules, bin/DLL), low-end hardware, small models | 18, 19, 20, 21 |
| A frontend an average coder or student understands | 23 |

## Non-goals

- Being a profiler. We sample at 1.5 s and explain; we do not replace perf, Instruments
  or Nsight.
- Guessing runtime behaviour. Static views say "can", never "will"; runtime claims need
  runtime data (05, 07).
- Elevation by default. Everything in this milestone runs unprivileged
  (see `docs/security/telemetry-elevation.md`).
