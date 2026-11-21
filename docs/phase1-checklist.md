# Phase 1 Execution Checklist: Static Ingestion & Systems Ontology

**Target.** `architekton index <repo> --out architecture.json` for TS/JS, Python, Rust and Go. It must be deterministic (byte-identical across runs) and incremental (a re-run after a one-file edit re-parses one file). There is no UI in Phase 1; the Tauri app is only a scaffold.

Versions were verified on crates.io and npm on 2026-11-21 (see [`research/synthesis.md`](research/synthesis.md)).

---

## 1. Toolchain

| Tool | Version | Notes |
|---|---|---|
| Rust (stable) | ≥ 1.95 (pin in `rust-toolchain.toml`) | `sysinfo` needs 1.95. `tauri`/`tree-sitter`/`ast-grep` need 1.90. Edition 2024 |
| Rust (nightly + `rust-src`, `bpf-linker`) | Phase 4 only | Only for `crates/ark-probe-ebpf`. Excluded from the default workspace members |
| Node.js | current LTS | Only needed to run pnpm and the frontend; not needed by the indexer |
| pnpm | 12.21.0 (`packageManager` field + Corepack) | Set `onlyBuiltDependencies` (allowlist esbuild, @tauri-apps/cli) |
| TypeScript | 7.0.2 | Go-native compiler. **Biome** handles lint because typescript-eslint has no TS 7 API yet |
| cargo-deny | 0.22.0 | License allowlist: MIT, Apache-2.0, BSD-2/3, ISC, Unicode-3.0, Zlib, CC0-1.0, MPL-2.0 (file-level only), Unlicense. Deny GPL/AGPL in the app graph |

## 2. Folder structure

```
Architekton/
├── Cargo.toml                    # [workspace] resolver = "3"; members = crates/* except ark-probe*
├── rust-toolchain.toml
├── deny.toml                     # cargo-deny licenses + advisories
├── package.json                  # pnpm workspace root ("packageManager": "pnpm@12.21.0")
├── pnpm-workspace.yaml           # packages: apps/*
├── biome.json
├── plan.md
├── crates/
│   ├── ark-schema/               # v2 graph types → schema/architecture.schema.json + TS via ts-rs
│   ├── ark-scan/                 # ignore-walk, blake3, Merkle tree, change set; notify watcher (feature)
│   ├── ark-store/                # rusqlite (bundled, FTS5): content-addressed artifacts, migrations
│   ├── ark-parse/                # tree-sitter grammars + per-language queries (queries/<lang>/*.scm)
│   │   └── queries/{typescript,javascript,python,rust,go}/{symbols,imports,calls}.scm
│   ├── ark-chunk/                # cAST split-then-merge chunker
│   ├── ark-link/                 # cross-file resolution, edge provenance; SCIP import (scip crate)
│   ├── ark-metrics/              # cyclomatic, cognitive, nesting, Big-O rules, memory flags
│   ├── ark-ontology/             # YAML rule loader + ast-grep matcher
│   ├── ark-structure/            # tier heuristics + guided seeded Leiden + consistency check
│   ├── ark-core/                 # pipeline orchestration (pure, sync, rayon)
│   ├── ark-cli/                  # `architekton` binary (clap)
│   ├── ark-telemetry/            # Phase 4: trait + sysinfo baseline (stub in Phase 1)
│   └── ark-probe/                # Phase 4: privileged helper (NOT a workspace default member)
├── ontology/
│   ├── rules/{os,network,cloud,data}.yaml
│   └── tiers.yaml
├── schema/architecture.schema.json   # generated, committed, CI checks it is fresh
├── fixtures/repos/                   # small vendored repos + expected/*.json goldens
├── apps/desktop/                     # Phase 2: Tauri 2 + React 19 (scaffold only in Phase 1)
│   ├── src-tauri/{Cargo.toml,tauri.conf.json,capabilities/main.json}
│   └── src/{gen/,workers/,components/,sketch/,stores/}
└── docs/
    ├── phase1-checklist.md
    ├── research/{synthesis.md,notes/}
    ├── security/telemetry-elevation.md
    └── architecture/semantic-zoom.excalidraw
```

## 3. Exact dependencies (Phase 1)

```toml
# Cargo.toml [workspace.dependencies]
tree-sitter            = "0.27.2"
tree-sitter-tags       = "0.27.2"
tree-sitter-typescript = "0.23.2"
tree-sitter-javascript = "0.25.0"
tree-sitter-python     = "0.25.0"
tree-sitter-rust       = "0.24.2"
tree-sitter-go         = "0.25.0"
ast-grep-core          = "0.51.3"
ast-grep-language      = "0.51.3"
scip                   = "0.10.0"
ignore                 = "0.4.34"
blake3                 = "1.8.7"
notify                 = "8.2.2"     # feature-gated; 9.0.0 too fresh (2026-11-21)
rusqlite               = { version = "0.43.0", features = ["bundled"] }  # FTS5 is in the bundled SQLite build; confirm when wiring ark-store
petgraph               = "0.8.3"
leiden-rs              = "0.8.1"     # evaluate in M1.8; fallback in-house
rust-lapper            = "2.0.1"
rayon                  = "1.12.0"
serde                  = { version = "1.0.229", features = ["derive"] }
serde_json             = "1"
serde_yaml_ng          = "*"         # pick current at bootstrap; serde_yaml is archived
schemars               = "1.2.3"
ts-rs                  = "12.0.1"
clap                   = { version = "4", features = ["derive"] }
thiserror              = "2"
anyhow                 = "1"
tracing                = "0.1"
tracing-subscriber     = "0.3"
# dev
insta                  = { version = "1.49.0", features = ["json"] }
proptest               = "1.12.0"
```

Frontend scaffold (Phase 2 will add the rest):

```jsonc
// apps/desktop/package.json (scaffold)
"dependencies": {
  "react": "19.3.1", "react-dom": "19.3.1",
  "@xyflow/react": "13.0.2", "elkjs": "0.12.1", "roughjs": "4.6.6",
  "zustand": "5.1.0", "@tanstack/react-query": "5.108.2",
  "@tauri-apps/api": "2.12.5", "@tauri-apps/plugin-dialog": "2.8.4", "@tauri-apps/plugin-fs": "2.7.1"
},
"devDependencies": {
  "typescript": "7.0.2", "vite": "8.3.8", "@vitejs/plugin-react": "6.3.0",
  "vitest": "5.1.1", "@biomejs/biome": "2.6.8",
  "tailwindcss": "4.3.5", "@tailwindcss/vite": "4.3.5", "@tauri-apps/cli": "2.12.6"
}
```

## 4. First 5 engineering tasks

### Task 1: Bootstrap the workspace, CI and license gate (M1.1)
- Create the Cargo workspace (resolver 3, edition 2024), `rust-toolchain.toml`, empty crates with `lib.rs`, the pnpm workspace and `biome.json`.
- Add `deny.toml` with the license allowlist above.
- CI matrix: ubuntu, windows and macOS. Steps: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo deny check`, `pnpm biome ci`.
- **Done when** CI is green on all three OSes and a deliberately added GPL dependency fails `cargo deny`.

### Task 2: Schema v2 types and generators (M1.2)
- In `ark-schema`, add Rust structs for every v2 type in `plan.md`, deriving `Serialize`, `Deserialize`, `JsonSchema` and `TS`.
- Collections use `BTreeMap`/sorted `Vec`, so output order is stable.
- An `xtask`/build step writes `schema/architecture.schema.json` and `apps/desktop/src/gen/*.ts`. CI fails if the generated files are stale.
- **Done when** a hand-written sample graph round-trips through JSON (serialize, deserialize, compare) and validates against the generated schema.

### Task 3: Scanner and content-addressed store (M1.3)
- `ark-scan`:
  - walk with `ignore` (honouring .gitignore and a built-in vendor/minified denylist);
  - never follow symlinks outside the root;
  - per-file size cap of 1 MB, configurable;
  - blake3 per file, then a Merkle tree over sorted paths;
  - `diff(old_root, new_root) -> ChangeSet`.
- `ark-store`:
  - SQLite schema (`files`, `artifacts(key PRIMARY KEY, blob)`, `symbols`, `edges`, `chunks`, `chunks_fts`);
  - artifact key = `blake3 ‖ grammar@version ‖ extractor_version`;
  - a migration table.
- **Done when** a fixture indexed twice reports 0 re-parses on the second run, and editing one file reports exactly 1. A proptest checks that the Merkle root is independent of walk order.

### Task 4: Tree-sitter extraction and cAST chunking (M1.4)
- `ark-parse`: `.scm` queries for TS, JS, Python, Rust and Go extract `FileFacts`:
  - functions and methods (name, qualified name, visibility, params with optional types, return type, byte and line ranges);
  - imports and exports;
  - call sites (callee text plus receiver);
  - loop and branch nodes for metrics.
- `ark-chunk`: cAST, split recursively when a node exceeds 2000 non-whitespace characters, then greedily merge adjacent small siblings.
- Parsing is parallel (rayon), and results are sorted by path before storing.
- **Done when** insta snapshots of `FileFacts` and chunks pass for every language fixture, and a malformed file yields partial facts plus a diagnostic rather than a panic.

### Task 5: Metrics, tiers and the first end-to-end `architecture.json` (M1.5 + start of M1.8)
- `ark-metrics`:
  - cyclomatic and cognitive complexity from per-language decision-node tables;
  - max nesting;
  - the Big-O rule engine (loop-bound classes, stride detection, recursion fan-out, library cost table), with a `basis` rule id on every label;
  - memory flags.
- `ark-structure`: tier rules from `ontology/tiers.yaml`, each with an id that is recorded as evidence. Clusters are directory-based placeholders until M1.8 adds Leiden.
- `ark-cli index <path> --out architecture.json`.
- **Done when**:
  - the TodoMVC and FastAPI fixtures produce goldens;
  - CI re-indexes each fixture twice, plus once with file enumeration shuffled, and the outputs are byte-identical;
  - Big-O labels match a 50-function hand-labelled set at ≥ 90%, with CodeComplex samples added later.

### Next (M1.6 to M1.9)
- **M1.6** SCIP import (`scip` crate) and sidecar detection (rust-analyzer, scip-go). Edge provenance and confidence. Unresolved `candidates[]`.
- **M1.7** Ontology v0: ≥ 60 YAML rules across OS, network, cloud and data, each with a fixture test.
- **M1.8** Guided Leiden with a fixed seed and anchor nodes for routers, data stores and tests. Tier ⇄ cluster consistency check.
- **M1.9** Golden suite over 5 fixture repos. A performance budget of 50k LOC in under 10 s cold and under 1 s warm on a laptop (target, to be measured).
- **Research checkpoint** before Phase 2: re-check React Flow 13 docs and ELK worker setup, and update `plan.md`.
