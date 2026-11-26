# Milestone 7: the system story

Feedback after 0.4.0: on a real AI project (Docker containers, an agent loop, dense + sparse
hybrid search, a React frontend, a `memory.py`), the Architecture view read like a generic
"frontend talks to backend" overview. M7 makes the running system the first thing you see.

## What ships

| Piece | Where | What it does |
|---|---|---|
| Deployment map | `crates/ark-patterns/src/deployment.rs`, `ontology/services-catalog.yaml` | Reads `docker-compose*.yml`, `compose.*`, Dockerfiles and nginx configs. One service per container: kind (from the image catalog, or from the code in its build context), image, ports, `deploy.resources.limits.memory` / `mem_limit`, reservations, CPU, GPU (`gpus`, nvidia runtime, device reservations), volumes, env **names** (never values), entry file from the start command (`uvicorn app.main:app` → `app/main.py`). |
| Proven links | same | A link exists only when an env value names another service's host (`QDRANT_URL=http://vectors:6333`, `${VAR:-default}` included), `depends_on`/`links`, or an nginx `proxy_pass`. |
| Runtime memory notes | same | `--maxmemory` + policy (Redis), `OLLAMA_KEEP_ALIVE`, `OLLAMA_NUM_PARALLEL`, `--gpu-memory-utilization`, `--max-model-len`. |
| Hybrid search rules | `ontology/ai-catalog.yaml` | Sparse vectors (fastembed `SparseTextEmbedding`, SPLADE, Qdrant `SparseVectorParams`), Qdrant `Prefetch` + `FusionQuery(RRF)`, Weaviate `.hybrid(alpha=)`, Postgres `to_tsvector`/`ts_rank`. |
| Cache rules | same | Redis capped lists (`ltrim`), TTL caches (`setex`, `expire`), `functools.lru_cache`. |
| Example runs | `crates/ark-core/src/story.rs` | For every UI element whose handler reaches a backend route: screen → handler → proxy → route → helper files (memory.py) → tables → AI pipeline → backing containers → response. |
| Grounded tool | `fknow run <repo or architecture.json> [--from <ui id or label>] [--list] [--text]` | The same hops as JSON for an AI narrator. |
| System view | `apps/desktop/src/components/system/SystemView.tsx`, `lib/system.ts` | Containers in four lanes (browser, edge, your code, what it leans on), code inside each, memory split bar, camera-follow example run with a rendered screen preview. Falls back to code tiers without compose. |
| UI map | `apps/desktop/src/components/UiMap.tsx` | Screens on the left, "where it goes" on the right, a wire per element; hover lights the wire, "play run" opens the System view on it. |
| DSA tools | `apps/desktop/src/components/tools/ToolsDrawer.tsx`, `lib/dsaTools.ts` | Structures in your code; structures hidden inside your tools (HNSW, inverted index, top-k heap, agent state graph, ring buffer, LRU, KV block table, route trie, B-tree); real-life problems mapped to structures and to functions here; slow spots. |

## No hallucinations: how a hop is allowed

Each `RunHop` carries `proof` (`path:line`, a compose link like `REDIS_URL -> cache:6379`, or an
edge id) and a `confidence`:

| Hop | Allowed when | Confidence |
|---|---|---|
| UI, handler | element and handler resolved by the parser | 1.0 |
| client call chain | resolved calls (confidence ≥ 0.6) | 0.85 |
| proxy | both legs declared: frontend → proxy and proxy → backend | 0.9 |
| route | HTTP edge matched to a declared route | 1.0 |
| helper file | resolved calls from the handler | 0.85 |
| table | SQL edge from the handler or a called function | 0.85 |
| pipeline entry | the handler's file imports a file with AI stages (dynamic call) | 0.7 |
| pipeline stage | an AI pipeline edge from the previous stage | 0.9 |
| container | a declared link from the code's container to exactly one container of the matching kind | 0.8 |

If the chain breaks, the run stops. A model narrating a run gets only these hops.

### Tool definition for a model

```json
{
  "name": "example_run",
  "description": "Follow one UI element through the running system. Returns only hops proven by code or config; describe nothing else.",
  "input_schema": {
    "type": "object",
    "properties": {
      "from": { "type": "string", "description": "UI element id, part of it, or its label. Omit for the deepest run." }
    }
  }
}
```

Back it with `fknow run <architecture.json> --from "<from>"`. The output's JSON Schema is
`ExampleRun` in `schema/architecture.schema.json`.

## Research behind it

- Compose specification: services, `deploy.resources` (limits, reservations, devices with
  `capabilities: [gpu]`), `gpus`, `mem_limit`, networks aliases, `depends_on`.
- Qdrant hybrid queries: dense + sparse named vectors, `prefetch` + `Fusion.RRF`; fastembed
  `SparseTextEmbedding` (SPLADE, BM25, BM42).
- Weaviate hybrid search (`alpha` blends BM25 and vector scores); Postgres full-text
  (`tsvector`, GIN, `ts_rank`) beside pgvector.
- Code2DFD (Schneider et al., TUHH, arXiv 2412.08352): dataflow diagrams recovered from
  microservice code plus compose files. This is the closest prior work to proven links.
- Infragenie (ECSA 2025): living architecture diagrams generated from infrastructure files.
- vLLM PagedAttention and prefix caching; Redis `maxmemory-policy`; Ollama
  `OLLAMA_NUM_PARALLEL` / `OLLAMA_KEEP_ALIVE`.

## Next

- Kubernetes manifests and Helm values (same `Service` model).
- Service links from client constructors (`QdrantClient(url=...)`) when the URL is a literal.
- Logo and installer wizard (low priority).
