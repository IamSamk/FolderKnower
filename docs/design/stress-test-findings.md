# Stress test: large modern repositories (M13)

Goal: run FolderKnower (`fknow index` / `fknow export`, v0.14.1 baseline) on big, current,
architecturally busy open-source repos and compare what it finds against what the
maintainers ship as ground truth (compose files, READMEs, docs). Every miss is logged here;
fixes land in the same milestone.

Shallow clones taken 2026-11-27. Index time on a 4-core cloud box, no cache.

| Repo | Stack | Source files | Index time | architecture.json (pretty) |
|---|---|---:|---:|---:|
| GoogleCloudPlatform/microservices-demo (Online Boutique) | Go, C#, Java, Node, Python, gRPC | 48 parsed of ~60 | 0.2 s | 3 MB |
| meilisearch/meilisearch | Rust workspace (actix, LMDB) | 725 | 1.7 s | 22 MB |
| langfuse/langfuse | TS monorepo (Next.js, tRPC, BullMQ) + Rust gateway | 7,127 | 11.6 s | 101 MB |
| onyx-dot-app/onyx | Next.js + FastAPI + Celery + model servers + Vespa/OpenSearch | 9,516 | 13.5 s | 142 MB |
| langgenius/dify | Flask API + Next.js + Celery + plugin daemon + sandbox | 13,232 | 19.9 s | 168 MB |

Speed is fine (under 20 s for 13k files). Everything below is about correctness.

## Findings

Status: **fixed** in this milestone, **open** (logged, not done).

### Routes and backends

| # | Miss | Repo | Status |
|---|---|---|---|
| R1 | Flask-RESTX/RESTful class routes (`@ns.route("/x")` on a `Resource` class, `api.add_resource`) not read. Dify's API went from 53 routes found to 1,499. | dify | **fixed** (verbs come from the class's `get`/`post`/... methods) |
| R2 | actix `web::resource("/x").route(web::get().to(h))` inside `web::scope("/p")` chains not read (axum was). | meilisearch | **fixed** (scope prefixes composed) |
| R3 | Meilisearch declares routes with its own `#[routes::routes("/x" => [get(h)])]` attribute DSL and nests them with `sub(Api)`. Still invisible: the app's real HTTP surface shows 2 routes (both in vendored examples). | meilisearch | open |
| R4 | Next.js `pages/api` handlers wrapped by middleware helpers all get method `ANY` (132 of 148 routes). tRPC routers are not modelled as routes at all. | langfuse | open |
| R5 | Flask-RESTX namespaces carry a path prefix (`/console/api`); not composed, so frontend calls may not link to it. | dify | open |

### Deployment map (compose)

| # | Miss | Repo | Status |
|---|---|---|---|
| D1 | Packaged copies of a compose file (CLI templates, embedded deploy files) overrode the canonical one, so build contexts resolved to 0 files and `api_server`/`background` came out as kind "other". | onyx | **fixed** (a copy in another folder with fewer resolved files no longer replaces the original) |
| D2 | Image-only services with no catalog match were "other". Now classified from start command (`uvicorn`, `celery`, `next start`), `MODE=worker|api` env and name/image words. | onyx, dify | **fixed** |
| D3 | `.devcontainer/`, `.cursor/` Dockerfiles became "services". | langfuse, onyx, dify | **fixed** |
| D4 | Headline counted test doubles and load generators (`mock_llm`, `loadtest`) as part of the running system and listed every name. | onyx | **fixed** (support names left out, lists capped at 4 plus "more") |
| D5 | Headline said "AI system" for any repo with one RAG stage. Online Boutique (one assistant endpoint in 11 services) and Meilisearch were called AI systems. | boutique, meilisearch | **fixed** ("AI system" needs 8+ files with AI stages or 10% of source; else "App with an AI feature") |
| D6 | Dify lists ten alternative vector/search stores as peers; they are profile-selected options. Redis cluster nodes (`redis-node-0..5`) listed as six services. | dify, langfuse | open |
| D7 | gRPC service-to-service edges (frontend to checkout, checkout to payment, ...) are the whole point of Online Boutique and are missing: `.proto` files and generated stubs are not read, so there are no edges between its services. | boutique | open |

### Structure and naming

| # | Miss | Repo | Status |
|---|---|---|---|
| S1 | Cluster names were folder basenames with a counter (`onyx-2`, `tasks-6`, `features-4`). Colliding names now take the last two folders their files share (`onyx-background`, `celery-tasks`). Some `-N` remain where files share only generic folders. | onyx, langfuse, dify | **partly fixed** |
| S2 | No workspace-package level: `web/`, `worker/`, `packages/shared`, `ai-gateway/` (Langfuse), crates (Meilisearch), `api/` vs `web/` (Dify) are what the maintainers draw first, but the tool groups by tier then by Louvain cluster. Top-level units should come from pnpm/Cargo/uv/go workspaces. | all | open (next milestone) |
| S3 | Cross-tier edges labelled "HTTP/REST xN" where files merely import shared packages (Langfuse Workers to Backend x28). Needs evidence check before labelling. | langfuse | open |
| S4 | Meilisearch's storage layer (milli over LMDB via heed) is 3 files in Storage; the engine crate sits in Backend. | meilisearch | open |

### Coverage and noise

| # | Miss | Repo | Status |
|---|---|---|---|
| C1 | C# (cartservice) and Java (adservice) are not parsed, silently. Now the summary says "Not read: 10 source files in languages without a parser yet (C# 8, Java 2)". Parsers still open. | boutique | **partly fixed** (honest, parsers open) |
| C2 | Generated protobuf stubs were indexed: Online Boutique reported 90 "data models" (the same 22 messages copied per service). Now `*.pb.go`, `*_pb2*.py`, `*.pb.ts` etc. are skipped (models 90 to 2). The `.proto` files themselves should become the models (open). | boutique | **fixed** (stubs), open (proto) |
| C3 | Vendored code (`external-crates/`, `third_party/`) and its examples produced routes and a hybrid-RAG pipeline claim. | meilisearch | **fixed** (denied dirs) |
| C4 | Data model counts are dominated by zod/DTO schemas (Dify 8,302, 4,903 from one `contracts` package; Onyx 2,339). Needs ranking or a collapse to tables first. | dify, onyx | open |
| C5 | `architecture.json` is pretty-printed: 168 MB for Dify, about half of it whitespace. Output should be compact by default. | dify, langfuse, onyx | open |
| C6 | Health false positives on Online Boutique: `grpc_health` and `pythonjsonlogger` reported as missing from requirements (import name differs from package name); locust calls to `/cart` flagged because the Go frontend's mux routes are not read. | boutique | open |

## What the maintainers' systems look like vs the detected one (after fixes)

- **Online Boutique** (11 services + Redis, gRPC): 11 containers found, kinds mostly right. Missing: the gRPC call graph (D7) and the C#/Java services' code (C1).
- **Onyx**: web server, API server, background workers, two model servers, nginx, Postgres, OpenSearch/Vespa, Redis, MinIO. All present; `background` now a worker, the model servers read as API (should be "model server", open).
- **Dify**: api, worker, worker_beat, web, plugin_daemon, sandbox, ssrf_proxy, nginx, db, redis, vector store. Present. Routes now visible (1,499).
- **Langfuse**: web, worker, ClickHouse, Postgres, Redis, MinIO, ai-gateway. Present in the container list; web/worker are not shown as the two main apps (S2).
- **Meilisearch**: single binary, index scheduler, milli engine, LMDB. The tool sees one container and no routes (R3, S2, S4).

## Next

1. S2: workspace packages as the first subsystem level.
2. D7: `.proto` service definitions and gRPC client/server edges.
3. R3/R4: route DSLs and tRPC.
4. C1: Java and C# parsers.
5. C5: compact JSON.
