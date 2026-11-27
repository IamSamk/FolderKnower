# Changelog

Notes for each version are shown in the in-app update pop-up and on the website's releases page.

## [Unreleased]

### Checked on large real repositories
- Flask-RESTX and Flask-RESTful class routes are read (Dify's API: 53 routes found before, 1,499 now). actix `web::resource` routes with `web::scope` prefixes are read.
- The container list is truer: copies of a compose file no longer hide the real one, services with no catalog image are told apart by start command, `MODE` and name, editor-tooling Dockerfiles and test doubles are left out of the headline.
- The headline says "AI system" only when the AI parts are a real share of the repository.
- Protobuf stubs, vendored and third-party folders are skipped. Source in languages without a parser (Java, C#, Kotlin, ...) is now reported as "Not read" instead of being silent.
- Parts with the same name are told apart by their folders instead of a counter.
- Findings and open gaps: docs/design/stress-test-findings.md.

## [0.14.1] - 2026-11-27

### Every search path is shown
- A system with several searches now shows each one: vector search, each keyword search, and graph search, all feeding the fusion step. Nothing is picked over another.
- Explain asks about the selection by name. A 32 GB computer is offered the 7B suggestion.
- Hand-made agent loops, tool registries and `/chat/completions` clients are recognised, and the application's own model client wins over scripts and browser code.

## [0.14.0] - 2026-11-27

### AI pipelines
- Hybrid search shows both lanes side by side: keyword search (BM25) and dense vector search each lead into rank fusion, instead of one lane being picked.
- Graph-assisted retrieval is found: keyword hits that expand through an entity graph (entities, aliases, mentions, relations) show as their own step.
- Evidence checks ("is this enough to answer?") show as a decision with a loop back to retrieval, so adaptive retrieval reads as a loop, not a straight line.
- Rerankers such as Qwen3-Reranker and mxbai-rerank, PDF/OCR loaders, chunk size, and local model files (GGUF) are named in the pipeline.
- The whole project's source is read for pipeline steps, not only config files. Tests, docs and prose are skipped.
- A web page calling its own backend (`/v1/chat/completions`, `/api/chat`) is no longer shown as the language model.

### Export
- New `fknow export <repo>` and a "Copy summary" button in the app: a Markdown digest with size, tiers, Mermaid diagrams of the tiers, containers and AI pipeline, routes, data models and health, ready to paste into a README, a PR or a chat.

### Tiers
- Python, Go and Rust code that calls other services over HTTP is no longer sorted into the frontend. A FastAPI service is shown as the API, not the web page.

### Example runs
- Buttons that call a small helper such as `req("/api/ask")` now link to the backend route, so the run follows the click all the way through the pipeline.
- Pipeline steps in a run follow the data: both search lanes, then fusion, reranking, the evidence check and the answer.

### App
- Update pop-up: long release notes scroll inside the card, and the app checks again when you come back to the window.

## [0.13.1] - 2026-11-26

### App
- Update pop-up checks as soon as the app opens, retries if the network is not ready, and checks again when you return to the window.
- Local AI settings show what the models need and compare it with this computer. Installed models get a "Change file" button.
- "Explain" starts from a question about the selected item and its place in the code.

### Detection
- Vector search written as SQL (pgvector: `<=>`, HNSW indexes) and rank fusion (RRF) in code or config now appear in the AI pipeline next to the embedding model.
- Postgres full-text search with a "BM25-style" comment is named Postgres full-text, not BM25.

## [0.13.0] - 2026-11-26

### New name: FolderKnower
- Architekton is now **FolderKnower**. The app, installer, website and docs use the new name. The command line tool is now `fknow`.
- Quick install: `curl -fsSL folderknower.vercel.app/i | sh`, or on Windows `irm folderknower.vercel.app/i.ps1 | iex`.
- Updates now come from github.com/IamSamk/FolderKnower. Your settings and cache stay.

## [0.12.0] - 2026-11-26

### Limits from the code
- Named numbers in your code that bound behaviour (`CHUNK_SIZE = 800`, `HISTORY_TURNS = 6`, `ANSWER_TTL = 3600`) are read with the line that defines them, and shown in the Memory and context flow under "Limits written in the code".
- The Flow map key now draws small swatches instead of long bars.

## [0.11.0] - 2026-11-26

### Live shows only your project
- The Live tab no longer shows the whole computer. No system memory card, no machine-wide process list. It follows one process tree: the project you ran or attached to by port, with its children, containers and model servers it owns.
- Opening a folder inside a bigger project analyses only that folder.

## [0.10.0] - 2026-11-26

### Run your project
- The Live tab can now launch your project: it finds start commands (docker compose, npm/pnpm/yarn scripts, cargo run, python), shows the exact command and folder, and runs it only after you confirm. It tails the log, measures the whole process tree and each docker container live, and has a Stop button.
- Attach by port: lists what is listening on this computer and measures the process that owns the port.
- Everything in this panel is measured on this computer, not estimated. The browser demo stays simulated and says so.

### Diagrams
- The Flow map has a key explaining box kinds and arrows. An architecture diagram checklist (C4, ISO 42010, UML, cloud styles) is in docs/.

## [0.9.0] - 2026-11-26

### Workflow diagrams
- New **Flow** map in the System view: boxes in phases with arrows, like a hand-drawn architecture sheet. Three kinds: Request (one action step by step), AI pipeline (index once, then search, fuse, rerank, answer) and Memory and context (what is kept, what one model call is made of, caches and the KV cache). Every box links to the code that proves it.
- Simple, Standard and Deep control how much each box says.

### Fixes
- A container built `FROM nginx`, `FROM postgres` or from a llama.cpp image is now named for what it is, not guessed from the files in its folder.
- Code map edge labels show when zoomed in or selected, so they stop piling up.

## [0.8.0] - 2026-11-26

### Example run as a walkthrough
- Playing an example run now zooms onto each step, pulls back while the request travels to the next one, then zooms in again. Simple stays a little further back; Standard and Deep go right onto the function and show the few lines of code that ran.
- Code map stays the place to browse the structure; its Play button opens this walkthrough instead of drawing a second one.

### Fixes
- Screen transitions can no longer leave a half-drawn dark shape over the UI map.
- Custom BM25 code and BGE-M3 (BGEM3FlagModel, dense plus sparse) are now recognized in the AI pipeline, also when the model id only sits in a settings file or a compose command, and Postgres full-text (ts_rank) next to other AI parts counts as the keyword half of hybrid search.
- `re.search()` and a game's `embed()` no longer make a project look like RAG; search and embed calls need a vector or embedding library in the file.
- Whisper, Gemini, Ollama / LM Studio / llama.cpp URLs and text-to-speech are now listed as AI parts.
- Fewer false problems: `python-frontmatter`-style package names, `cv2` with any opencv package, `src` / `app` as local code, dynamic `import()` with a template string, `&` in JSX text, Dockerfile entry files under a copied sub-folder, Next standalone `server.js`.
- Top bar stays on one line at laptop widths; a big data diagram opens on its busiest table instead of at 16%.

## [0.7.0] - 2026-11-26

### FolderKnower explains itself
- The main example on the welcome screen is now FolderKnower's own code: a Tauri desktop app whose React window calls 21 Rust commands. The other examples (web app, AI app, broken project) sit in a quiet row underneath.
- The example is rebuilt from the repository at every release, and its sources open in the code view.

### Desktop apps
- Tauri `#[tauri::command]` functions and Electron `ipcMain.handle` channels are read as the app's native commands, and every `invoke("...")` / `ipcRenderer.invoke("...")` is linked from the window to the Rust or main-process code it runs.
- A call to a command that does not exist is reported in Problems.
- New project kind "Desktop app", with its own words in the System view: app window, native side, saved data, commands instead of routes.
- Features for desktop apps come from command names: `ai_install`, `ai_status`, `ai_remove` become one "AI" feature; `check_update` and `install_update` become "Update".
- Rust workspace crates resolve by their Cargo.toml name (`use ark_core::...`), so a Tauri app's crates land with its native side instead of in Shared.

### Memory map for native code
- GGUF model files kept on disk, model weights loaded by a local runtime process (RAM, or GPU memory when it offloads layers), the app data and cache folders, SQLite files (and in-memory SQLite), and state shared across commands.

### Use models you already have
- Each local AI part has "Use a file I have": point at a .gguf model or a llama-server program from LM Studio, Ollama, Hugging Face or your own llama.cpp build instead of downloading it.
- "Look on this computer" searches the usual folders and PATH and lists what it finds.
- Every file is checked first: it must be a GGUF model; the exact pinned model is confirmed by SHA-256 (Laya must be the exact file); embedding-only models are refused; llama-server must answer `--version`. A different chat model works for explanations and is labelled as not the tested one.
- Your files are only read. "Stop using" forgets the link and never deletes the file. The setup screen shows where downloads are saved.

### Fewer false alarms
- Sample projects in `fixtures/`, `testdata/` and `__fixtures__/` folders are no longer mapped as part of the app.
- Valid code the parser grammars miss (`export type * from`, a bare `&` in JSX text) is no longer reported as a syntax error.
- Imports of CSS, JSON and other asset files that exist are no longer reported as missing.
- Groups are named after their most specific folder (`ark-core`, not `crates`), and a large flat folder stays one group.

## [0.6.0] - 2026-11-26

### Depth toggle (System view)
- Simple: plain words for non-technical readers. Each box says what it is for ("Short-term memory: remembers recent things so repeats are instant") and which features it helps with. No code, ports or images.
- Standard: containers, the code inside them and the AI steps, as before.
- Deep: every file in each container, the packages that code imports, and the data each container keeps (GPU memory, RAM, disk) with how long it lives.

### Features
- The project is split into features by route family (`/todos/*`, `/auth/*`, `/ask`, `/ingest`). Each feature lists its routes, screens, own and shared files, tables, containers, AI steps and tech stack.
- Pick a feature to get its own diagram: everything else fades, code rows and AI steps narrow to that feature, and the example runs follow.
- Indexing and asking stay apart: an `/ingest` route gets the loader, splitter and embedder; `/ask` gets the agent, retriever and reranker it reaches through its imports.

### Memory map (new diagram next to System)
- Columns for GPU memory, RAM, disk, the user's browser and outside providers. Every entry shows who holds it (which container or process), how long it lives (per request, per session, until restart, kept) and what bounds it (maxmemory and eviction policy, TTL, list trim, keep_alive, gpu_memory_utilization, container limit).
- Covers model weights and KV cache (Ollama, vLLM, llama.cpp, transformers), vector indexes in RAM and their collections on volumes, BM25 indexes, embedding and reranker models, Redis history and caches, conversation memory, database tables, prompt caching at the provider, and localStorage / sessionStorage / IndexedDB in the browser.
- Each line links to the code or compose line it rests on.

### Broken code
- New Problems button lists what stops the code from running, found without running it: syntax errors with the line, relative imports of files that do not exist, packages imported but missing from requirements/pyproject/package.json, frontend calls to routes the backend never declares, compose start commands that name a missing file, and GPU code in a container with no GPU reserved.
- Analysis keeps going on broken files: functions before a syntax error still show up in the map, and broken files are marked in the System view.
- `fknow check <repo>` prints the same list and exits 1 when an error is found, for CI.
- New "Try a broken project" demo.

## [0.5.0] - 2026-11-26

### System (new first tab)
- Reads docker-compose files, Dockerfiles and nginx configs. Each container is a box with its image, ports, memory limit and GPU, and your code sits inside the container that builds it. The links between boxes come from env URLs, depends_on or proxy_pass.
- Memory split: how the compose limits divide your RAM, plus Redis maxmemory, Ollama keep-alive and parallelism, and vLLM GPU share.
- Example run: press Play, the screen preview types a question and presses the button, and the camera follows it through the proxy, the route, your memory/cache code, the tables, the AI pipeline and the containers it calls, then back. Each step shows the line that proves it.
- Without a compose file, the boxes are your code tiers.

### Grounded AI tool
- `fknow run <repo>` returns the same example run as JSON (or `--text`) so a model can narrate it using only proven hops.

### UI map
- Screens on the left, where each element goes on the right (backend route with its tables and container, browser-only, or not wired), joined by wires. "Play run" jumps to the System view.

### DSA tools (new sidebar)
- Structures in your code, structures hidden inside your tools (vector index graph, inverted index, top-k heap, agent state graph, ring buffer, LRU, KV block table, route trie, B-tree), real-life problems mapped to structures and to functions in your project, and slow spots.

### Recognition
- Hybrid search: sparse vectors (SPLADE, fastembed), Qdrant prefetch + RRF, Weaviate hybrid, Postgres full-text. Redis capped history and TTL caches.

## [0.4.0] - 2026-11-26

### How it works (new tab)
- AI pipeline: recognizes RAG, hybrid search, agents and agentic RAG, and draws them in lanes with the real settings from your code (chunk size, overlap, embedding model, top-k, reranker, model, context window). "Play a request" walks a question through every step.
- Data structures: finds stacks, queues, heaps, trees, graphs, recursion, memoization and binary search, and shows each one as a small animation.
- Memory and caching: names the techniques your code uses (PagedAttention, prefix caching, KV cache quantization, attention sinks, prompt caching), with a transformer walk-through and a KV cache calculator.
- Libraries: about 58 known AI and data libraries explained in one line each, plus Google OKF knowledge folders.

### Workflow button
- At every zoom level: the clean path through a function in green, decisions in amber, possible error exits in red, with a step-by-step player.

### Live (new tab)
- Memory, every CPU core, top processes with trends, one process's memory explained, its threads, and local model servers (Ollama, llama-server, vLLM) with KV cache usage. No admin rights needed.

### Demo
- New AI demo project on the welcome screen: RAG + agent + a tiny GPT.

## [0.3.2] - 2026-11-25

### Updates
- New "Check for updates" button in the top bar. Tells you when you are already on the latest version.
- This release is the first one delivered to 0.3.1 through the new update pop-up.

### Website
- Download buttons always point at the newest release, with file sizes, read live from GitHub.
- New Releases page with every version's notes and installers.
- New Code signing policy page explaining what is signed and how to verify a download.

## [0.3.1] - 2026-11-25

### Updates
- Auto-update now actually ships: releases include a signed `latest.json` and per-platform signatures, so installed apps find new versions on their own.
- New update pop-up with release notes, a real download progress bar, Later and Skip this version.
- Checks 4 seconds after launch and every 6 hours. Packages are signature-checked before install.
- Windows installs updates in passive mode (progress only, no wizard).

### Trust
- Every release lists `SHA256SUMS.txt` and carries GitHub build-provenance attestations.
- Release pipeline ready for free Windows signing (SignPath Foundation) and GPG-signed Linux AppImages.
