# M9: Architekton reads itself, desktop apps, models you already have

## Why

- The best example of what Architekton shows is Architekton. Running it on its own
  repository also exposed what it misread: a Tauri app came out as "Frontend app with
  93 components", its Rust crates landed in Shared, its test fixtures were mapped as
  part of the app, and valid TypeScript was reported as broken.
- People who run local models already have the files. Downloading another copy wastes
  half a gigabyte and minutes.

## Desktop apps

- `ark-parse`: `#[tauri::command] fn name` becomes `RouteFact { method: "IPC", path: name,
  framework: "tauri" }`. `invoke("name")` (bare, or on `core` / `tauri`) and
  `ipcRenderer.invoke|send("ch")` become `HttpCallFact { method: "IPC" }`.
  `ipcMain.handle|on("ch", fn)` is an Electron route. Names must look like identifiers.
- `ark-link`: IPC calls match routes by exact name and produce `Protocol::Ipc`
  cross-tier edges; HTTP matching ignores IPC routes. An IPC call with no command, when the
  project declares any, is a dead call.
- Rust workspace crates: `Cargo.toml` `[lib] name` or `[package] name` (with `-` as `_`)
  maps `use ark_core::x` to `crates/ark-core/src/x.rs`, falling back to `lib.rs`.
- Tiers: once a Rust/Go file is Backend, weakly classified Rust/Go files in Shared move to
  Backend (`tier.be.native-code`): native code cannot run in the browser.
- `ProjectKind::Desktop` when there is a frontend and IPC routes. Features group commands
  by a shared first word, then a shared last word, ignoring a plural `s`.

## Memory map (native)

`memory::native` reads call facts: `.gguf` string literals (disk), `Command::new` with
`.arg("-m")` (weights in a child runtime process; GPU memory when `-ngl` is passed),
`app_data_dir` / `getPath("userData")`, `cache_dir`, `Connection::open` /
`sqlite3.connect`, `Connection::open_in_memory`, and Tauri `.manage(state)`. In a desktop
app, native code is held by "the app's native process".

## Using files the user already has

`ai_link(id, path)` checks, then stores `{path, check, modelName}` in `ai/links.json`:

| Part | Check | Result |
| --- | --- | --- |
| runtime | a `llama-server` in the file or folder that answers `--version` | compatible |
| decider | GGUF magic, then the pinned size and SHA-256 | exact, else refused |
| narrator | GGUF magic, then the pinned SHA-256 | exact |
| narrator | another GGUF chat model (not an embedding architecture, under 24 GB) | compatible |

`ai_find` walks `~/.cache/huggingface/hub`, `~/.lmstudio/models`,
`~/.cache/lm-studio/models`, `~/.ollama/models/blobs` (blob names are the SHA-256, so the
exact model is recognised without hashing), `~/.cache/llama.cpp`, `~/models`,
`~/Downloads`, and `PATH` for `llama-server`. It never links anything by itself.

A linked file is only read. Removing it forgets the link.

## Self example

`pnpm example:self` (in `apps/desktop`) writes `src/examples/architekton.json`; the release
workflow regenerates it at the release commit. The welcome screen loads it, and this
repository's sources, only when opened.
