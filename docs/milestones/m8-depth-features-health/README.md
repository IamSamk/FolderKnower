# Milestone 8: depth, features, memory map, broken code

Feedback after 0.5.0: big projects need more than one diagram. People want a plain
overview and a deep technical one, a diagram per feature, a memory diagram that says
exactly what is cached where (vector DB? VRAM or RAM?), and the app has to make sense
of code that does not run yet.

## What ships

| Piece | Where | What it does |
|---|---|---|
| Features | `crates/ark-core/src/features.rs` | One feature per route family (`/api/v1/todos/:id` → `todos`; auth-like names merge into "Sign in"; health/docs/metrics into "Health checks"). Only emitted when there are at least two. |
| Memory map | `crates/ark-core/src/memory.rs` | `memoryMap`: one placement per kind of runtime data with place (VRAM, RAM, disk, browser, remote), holder (container, code tier, browser, provider), lifetime, bound and proof. |
| Code health | `crates/ark-core/src/health.rs`, `ark-parse` (syntax error locations), `ark-link` (unresolved imports, dead calls) | `health`: syntax errors, relative imports to nowhere, packages missing from manifests, same-origin calls to undeclared routes, start commands naming missing files, GPU code without a GPU. |
| Per-file packages | `SourceFileNode.packages` | Third-party imports per file (stdlib left out), used for each box's tech stack. |
| CLI | `fknow check <repo> [--json]` | Prints the problems with a fix each; exits 1 on any error. |
| Depth toggle | `components/system/SystemView.tsx`, `lib/system.ts` | Simple / Standard / Deep, remembered per machine. |
| Feature filter | same | Fades boxes and links outside the feature, narrows code rows, AI steps, memory rows and example runs. |
| Memory diagram | `components/system/MemoryMap.tsx` | Columns per place, grouped by holder, with limits, GPU flag, lifetime pills and proof links. |
| Problems drawer | `components/tools/HealthDrawer.tsx` | Grouped by kind, with why it matters and a one-line fix; click opens the file. |
| Broken fixture | `fixtures/repos/broken-shop` | One of each problem; also the "Try a broken project" demo. |

## How a feature is assembled (nothing by name similarity)

1. Seeds: route handlers of the family.
2. Backend: resolved calls from the handlers, depth 4.
3. Dynamic entries: a call on an imported object (`agent.invoke(...)` with
   `from .agent import app as agent`) enters that module when the import resolves to one of
   the file's import edges with the same module stem. Its AI stages join the feature, and the
   pipeline is followed forward from them, never back into indexing (loader, splitter,
   embedder, vector store, keyword index).
4. Frontend: client functions with an HTTP edge to a handler; UI elements whose handler
   reaches one within 3 calls; inputs on the same screen component.
5. Tables: SQL/ORM edges from the feature's functions.
6. Containers: build contexts of its files, plus the single database / vector store /
   model server / cache when the feature has tables / retrieval / an LLM / Redis memory.
7. A file reached from two features is `sharedFiles`, not owned.

## How a memory placement is allowed

| Fact | Placement |
|---|---|
| LLM stage on Ollama + one model-server container | weights and KV cache in that container; VRAM if it reserves a GPU, else RAM |
| vLLM `LLM(...)` | weights, KV cache, prefix cache in VRAM of the container that builds the file |
| Local embedder / reranker (sentence-transformers, HF) | RAM, or VRAM with `device="cuda"` or a GPU container |
| Hosted model / embeddings class (ChatOpenAI, OpenAIEmbeddings ...) | remote, per request |
| Vector store stage + matching vector container | index in that container's RAM; collections on disk when a volume is mounted (otherwise lifetime is "until restart", with a warning in the text) |
| `persist_directory=` | index files on disk in the process that writes them |
| BM25 retriever | RAM of the process, rebuilt on restart |
| Redis `ltrim` / `setex` + one Redis container | RAM in Redis, per session, bound by the trim, `--maxmemory`, policy and container limit |
| Conversation memory / LLM cache classes | RAM, Redis, SQLite, Postgres or remote by class name |
| Database container | tables on disk |
| `localStorage.setItem`, `sessionStorage.setItem`, `indexedDB.open`, zustand `persist` | browser, with the key when it is a literal |

Placements for the same thing in the same holder merge their proof; GPU memory wins over RAM.

## Health checks and false positives

- Package checks run only when the language has a manifest above the file. Standard library,
  Node built-ins, local module names, tsconfig path aliases, import/distribution name
  differences (`yaml` → PyYAML, `cv2` → opencv-python ...) and packages a declared one brings
  along (`fastapi` → pydantic) are not flagged.
- Dead routes are only reported for same-origin URLs (`/api/...`) and only when the repository
  declares routes at all.
- Tests: `broken_code_is_reported_and_still_mapped` asserts all seven problems in broken-shop and
  no others; `healthy_fixtures_have_no_false_alarms` asserts fullstack-todo has none and
  rag-agent has exactly the real one (vLLM in a container without a GPU).
