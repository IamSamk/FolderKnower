# 25. Phased roadmap

Living document. Dates are targets, not promises.

## M6.1: Recognize and explain (done, 2026-11-25)

- [x] Enrich pass: identifiers, call arguments, control-flow outline (Py, JS/TS, Rust, Go)
- [x] `ark-patterns`: DSA, AI pipeline, memory techniques, libraries, OKF
- [x] `ark-telemetry`: sampler, Linux breakdown, model-server parsers, Tauri commands
- [x] How it works view, DSA animations, Workflow drawer, Live view, KV calculator
- [x] AI demo fixture and goldens

## M6.2: Precision and runtime truth (next, ~3 weeks)

- [ ] Lower typed map/set confidence; tighten recursion to `self`/`this` receivers
- [ ] py-spy / `profiling.sampling` attach → per-function CPU on the architecture view
- [ ] PEP 669 bootstrap → Workflow shows the path a run actually took
- [ ] OTLP receiver + OpenInference mapping → real chunk counts and reranker cuts
- [ ] Pin the analysed repo's process in Live; Windows and macOS memory names

## M6.3: Scale (~3 weeks)

- [ ] Level-at-a-time graph paging from SQLite; virtualization everywhere
- [ ] LOD and flatbush culling; WebGL renderer above 5k nodes
- [ ] Lockfile-first dependency view; native binary view (object/goblin)
- [ ] `architekton bench` and CI budgets on the reference machine

## M7: Networks pack (~4 weeks)

- [ ] Catalog and view (doc 22); live sockets per process

## M8: Security pack · M9: IoT pack

- [ ] As in doc 22

## Later

- Dynamic DSA (heap shape), small-model suggestions, GIF/WebM export of animations,
  multi-agent graphs, OKF view.
