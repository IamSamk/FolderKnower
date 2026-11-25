# M6: Live insight (memory, data structures, AI architecture)

Milestone 6 teaches what a program *is* (its data structures, its AI pipeline, its
clean path) and what it *does* while it runs (memory, cores, threads, KV cache). It is
written for a student first: every number on screen comes with one sentence saying
what it means, and every recognized pattern links back to the line that proved it.

Status key: **done** shipped on `feat/m6-live-insight`; **next** the following slice;
**later** designed, not scheduled.

| # | Doc | Status |
|---|-----|--------|
| 01 | [Goals and scope](01-goals-and-scope.md) | done |
| 02 | [Research synthesis and sources](02-research-synthesis.md) | done |
| 03 | [Live telemetry (unprivileged sampler)](03-live-telemetry.md) | done |
| 04 | [Process memory model for students](04-process-memory-model.md) | done |
| 05 | [Runtime tracing hooks](05-runtime-tracing.md) | next |
| 06 | [Model server introspection](06-model-server-introspection.md) | done |
| 07 | [OpenTelemetry GenAI and OpenInference ingest](07-otel-openinference-ingest.md) | next |
| 08 | [Static DSA detection](08-dsa-detection-static.md) | done |
| 09 | [Dynamic DSA detection](09-dsa-detection-dynamic.md) | later |
| 10 | [DSA animations](10-dsa-animations.md) | done |
| 11 | [Workflow: the ideal (clean) path](11-ideal-path-workflow.md) | done |
| 12 | [AI architecture recognition](12-ai-architecture-recognition.md) | done |
| 13 | [RAG visualization](13-rag-visualization.md) | done |
| 14 | [Agent loops](14-agent-loops.md) | done |
| 15 | [Transformer internals and KV cache](15-transformer-and-kv-cache.md) | done |
| 16 | [Memory techniques catalog](16-memory-techniques-catalog.md) | done |
| 17 | [Framework catalog and OKF](17-framework-catalog-and-okf.md) | done |
| 18 | [Scalable rendering for huge trees](18-scalable-rendering.md) | next |
| 19 | [node_modules, binaries and DLLs](19-binary-and-dependency-trees.md) | next |
| 20 | [Low-end hardware budget](20-low-end-hardware.md) | next |
| 21 | [Small decision models](21-small-decision-models.md) | later |
| 22 | [Domain packs: networks, security, IoT](22-domain-packs.md) | later |
| 23 | [Frontend and student UX](23-frontend-student-ux.md) | done |
| 24 | [Testing and goldens](24-testing-and-goldens.md) | done |
| 25 | [Phased roadmap](25-roadmap.md) | living |

Out of scope here: auto-update and code signing (owned by the release track).

## What shipped in the first slice

- `ark-patterns` crate: DSA, AI pipeline, memory-technique and library recognition from
  the parsed facts. Deterministic, rule ids on every finding, YAML catalog.
- `ark-parse` enrich pass: per-function identifiers, call arguments (arity, kwargs) and
  a control-flow outline for Python, JS/TS, Rust and Go.
- `ark-telemetry` crate: unprivileged system/process sampler, Linux memory breakdown,
  parsers for Ollama, llama-server and vLLM endpoints. Tauri commands in the desktop app.
- Desktop: **How it works** view (AI pipeline, data structure animations, memory and
  caching, libraries), **Workflow** drawer at every zoom level, **Live** view with a KV
  cache calculator, and an AI demo project (`fixtures/repos/rag-agent`).
