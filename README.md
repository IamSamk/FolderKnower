# Architekton

Drop a repository folder and see how it is built. Architekton reads the code **offline**, works out what each part *does* (frontend, gateway/middleware, backend, workers, data) instead of trusting folder names, and draws the whole system as a zoomable, animated map: from system tiers down to subsystems, files, call graphs and individual functions with their Big-O and the OS / network / database concepts they use. Pick a button in the UI map and follow it, hop by hop, through the API call, the route handler and the ORM down to the table in the ER diagram.

Every edge comes from static analysis (tree-sitter) and carries provenance and confidence. An optional local model can narrate what you selected, but it never draws edges and nothing leaves your machine.

## Install

Download the installer for your OS from the [latest release](https://github.com/IamSamk/Architekton/releases/latest):

| OS | File | Notes |
|---|---|---|
| Windows 10/11 | `Architekton_<version>_x64-setup.exe` (or `.msi`) | SmartScreen may warn because the app is not code-signed yet: **More info → Run anyway**. |
| macOS (Apple Silicon + Intel) | `Architekton_<version>_universal.dmg` | Not notarized yet: right-click the app → **Open** the first time, or run `xattr -cr /Applications/Architekton.app`. |
| Linux | `.AppImage`, `.deb` or `.rpm` | AppImage: `chmod +x Architekton_*.AppImage && ./Architekton_*.AppImage`. |

## Use it

1. Open Architekton and **drop a repository folder** on the window (or click to choose one). Try **Explore the demo full-stack app** first.
2. Watch it narrate what it finds: files scanned, routes linked, and what kind of project it thinks this is.
3. **Architecture** view, five zoom levels: *System tiers → Subsystems → Files → Functions & calls → Function detail*.
   - Click to inspect, **double-click to zoom in**, use the breadcrumb to zoom out.
   - Click a glowing arrow or a dashed "portal" node to **travel along it** into another tier or file.
4. **UI map**: turn on the **pointer** (top right), hover any button/input like Chrome DevTools, click it, and the trace shows the path: element → handler → API client → `POST /route` → handler → table.
5. **Data (ER)**: tables, columns, PK/FK, relations, and the Pydantic/zod/DTO schemas that mirror each table.
6. The **Inspector** shows details, highlighted source (**Code** tab) and an optional **Explain ✦** tab.
7. **Help me classify**: when the rules are unsure about a folder, Architekton asks. Your answer re-runs the analysis.
8. `Ctrl/Cmd + K` jumps to any file, function, route or table. The download button exports `architecture.json`.

Languages: TypeScript/TSX, JavaScript/JSX, Python, Rust, Go. Frameworks recognised include React/Next.js/Vue/Svelte, Express/Fastify/Hono/NestJS, FastAPI/Flask/Django, Axum/Actix, Gin/Echo/net/http, SQLAlchemy/SQLModel/Django ORM/Pydantic, Prisma/TypeORM/Sequelize/zod, sqlx/Diesel/GORM, SQL DDL, Celery/BullMQ.

### Optional: local model (SLM)

The app works fully without a model. If you want plain-language explanations and a second opinion on ambiguous folders:

```sh
# Ollama (https://ollama.com)
ollama pull qwen2.5-coder:3b
```

or run llama.cpp's `llama-server` (port 8080) or LM Studio (port 1234). Architekton detects a server on `127.0.0.1`, sends it only the facts of the selected subgraph, forces JSON output whose citations must be ids from that subgraph, and drops any sentence that cites something else.

## Command line

Each release also ships `architekton-cli-*` archives:

```sh
architekton index path/to/repo --out architecture.json   # deterministic, incremental (SQLite cache)
architekton index path/to/repo --set scripts/=Workers     # force a tier for a path prefix
architekton schema                                        # JSON Schema of architecture.json
architekton ontology                                      # the Systems Ontology catalogue
```

## Build from source

Prerequisites: Rust (the pinned toolchain installs itself via `rust-toolchain.toml`), Node.js LTS, and the [Tauri 2 system dependencies](https://v2.tauri.app/start/prerequisites/) for your OS (WebView2 on Windows is preinstalled on 10/11; on Linux install `libwebkit2gtk-4.1-dev librsvg2-dev`).

```sh
git clone https://github.com/IamSamk/Architekton.git
cd Architekton
npx pnpm@12.21.0 install          # or: corepack enable && pnpm install
cd apps/desktop
npx tauri dev                      # run the app with hot reload
npx tauri build                    # produce installers in target/release/bundle/
```

Run only the UI in a browser (demo data, no Rust needed): `cd apps/desktop && npx vite`, then open http://localhost:1420.

## How it works

`crates/` holds the Rust pipeline:

| Crate | Job |
|---|---|
| `ark-scan` | gitignore-aware walk, blake3 hashes, order-independent Merkle root |
| `ark-store` | content-addressed SQLite cache: an unchanged file is never re-parsed |
| `ark-parse` | tree-sitter extraction: functions, params, imports, classes, routes, HTTP calls, JSX elements |
| `ark-metrics` | cyclomatic/cognitive complexity, nesting, rule-based Big-O with the rule ids that produced it |
| `ark-link` | import resolution, call edges with provenance + confidence, route handlers, UI handlers, frontend→backend HTTP edges |
| `ark-structure` | semantic tier scoring + import-graph propagation, deterministic Louvain clustering, ER models, questions |
| `ark-ontology` | 60 OS / network / cloud / data rules with teaching text (`ontology/rules.yaml`) |
| `ark-core`, `ark-cli` | the pipeline and the `architekton` binary |

`apps/desktop` is the Tauri 2 + React 19 app (React Flow + ELK layout, Motion animations). The design and research behind it are in [`plan.md`](plan.md), [`docs/phase1-checklist.md`](docs/phase1-checklist.md) and [`docs/research/synthesis.md`](docs/research/synthesis.md).

## Development

```sh
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test
cargo deny check                     # license allowlist + advisories
bash scripts/deny-negative-test.sh   # proves an LGPL crate is rejected
npx pnpm@12.21.0 install && npx pnpm@12.21.0 lint   # Biome
UPDATE_GOLDEN=1 cargo test -p ark-core golden        # refresh the fixture snapshot
```

Pushing a tag `v*` runs `.github/workflows/release.yml`, which builds Windows, macOS (universal) and Linux installers plus CLI archives and publishes them as a GitHub Release.
