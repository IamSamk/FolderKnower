# FolderKnower: Project Specification & Living Implementation Plan

> **Living document (2025–2026+).** Contributors and AI collaborators research current papers, benchmarks and open-source releases on repo-scale code search, hierarchical AST/file indexing, static + dynamic telemetry fusion and graph-grounded small models, and update this plan before implementing the affected phase.
>
> **Revision 2 (2026-11-21).** Folds in the Phase 1 research pass. Evidence: [`docs/research/synthesis.md`](docs/research/synthesis.md) (synthesis table + verified dependency and model tables) and raw notes in [`docs/research/notes/`](docs/research/notes/). The changelog at the end lists every change from Revision 1.

---

## Executive Summary

**FolderKnower** is an offline-first, deterministic system-design teacher and codebase visualizer, packaged as a cross-platform desktop app with [Tauri 2](https://github.com/tauri-apps/tauri). It turns any local repository into an interactive, hand-drawn whiteboard in the style of Excalidraw.

Every edge it draws comes from static analysis or observed runtime facts, never from an LLM. The engine is:

- a polyglot indexer: Tree-sitter for speed and coverage, plus SCIP where a compiler-exact indexer exists;
- a deterministic, content-addressed incremental index;
- clustering and complexity rules that give the same output on every run;
- an unprivileged-by-default telemetry layer, with a separate, opt-in privileged probe helper;
- a neuro-symbolic Systems Ontology;
- optional small local models that can only *explain* verified subgraphs and must cite node ids.

**Determinism contract.** The same repository bytes, FolderKnower version and ruleset version always produce a byte-identical `architecture.json`. This rules out unseeded randomness (clustering uses a fixed seed), hash-map iteration order leaking into output (output is sorted), and model output inside the graph (models only annotate, in a separate layer).

---

## The 5-Level Semantic Zoom Hierarchy

FolderKnower models a codebase with an automated variant of the C4 model and supports in-place recursive expansion. Sketch: [`docs/architecture/semantic-zoom.excalidraw`](docs/architecture/semantic-zoom.excalidraw).

```
L1 System Tiers        Frontend ⇄ Gateway/Middleware ⇄ Backend ⇄ Storage ⇄ Workers
 └ L2 Subsystem Clusters  Auth, Billing, ETL … (seeded, guided Leiden over the import/call graph)
    └ L3 Files & Non-code Nodes  source files, plus config, docs, tests, migrations
       └ L4 Functions & Call Graph  signatures; edges carry provenance + confidence
          └ L5 Telemetry HUD  Big-O, CC/cognitive, memory flags, live threads/syscalls, ontology
```

| Level | What is shown | How it is inferred | What changed in Rev 2 |
|---|---|---|---|
| L1 Tiers | 3–5 large containers with protocol arrows (HTTP, gRPC, WS, SQL, IPC) | Manifest and directory heuristics (below), then tier assignment checked against cluster membership | Tier ⇄ cluster consistency check, so a mis-classified folder is flagged rather than silently drawn |
| L2 Clusters | Functional domains | **Guided Leiden** with a fixed seed over the file graph (import edges + aggregated call edges). Synthetic anchor nodes for routers, data stores and tests are weighted to steer clusters; directory boundaries are a prior, not a hard rule | Was "directory boundaries". Now: research shows guided modularity clustering, and directory-only grouping misses cross-cutting modules |
| L3 Files | File cards, exported interfaces | Imports/exports from Tree-sitter, refined by SCIP | **Non-code files become nodes**: config, docs, tests, migrations, Dockerfiles. MULocBench shows real fixes touch them, and the best methods still find the right one only 35% of the time without them |
| L4 Functions | Signature, visibility, typed params, return type, caller → callee arrows | Tree-sitter queries, plus SCIP occurrences where available | Every edge carries `provenance` ∈ {`scip_exact`, `ts_resolved`, `ts_name_match`, `runtime_observed`} and a `confidence`. Code2flow (Python) is dropped |
| L5 HUD | Big-O, CC, cognitive complexity, memory flags, live CPU/RSS/threads/syscalls, ontology tags | Native Rust metrics over the Tree-sitter AST; optional telemetry; ontology rules | Lizard and Radon (Python) are dropped in favour of native Rust metrics. Big-O labels carry a `basis` (rule id) |

Interaction is unchanged from Rev 1: click a tier to expand in place, double-click a cluster to open it, hover a file to highlight imports, click a function to open the HUD. Edges are drawn only along the active drill-down path; everything else is dimmed.

---

## Technology Stack (verified 2026-11-21)

Versions were read from crates.io and the npm registry on 2026-11-21. Pin these exact versions in Phase 1 and bump them deliberately.

### Rust core

| Concern | Crate (version) | License | Notes and caveats |
|---|---|---|---|
| Desktop shell | `tauri` 2.12.7, `tauri-build` 2.7.6, `tauri-plugin-dialog` 2.8.4, `tauri-plugin-fs` 2.7.1, `tauri-plugin-shell` 2.6.0 | MIT/Apache-2.0 | Stay on v2. 3.0 is alpha. MSRV 1.90. Capabilities live in `src-tauri/capabilities/*.json`; each sidecar needs a scoped shell-execute permission |
| AST parsing | `tree-sitter` 0.27.2, `tree-sitter-tags` 0.27.2 | MIT | The core accepts grammar ABI 13–15 |
| Grammars | `tree-sitter-rust` 0.24.2, `-python` 0.25.0, `-javascript` 0.25.0, `-typescript` 0.23.2, `-go` 0.25.0, `-java` 0.23.5 | MIT | Use the individual crates. **Not** `tree-sitter-language-pack`, which fetches parsers at runtime and breaks offline use |
| Pattern rules | `ast-grep-core` / `ast-grep-language` 0.51.3 | MIT | Built on tree-sitter 0.27. Drives ontology and framework-idiom rules |
| SCIP | `scip` 0.10.0 (repo moved to `scip-code/scip`) | Apache-2.0 | Protobuf reader only. The indexers are external binaries (see SCIP sidecars below) |
| Walk + ignore | `ignore` 0.4.34 | MIT/Unlicense | Honours `.gitignore` |
| Hash | `blake3` 1.8.7 | CC0/Apache-2.0 | Content addressing and Merkle tree |
| Watch | `notify` 8.2.2 | CC0-1.0 | 9.0.0 shipped the same day as this check (2026-11-21). Stay on 8.2.2 until 9.x has had time to settle |
| Git metadata | `gix` 0.90.0 | MIT/Apache-2.0 | Optional: blame, churn, HEAD-relative diffs |
| Store | `rusqlite` 0.43.0 (`bundled`, FTS5) | MIT | **Chosen** over `redb` 5.0.0, because SQL queries over the graph and FTS5 keyword search are needed. `sled` is stale (2021); avoid |
| Graph | `petgraph` 0.8.3 | MIT/Apache-2.0 | In-memory graph algorithms |
| Clustering | `leiden-rs` 0.8.1 (candidate) | MIT/Apache-2.0 | Adopt only if it accepts a fixed seed and gives byte-stable output. Otherwise use an in-house Leiden of about 600 LOC over petgraph |
| Interval lookup | `rust-lapper` 2.0.1 | MIT | `FunctionLocator` (byte offset → enclosing function) |
| Tests / gates | `insta` 1.49.0, `proptest` 1.12.0, `cargo-deny` 0.22.0 | Apache-2.0 / MIT/Apache | Golden snapshots, property tests, license and advisory gate |
| Parallelism | `rayon` 1.12.0 | MIT/Apache-2.0 | Per-file parse is embarrassingly parallel. Results are sorted before output |
| Async | `tokio` 1.54.1 | MIT | App, sidecar and telemetry I/O only. The indexer core stays synchronous |
| Schema | `serde` 1.0.229, `schemars` 1.2.3, `ts-rs` 12.0.1 | MIT/Apache-2.0 | Rust types are the source of truth, exported to JSON Schema and TS. `specta`/`tauri-specta` are still 2.0 rc, so they are avoided in Phase 1 |
| Baseline telemetry | `sysinfo` 0.40.4 | MIT | Unprivileged. MSRV 1.95 |
| macOS | `libproc` 0.15.0, `mach2` 0.7.0 | MIT / BSD-2/MIT/Apache | Same-user processes only (see Security) |
| Windows | `windows` 0.62.2 (ETW APIs) and/or `ferrisetw` 1.2.0 | MIT/Apache-2.0 | **`krabsetw` is replaced**: it is C++ distributed via NuGet, not on crates.io, and its license was unconfirmed. `ferrisetw` is pure Rust but its last release was 2024-06, so be ready to call `windows` ETW directly |
| Linux eBPF | `aya` 0.16.0, `aya-log` 0.4.0, `aya-build` 0.2.1, `aya-ebpf` 0.3.0 | MIT/Apache-2.0 | MSRV 1.98. The eBPF side needs nightly + `rust-src` + `bpf-linker`, so it lives only in the separate `ark-probe` crate. eBPF programs calling GPL-only helpers must declare a GPL-compatible license section (to be verified against kernel docs before Phase 4) |
| Async correlation | `console-subscriber` 0.5.0, `pprof` 0.15.0 | MIT / Apache-2.0 | Phase 4 |
| Local ML | `candle-core` / `candle-transformers` 0.11.1, `tokenizers` 0.23.2, `llama-cpp-2` 0.1.161 | MIT/Apache-2.0 | Phase 5. candle covers `qwen3`, `quantized_qwen3`, `nomic_bert`, `modernbert`, `jina_bert`; it has **no `qwen3_5` module** yet |

**Dropped:** `rust-code-analysis` (last release 2023-01, MPL-2.0, stale), Lizard and Radon (Python runtime), code2flow (Python).

### Frontend

| Concern | Package (version) | License | Notes |
|---|---|---|---|
| Package manager | `pnpm` 12.21.0 | MIT | pnpm 12 blocks dependency build scripts by default (allowlist via `pnpm.onlyBuiltDependencies`) and has a minimum-release-age gate |
| Language | `typescript` 7.0.2 | Apache-2.0 | This is the Go-native compiler (`tsgo`). It has **no programmatic API** until 7.1, so lint with **Biome** rather than typescript-eslint. TS 7 makes `baseUrl` and `target: es5` hard errors |
| Lint/format | `@biomejs/biome` 2.6.8 | MIT/Apache-2.0 | Single tool for lint and format |
| Build/test | `vite` 8.3.8, `@vitejs/plugin-react` 6.3.0, `vitest` 5.1.1 | MIT | |
| UI | `react` / `react-dom` 19.3.1 | MIT | |
| Canvas | `@xyflow/react` 13.0.2 | MIT | **v13 (2026-11-16) has breaking changes**: `useHandleConnections`/`getHandleConnections`/`useStore` were removed or replaced; parents no longer need to precede children; new `changeParent`. Write against the v13 API from the start |
| Layout | `elkjs` 0.12.1 | **EPL-2.0** OR GPL-3.0+ | Choose EPL-2.0. Ship unmodified, in a Web Worker via `elk-api` + `workerFactory` |
| Hand-drawn | `roughjs` 4.6.6 | MIT | Last release was 2023. It is stable but unmaintained, so keep it behind a thin `sketch/` adapter |
| Interop | `@excalidraw/excalidraw` 0.18.1 | MIT | Only for `.excalidraw` export/import (Phase 3); lazy-loaded |
| State | `zustand` 5.1.0, `@tanstack/react-query` 5.108.2 | MIT | |
| Styling | `tailwindcss` / `@tailwindcss/vite` 4.3.5 | MIT | |
| Tauri JS | `@tauri-apps/api` 2.12.5, `@tauri-apps/cli` 2.12.6, plugins 2.x | MIT/Apache-2.0 | |

### SCIP sidecars (optional, detected, never required)

| Language | Indexer | Runtime need | Plan |
|---|---|---|---|
| Rust | `rust-analyzer scip` | none (native) | Bundle as a sidecar |
| Go | `scip-go` v0.2.10 | none (native) | Bundle as a sidecar |
| TS/JS | `@sourcegraph/scip-typescript` 0.6.2 | Node | Use it if a Node runtime is present; otherwise Tree-sitter only |
| Python | `@sourcegraph/scip-python` 0.6.8 | Node | Same as TS/JS |
| Java | `scip-java` 0.12.3 | JVM + Gradle/Maven | Detect only. Never bundled |

A user can also drop in a pre-built `index.scip`. SCIP runs only when a project's build manifests change; ordinary edits are re-linked by Tree-sitter, and SCIP edges are marked stale until the next SCIP run.

---

## Detailed System Architecture

### 1. Ingestion and indexing pipeline (Phase 1)

```
repo root
  │
  ├─1 Scan        ignore-aware walk → blake3(content) per file → Merkle dir tree
  │               diff old/new Merkle roots → changed set (only these are re-parsed)
  ├─2 Parse       Tree-sitter per file (rayon) → FileFacts {symbols, imports, calls, ranges}
  │               cached by key (blake3, grammar_abi+version, extractor_version)
  ├─3 Chunk       cAST: split-then-merge over the AST, ~2000 non-whitespace chars
  │               (embedding/search units; FTS5 indexed)
  ├─4 Link        cross-file resolution: SCIP occurrences if present, else import-scoped name resolution
  │               each edge tagged with provenance + confidence
  ├─5 Metrics     cyclomatic, cognitive, nesting, Big-O rule engine, memory flags (per function)
  ├─6 Ontology    ast-grep + symbol rules (YAML) → OS / network / cloud tags
  ├─7 Structure   tier heuristics (L1) → guided seeded Leiden (L2) → consistency check
  ▼
architecture.json (schema v2, sorted, byte-stable)  +  .folderknower/index.sqlite (cache)
```

**Index format decision.** Per-file artifacts are content-addressed: the key is `blake3(file bytes) ‖ grammar id@abi ‖ extractor version`, and artifacts are stored in SQLite tables (`files`, `symbols`, `edges`, `chunks`, `chunks_fts`). Unchanged files are never re-parsed, and per-file facts are independent of other files, as in stack-graphs; only the cheap link pass (4) is global. This follows the per-file-isolation idea from stack graphs and the Merkle-diff design in "Code Isn't Memory" (2026). Parent summaries are cached by the hashes of their children, so they regenerate only when something below them changes.

**Search.** Three channels feed results: (a) a deterministic repo-map (paths, symbols, references), (b) FTS5 keyword search over cAST chunks, (c) optional embeddings (Phase 5). The repo-map comes first: Agent Retrieval Bench (2026) found it gives the most relevant context per token, and fusing it with embeddings is an add-on, not a replacement. Graph expansion defaults to 1 hop (the small-agent depth study, 2026: gains concentrate at 0–1 hops).

### 2. Call graphs and complexity

- **Edges.** Static tools beat LLMs at call-graph construction by a wide margin (the 2024–25 Paderborn study found PyCG at 84.9% completeness versus 60.3% for the best LLM), so LLMs are never used to create edges. Unresolved call sites keep a `candidates[]` list. Phase 4 runtime samples can promote a candidate to `runtime_observed`. There is no learned pruner: the ICSE 2026 study found baseline precision already about 0.97, and pruning mainly lost coverage.
- **Big-O rules** keep the Rev 1 table (O(1), O(N), O(N²), O(N^k), O(log N), O(N log N), O(2^N)) with additions:
  - loop bounds are classified as constant literals (O(1)), input-derived (N), or halving/doubling strides (log N);
  - recursion is classified by its number of self-calls and how the argument shrinks;
  - a **library cost table** covers calls such as `sort` (N log N), `includes`/`in list` (N) and `dict get` (1);
  - each label carries a `basis` rule id, so the HUD can say *why*.

  LLMs are not used here either: the best model on BigO(Bench) reaches only 29.2 pass@1. CodeComplex (EMNLP 2025 Findings; 4.9k Java + 4.9k Python, labelled) is the regression set. An *optional* empirical fit (BigO(Bench)'s profile-and-least-squares method) can confirm labels later, using the Phase 4 telemetry.
- **Cyclomatic and cognitive complexity** are computed natively over Tree-sitter node kinds, with per-language decision-node tables. There is no Python dependency.

### 3. Cross-platform telemetry (Phase 4)

```rust
/// Unprivileged, in-process. Always available.
pub trait TelemetryProvider: Send + Sync {
    fn capabilities(&self) -> ProviderCaps;                 // what this OS/tier can report
    fn snapshot(&self, target: Pid) -> Result<ProcSnapshot>; // threads, cpu, rss, fds/sockets
    fn subscribe(&self, target: Pid, cfg: StreamCfg) -> Result<TelemetryStream>;
}

/// Lives ONLY in the separate privileged helper `ark-probe`; the app talks to it over IPC.
pub trait KernelProbe {
    fn attach(&mut self, req: AttachRequest) -> Result<SessionId>; // validated, user-owned pid only
    fn poll(&mut self, s: SessionId, max: usize) -> Result<Vec<KernelEvent>>; // rate-limited
    fn detach(&mut self, s: SessionId) -> Result<()>;
}

/// Maps samples back to the static graph.
pub trait Symbolizer {
    fn resolve(&self, frame: &Frame) -> Option<SourceLoc>;      // addr → symbol → file:line
}
pub struct FunctionLocator { /* interval tree of Tree-sitter function ranges per file */ }
```

Implementations:

| Tier | Linux | Windows | macOS |
|---|---|---|---|
| Baseline (in app) | `sysinfo` + `/proc` (own uid) | `sysinfo` + PDH | `sysinfo` + `libproc` + Mach (same user) |
| Deep (`ark-probe`) | aya: `sched_switch`, `sys_enter_*`, `tcp_connect`, perf-event stack sampling | ETW kernel session via `windows` crate / `ferrisetw` | No kernel tracing in v1. ES framework needs an Apple-restricted entitlement; deferred |

**Fusion path.** A sample stack is symbolized to a symbol, then to file:line, then to the `FunctionLocator` interval tree to find the enclosing Tree-sitter function, then to that function's graph node id. Leaf frames use the profiler's unwind data. The OpenTelemetry eBPF profiler is the reference design: it unwinds from `.eh_frame` with no frame pointers or debug info and stays under 1% CPU. Observed thread entry points and callbacks are added as roots (FSE 2025: using runtime-observed roots as extra entry points grew Android static call graphs from about 50.6K to 65.5K nodes on average). Async tasks map to OS threads through `console-subscriber` for tokio targets.

### 4. Security boundaries and least-privilege elevation (Phase 4, designed now)

**Principle.** The Tauri app *never* runs elevated. The only privileged code is a small, separately signed helper, `ark-probe`, installed once with explicit consent. It exposes a narrow, typed, rate-limited IPC API, and it reads telemetry only for processes owned by the requesting user.

| | Linux | Windows | macOS |
|---|---|---|---|
| Privilege needed | `CAP_BPF` + `CAP_PERFMON` (kernel ≥ 5.8). Before 5.8, `CAP_SYS_ADMIN` | Kernel logger needs Admin/SYSTEM. Performance Log Users can run real-time sessions for user-mode providers | Same-user `proc_pidinfo` works unprivileged. `task_for_pid` is debugger-only (avoid). Endpoint Security needs a restricted entitlement |
| Helper form | systemd **service** with `AmbientCapabilities=CAP_BPF CAP_PERFMON`, `CapabilityBoundingSet` = the same, `NoNewPrivileges`, `ProtectSystem=strict`, `PrivateTmp`, `ProtectHome=read-only`. Not setcap on a binary and not pkexec per use | Windows **service** installed by the MSI. The app runs `asInvoker`, never UAC-prompts at runtime | `SMAppService` daemon (macOS 13+), user-approved in Login Items. Deep tier deferred, so v1 ships no helper on macOS |
| IPC | Unix socket in `/run/folderknower/`, `SO_PEERCRED` → uid; requests only for PIDs owned by that uid | Named pipe with a strict DACL; authenticate by **SID via impersonation**, not `GetNamedPipeClientProcessId` (spoofable, per Project Zero) | XPC with `xpc_connection_set_peer_code_signing_requirement` pinning Team ID + bundle id. No PID-based checks (TOCTOU) |
| Host settings | Respect `kernel.unprivileged_bpf_disabled` (Ubuntu default 2) and `perf_event_paranoid` (Debian/Ubuntu 3); explain, never auto-change | | |

**Cross-cutting rules**
- Data minimization: no argv, env, file contents or packet payloads by default. Only counts, thread ids, syscall numbers, socket 4-tuples and stack addresses.
- PID allowlist: the target must be owned by the requesting user and chosen explicitly in the UI. There is no system-wide capture mode.
- Ring buffers are bounded and events rate-limited. The helper drops on overload rather than blocking.
- eBPF objects are embedded in the signed helper. No runtime loading of user-supplied BPF.
- The helper has no network access, does no file writes beyond its socket, and exposes no generic "run" or "load" command.
- Treat the Linux helper as **root-equivalent for confidentiality**: with CAP_BPF + CAP_PERFMON it can read kernel memory, so it must be small, audited and fuzzed.
- Tauri: one capability file per window, with the minimal permission set. `fs` is scoped to the user-chosen repo root (read-only) plus `$APPDATA/folderknower`. `shell` may run only the bundled sidecars, with fixed argument validators. No `http` plugin, so the app makes zero network requests. A strict CSP. Capabilities do not protect against bugs in Rust commands, so every command validates its paths.
- Analysed repositories are untrusted input. The indexer never executes repo code, never runs repo build scripts and never follows symlinks out of the root. SCIP indexers that need a project build (`scip-java`) are user-triggered with a warning.

Threat model and details: [`docs/security/telemetry-elevation.md`](docs/security/telemetry-elevation.md).

### 5. Neuro-symbolic layer (Phase 5; schema reserved in Phase 1)

1. **Ontology (no AI).** Versioned YAML rules (ast-grep patterns + fully-qualified symbol matches) map code to concepts (thread spawn, event-loop blocking risk, OSI L4/L7, managed service, IAM boundary, and so on). Each tag stores its `rule_id`.
2. **Role classifier.** A prototype (nearest-centroid) classifier over function embeddings plus structural features (fan-in/out, ontology tags, I/O calls), with a fixed role set: Network I/O, Auth Gate, State Mutation, CPU-bound Compute, Serialization, Orchestration, Unknown. No 2025–26 benchmark exists for this task, so FolderKnower ships its own labelled fixture set. Below a confidence threshold the role is `Unknown`, never guessed.
3. **Grounded narrator (optional SLM).** The input is a serialized, verified subgraph with stable node ids. Output uses JSON-schema-constrained decoding (llama.cpp grammars), with citation fields as an **enum of the node ids in the subgraph**, so the model cannot cite anything outside it. A post-check (ETF-style entity tracing) rejects sentences that mention entities not in the subgraph. Narration is stored separately and never modifies the graph.

**Model shortlist (all verified on the HF Hub, 2026-11-21).** Models are opt-in, never downloaded automatically, and checked against SHA-256 values pinned in a manifest.

| Role | Default | Alternative | Avoid (license) |
|---|---|---|---|
| Code embeddings | `nomic-ai/CodeRankEmbed` (137M, MIT, CoIR 60.1, candle `nomic_bert`) | `Qwen/Qwen3-Embedding-0.6B` (596M, Apache-2.0, official GGUF); evaluate `google/embeddinggemma-2` (2026-09, Apache-2.0, GGUF/ONNX) | `jinaai/jina-code-embeddings-*` (CC-BY-NC), `embeddinggemma-300m` (Gemma license, gated) |
| Tiny encoder for classifier fine-tune | `ibm-granite/granite-embedding-small-english-r2` (48M, Apache-2.0, ModernBERT) | CodeRankEmbed | |
| Narrator SLM | `Qwen/Qwen3-4B-Instruct-2507` (4.0B, Apache-2.0, Q4_K_M ≈ 2.5 GB, candle + llama.cpp) | Low-RAM: `Qwen/Qwen3-1.7B` (Apache-2.0, ≈ 1.1 GB). MIT option: `microsoft/Phi-4-mini-instruct`. Track `Qwen/Qwen3.5-2B` / `-4B` (Apache-2.0, 2026) once candle gains `qwen3_5` support | `Qwen2.5-Coder-3B-Instruct` (Qwen research license), Llama 3.2 (community license), Gemma 3 (Gemma license) |

Runtime: `llama-cpp-2` for GGUF generation (mature quantization), and candle for embeddings and the classifier (pure Rust, no C++ build). Both sit behind one `ark-ml` trait so either can be swapped.

---

## Core Data Schema (v2)

Rust types in `ark-schema` are the source of truth. They are exported to `schema/architecture.schema.json` (schemars) and `apps/desktop/src/gen/*.ts` (ts-rs). Changes from v1 are marked `// v2`.

```typescript
export interface SystemArchitectureGraph {
  schemaVersion: "2.0";                        // v2
  repoName: string;
  rootPath: string;                            // stored relative-safe; never leaves the machine
  generator: { version: string; rulesetVersion: string; grammars: Record<string, string> }; // v2
  contentRoot: string;                         // v2: Merkle root hash (blake3) → determinism key
  tiers: TierNode[];
  crossTierEdges: CrossTierEdge[];
  nonCodeNodes: NonCodeNode[];                 // v2: config/docs/tests/migrations/infra
}

export interface TierNode {
  id: string;                                  // "tier:backend"
  label: "Frontend" | "Gateway" | "Backend" | "Storage" | "Workers" | "Shared";
  evidence: string[];                          // v2: manifest/dir rule ids that fired
  clusters: SubsystemCluster[];
}

export interface SubsystemCluster {
  id: string;                                  // "cluster:auth"
  name: string;
  method: { algo: "leiden"; seed: number; resolution: number; anchors: string[] }; // v2
  files: SourceFileNode[];
}

export interface SourceFileNode {
  id: string;                                  // "file:src/auth/service.ts"
  path: string;
  language: string;
  contentHash: string;                         // v2: blake3
  imports: Edge[];                             // v2
  functions: FunctionNode[];
}

export interface NonCodeNode {                 // v2
  id: string; path: string;
  kind: "config" | "doc" | "test" | "migration" | "infra" | "manifest";
  relatesTo: string[];                         // file/function ids (test→source, doc→source)
}

export interface Edge {                        // v2: shared by imports and calls
  target: string;
  provenance: "scip_exact" | "ts_resolved" | "ts_name_match" | "runtime_observed";
  confidence: number;                          // 0..1, deterministic from provenance + resolution rule
  candidates?: string[];                       // unresolved: ranked by deterministic rule, not model
}

export interface FunctionNode {
  id: string;                                  // SCIP symbol if available, else "fn:<path>#<qualname>"
  name: string;
  visibility: "public" | "private" | "protected" | "internal";
  lineRange: [number, number];
  byteRange: [number, number];                 // v2: for FunctionLocator
  signature: { parameters: Array<{ name: string; type?: string; defaultValue?: string }>; returnType?: string };
  complexity: {
    cyclomatic: number;
    cognitive: number;
    maxNesting: number;                        // v2
    timeBigO: BigO; spaceBigO: BigO;
    basis: string[];                           // v2: rule ids that produced the label
    memoryFlags: string[];
  };
  ontologyTags: Array<{ concept: string; layer: "os" | "network" | "cloud" | "data"; ruleId: string }>; // v2
  inferredRole?: { role: string; confidence: number; model: string }; // Phase 5, separate layer
  liveTelemetry?: { cpuPercent: number; rssBytes: number; activeThreads: number; syscallsPerSec?: number };
  calls: Edge[];                               // v2: was string[]
  calledBy: string[];
}

type BigO = "O(1)" | "O(log N)" | "O(N)" | "O(N log N)" | "O(N^2)" | "O(N^k)" | "O(2^N)" | "unknown";

export interface CrossTierEdge {
  id: string; source: string; target: string;
  protocol: "HTTP/REST" | "WebSocket" | "gRPC" | "SQL" | "IPC" | "Queue";
  evidence: string[];                          // v2: e.g. ontology rule ids at call sites
}
```

---

## Frontend Component Hierarchy (Phase 2–3, designed now)

```
<App>
 ├ <TauriBridge/>                      typed invoke wrappers (generated TS types)
 ├ <ProjectShell>
 │   ├ <TopBar>  repo picker · index status · LOD breadcrumb (L1 › L2 › L3 › L4)
 │   ├ <Whiteboard>                    @xyflow/react v13 <ReactFlow>, controlled by zustand graphStore
 │   │   ├ nodeTypes: TierNode · ClusterNode · FileCard · NonCodeCard · FunctionNode
 │   │   │    each wraps <SketchFrame> (roughjs → SVG, seeded by node id so strokes are stable)
 │   │   ├ edgeTypes: SketchEdge (roughjs path; style encodes provenance:
 │   │   │    solid = scip_exact, dashed = ts_*, dotted = candidate, glow = runtime_observed)
 │   │   ├ <MiniMap/> · <Controls/> · <Background variant="dots"/>
 │   │   └ <SemanticZoomController>    maps viewport zoom + expansion set → visible LOD
 │   ├ <Inspector>  side panel: signature, edges w/ provenance, ontology tags + rule ids
 │   └ <TelemetryHUD>  docked to the selected node (L5): Big-O + basis, CC, memory, live sparkline
 └ workers/
     ├ layout.worker.ts                elkjs (elk-api + workerFactory), layered, hierarchyHandling=INCLUDE_CHILDREN
     └ diff.worker.ts                  applies incremental graph patches from Rust
```

Design rules (from the frontend-design pass):
- Excalidraw-like warmth: Virgil/Excalifont for labels, a system UI font for data.
- A paper-white canvas with a dark-mode equivalent.
- Tier colours from one palette: blue = frontend, purple = gateway, green = backend, teal = storage, amber = workers.
- Rough strokes are **seeded by node id**, so a node never "wiggles" between renders (determinism extends to visuals).
- Expansion animates layout deltas: ELK runs in a worker, and React Flow interpolates positions over 250 ms.
- Only the active drill-down path draws cross-function edges.

---

## Deterministic Heuristics

### Macro-tier classification (unchanged in substance; each rule now has an id)
- **Frontend** (`tier.fe.*`): `vite.config.*`, `next.config.*`, `package.json` with a React/Vue/Svelte/Solid dependency; folders `client/`, `frontend/`, `web/`, `app/`, `ui/`.
- **Backend** (`tier.be.*`): `Cargo.toml` with axum/actix/rocket, `go.mod`, `pom.xml`/`build.gradle` with Spring, `pyproject.toml` with FastAPI/Django/Flask, an Express/Nest/Hono dependency.
- **Gateway/middleware** (`tier.gw.*`): `middleware/`, `gateway/`, `proxy/`, `interceptors/`, `nginx.conf`, `envoy.yaml`, `traefik.*`.
- **Storage** (`tier.db.*`): `schema.prisma`, `migrations/`, `alembic/`, `*.sql`, compose services `postgres|mysql|mongo|redis`.
- **Workers** (`tier.wk.*`): `celery`, `bullmq`, `sidekiq`, `worker/`, `jobs/`, cron manifests.

### Static memory profile (unchanged), plus a rule id per flag.

---

## Implementation Roadmap

### Phase 1: Static ingestion and Systems Ontology (weeks 1–3). Detailed checklist: [`docs/phase1-checklist.md`](docs/phase1-checklist.md)

Deliverable: `fknow index <path> --out architecture.json` (CLI, no UI) for TS/JS, Python, Rust and Go, with a deterministic, incremental SQLite cache.

Milestones:
- **M1.1** Workspace, CI, license gate.
- **M1.2** Schema v2 and its generators.
- **M1.3** Scanner, content-addressed store and incremental re-runs.
- **M1.4** Tree-sitter extraction and cAST chunks.
- **M1.5** Metrics and Big-O rules.
- **M1.6** SCIP import and edge provenance.
- **M1.7** Ontology v0 (≥ 60 rules).
- **M1.8** Tiers and guided Leiden.
- **M1.9** Golden-output tests on fixture repos.

### Phase 2: Hand-drawn canvas and compound layout (weeks 4–6)
- Tauri 2 + React 19 + `@xyflow/react` 13 app. Custom rough-styled nodes and edges. ELK in a worker.
- Tauri capabilities locked down: read-only repo scope, no network.

### Phase 3: Recursive semantic zoom and export (weeks 7–8)
- In-place expansion across L1–L4, the LOD controller, the provenance legend, `.excalidraw` export (lazy-loaded), and a live `notify` watcher driving incremental patches.

### Phase 4: Cross-platform telemetry (weeks 9–10)
- `ark-telemetry` baseline on all three OSes.
- `ark-probe` on Linux (aya) and Windows (ETW).
- Symbolizer, `FunctionLocator` and runtime-edge promotion.
- Security review and fuzzing of the helper IPC.
- **M4.1** Provider trait and sysinfo baseline.
- **M4.2** Linux probe and systemd unit.
- **M4.3** Windows service and ETW.
- **M4.4** Symbolization and fusion.
- **M4.5** IPC fuzzing (cargo-fuzz) and threat-model sign-off.

### Phase 5: Neuro-symbolic teacher and hardening (weeks 11–12)
- `ark-ml`: embeddings (candle), the prototype role classifier, and the constrained-decoding narrator (llama.cpp).
- Benchmarks on TodoMVC, FastAPI RealWorld, Express, a Rust/axum service and an Ollama-sized repo.
- Installers, and a privacy audit (zero outbound connections, verified with a network-deny test).

### Milestone 6: Live insight (memory, data structures, AI architecture). Plan: [`docs/milestones/m6-live-insight/`](docs/milestones/m6-live-insight/README.md)
- **M6.1 (done 2026-11-25):**
  - `ark-patterns`: DSA, AI pipeline, memory-technique, library and OKF recognition.
  - Enrich pass: identifiers, call arguments and control-flow outline.
  - `ark-telemetry` sampler and model-server parsers.
  - Desktop: How it works, Workflow (clean path) at every zoom level, Live with the KV cache calculator.
- **M6.2:** precision fixes, py-spy / PEP 669 runtime overlay, OTLP + OpenInference ingest.
- **M6.3:** scale (paging, LOD, WebGL), dependency and native-binary views, low-end budgets in CI.
- **M7–M9:** domain packs in order: networks, security, IoT.

---

## Technical Challenges and Mitigations

1. **Compound layout churn.** ELK in a worker. Layout input is sorted by id for stable results. Positions are animated.
2. **Dynamic typing.** Types are optional in the schema. SCIP is used where available, and the UI never shows a fake type.
3. **Kernel privilege.** The app is unprivileged by default, with a separate signed helper. macOS deep mode is deferred (see Security).
4. **Graph density.** Leiden clusters, path-only edge rendering, 1-hop default expansion, and LOD.
5. **Indexer runtimes.** SCIP is optional, and Tree-sitter-only mode is fully functional. Provenance tells users exactly which edges are compiler-exact.
6. **Determinism regressions.** A CI golden test re-indexes every fixture twice, plus once with shuffled file order, and diffs the bytes.
7. **Untrusted repos.** Parsing is pure. Repo code is never executed, symlinks are not followed outside the root, and per-file size and time limits apply (huge or minified files are skipped with a reason).

---

## Ongoing Research Tracking

Before each phase, re-run the literature check, update `docs/research/synthesis.md`, and record changes here. Open questions from Rev 2:
- No 2025–26 call-graph benchmark exists for Go or Rust. Build our own from fixture repos with SCIP as ground truth.
- No peer-reviewed work fuses eBPF stack samples with AST/SCIP graphs. This is a potential contribution of the project.
- No prior work clusters above the file level into tiers. Evaluate L1/L2 against hand-labelled architectures of the fixture repos.
- Verify: the GPL license-section requirement for aya eBPF programs, `ferrisetw` maintenance status, candle `qwen3_5`/`gemma4` support, and EmbeddingGemma-2's size and code scores.

---

## Changelog: Revision 1 → Revision 2

| # | Change | Why (source) |
|---|---|---|
| 1 | Content-addressed per-file cache + Merkle diff in SQLite | Incremental indexing (stack graphs; "Code Isn't Memory" 2026) |
| 2 | cAST chunking for search units | cAST (arXiv 2506.15655): +1.8–4.3 Recall@5 over line chunks |
| 3 | Non-code files as graph nodes | MULocBench (2509.25242), LARGER (2605.16352) |
| 4 | Edge provenance + confidence; no LLM edges; no learned pruning | LLM call-graph study (2410.00603); ICSE 2026 pruning study |
| 5 | Guided, seeded Leiden for L2 | Guided Leiden (Univ. Vienna); ArchAgent (2601.13007) |
| 6 | Repo-map + FTS5 before embeddings; 1-hop default | Agent Retrieval Bench (2607.24882); depth study (2610.33958) |
| 7 | Lizard/Radon/code2flow replaced by native Rust metrics | No Python runtime offline; rust-code-analysis stale |
| 8 | Big-O rules with a `basis` and library cost table; CodeComplex regression set | BigO(Bench) (2503.15242): LLM pass@1 29.2 |
| 9 | krabsetw → `windows` crate / `ferrisetw` | krabsetw is C++, NuGet-only |
| 10 | Separate signed `ark-probe` helper; per-OS IPC authentication | Security research (capabilities(7), ETW docs, Apple DTS, Project Zero) |
| 11 | Model shortlist with license gate; constrained decoding with node-id enum citations | HF Hub verification; JSONSchemaBench (2501.10868); ETF (2410.14748) |
| 12 | Stack bump: pnpm 12, TS 7 (tsgo) + Biome, Vite 8, React 19.3, React Flow 13, Tauri 2.12 | Registry checks, 2026-11-21 |

---

## Distribution, CLI & Web Platform Roadmap (v0.3.0+)

### 1. Zero-Install CLI: `npx folderknower` / `pnpm dlx folderknower`
To allow any developer or CI environment to run FolderKnower without installing Rust, Cargo, or system libraries:
- **npm package wrapper (`folderknower`)**:
  - Bin script `bin/folderknower.js` in a lightweight npm package.
  - When executed (`npx fknow index .` or `pnpm dlx fknow index .`), detects `process.platform` (`win32`, `darwin`, `linux`) and `process.arch` (`x64`, `arm64`).
  - Fetches the exact pre-compiled release binary (`fknow-cli-*`) from GitHub Releases (`https://github.com/IamSamk/FolderKnower/releases/latest/download/...`).
  - Caches binary locally in `~/.folderknower/bin/` and executes it directly with full arguments.

### 2. One-Line Shell Installers
- **macOS / Linux**: `curl -fsSL https://<domain>/install.sh | bash`
- **Windows (PowerShell)**: `irm https://<domain>/install.ps1 | iex`
- Hosted in `apps/web/public/` so they are immediately available on the hosted platform.

### 3. Vercel Monorepo Deployment
- **Architecture**: Next.js 15 (App Router) in `apps/web/`.
- **Root Directory setting**: In Vercel Project Settings > General, set **Root Directory** to `apps/web`.
- Vercel automatically detects Next.js, installs workspace dependencies via pnpm from the repo root, and outputs the live platform without configuration conflicts.

### 4. Free OS Safety & Gatekeeper Verification
- **Windows SmartScreen**: Integrate **SignPath.io** (free Authenticode code signing for open-source GitHub repositories) into `.github/workflows/release.yml`.
- **macOS Gatekeeper**: Requires Apple Developer ID certificate in GitHub secrets (`APPLE_CERTIFICATE`, `APPLE_SIGNING_IDENTITY`) to automate Apple Notarization.
- **Linux Distribution**: Package for Flathub (Flatpak) and maintain native `.deb` / `.rpm` repositories.
