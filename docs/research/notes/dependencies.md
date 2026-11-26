# FolderKnower Dependencies: Versions, Licenses, Caveats (fetched 2026-11-21)

Fetch method and date: all registry values below were fetched on **2026-11-21** from the crates.io API (`https://crates.io/api/v1/crates/<name>`), the npm registry (`https://registry.npmjs.org/<pkg>`), the Go module proxy (`proxy.golang.org`), Maven Central (`repo1.maven.org`) and PyPI (`pypi.org/pypi/<pkg>/json`). **The `gh api` calls failed** (HTTP 403, "GitHub access to this repository is not enabled for this session"), and so did direct `curl` to api.github.com and github.com (HTTP 403). GitHub data below comes from WebFetch of the github.com HTML pages. Those pages may be cached or stale, and they sometimes disagree with the registries (noted where it happens). Release dates are the publish dates of the listed version on its registry.

## Q1: Verified dependency table: latest versions, licenses, release dates, maintenance

### Takeaway
Nearly every dependency had a release in the last 3 months and uses MIT or Apache-2.0. The exceptions:
- Stale: rust-code-analysis (2023), roughjs (2023), ferrisetw (2024), sled stable (2021).
- License flags: elkjs (EPL-2.0 OR GPL-3.0-or-later), rust-code-analysis (MPL-2.0), notify (CC0-1.0).

Majors released in 2026: TypeScript 7 (Go native), pnpm 12, Vite 8, Vitest 5, ESLint 10, @xyflow/react 13, redb 5, notify 9. Tauri 3 is in alpha.

### Cited Findings

#### Rust crates (crates.io API, max_stable_version)
| Crate | Latest stable | Pre-release | License | Released | MSRV | Notes |
|---|---|---|---|---|---|---|
| tauri | 2.12.7 | 3.0.0-alpha.6 | Apache-2.0 OR MIT | 2026-11-18 | 1.90 | active |
| tauri-build | 2.7.6 | 3.0.0-alpha.4 | Apache-2.0 OR MIT | 2026-11-18 | 1.90 | |
| tauri-plugin-shell | 2.6.0 | 3.0.0-alpha.3 | Apache-2.0 OR MIT | 2026-10-28 | 1.90 | repo tauri-apps/plugins-workspace |
| tauri-plugin-fs | 2.7.1 | 3.0.0-alpha.3 | Apache-2.0 OR MIT | 2026-10-14 | 1.90 | |
| tauri-plugin-dialog | 2.8.4 | 3.0.0-alpha.4 | Apache-2.0 OR MIT | 2026-10-28 | 1.90 | |
| tree-sitter | 0.27.2 | — | MIT | 2026-11-07 | 1.90 | ABI 15, min compatible 13 |
| tree-sitter-tags | 0.27.2 | — | MIT | 2026-11-07 | 1.90 | |
| tree-sitter-language-pack | 1.25.2 | — | MIT | 2026-11-15 | — | repo xberg-io/tree-sitter-language-pack; "compiled parsers for 371 languages" |
| tree-sitter-rust | 0.24.2 | — | MIT | 2026-03-27 | — | parser ABI 15 |
| tree-sitter-python | 0.25.0 | — | MIT | 2025-09-11 | — | parser ABI 15 |
| tree-sitter-javascript | 0.25.0 | — | MIT | 2025-09-01 | — | |
| tree-sitter-typescript | 0.23.2 | — | MIT | 2024-11-11 | — | parser ABI 14, about 2 years old |
| tree-sitter-go | 0.25.0 | — | MIT | 2025-08-29 | — | |
| tree-sitter-java | 0.23.5 | — | MIT | 2024-12-21 | — | parser ABI 14 |
| scip | 0.10.0 | — | Apache-2.0 | 2026-09-03 | 1.81.0 | crates.io repo field now points to **scip-code/scip** |
| ast-grep-core | 0.51.3 | — | MIT | 2026-11-16 | 1.90.0 | depends on tree-sitter ^0.27.0 |
| ast-grep-language | 0.51.3 | — | MIT | 2026-11-16 | 1.90.0 | |
| aya | 0.16.0 | — | MIT OR Apache-2.0 | 2026-10-28 | 1.98.0 | |
| aya-ebpf | 0.3.0 | — | MIT OR Apache-2.0 | 2026-10-31 | 1.98.0 | |
| aya-log | 0.4.0 | — | MIT OR Apache-2.0 | 2026-11-01 | 1.98.0 | |
| aya-build | 0.2.1 | — | MIT OR Apache-2.0 | 2026-10-16 | 1.98.0 | |
| ferrisetw | 1.2.0 | — | MIT OR Apache-2.0 | 2024-06-27 | — | no release in about 29 months |
| krabsetw | **not on crates.io** (API returned no crate) | | | | | C++ library, see below |
| windows | 0.62.2 | — | MIT OR Apache-2.0 | 2025-10-06 | 1.82 | |
| sysinfo | 0.40.4 | — | MIT | 2026-11-16 | 1.95 | |
| console-subscriber | 0.5.0 | — | MIT | 2025-10-30 | 1.74.0 | |
| tokio | 1.54.1 | — | MIT | 2026-10-21 | 1.85 | |
| rayon | 1.12.0 | — | MIT OR Apache-2.0 | 2026-04-14 | 1.80 | |
| ignore | 0.4.34 | — | Unlicense OR MIT | 2026-10-27 | 1.88 | |
| notify | 9.0.0 | — | **CC0-1.0** | 2026-11-21 (same day as fetch) | 1.91 | brand-new major, so 8.x may be safer at first |
| gix | 0.90.0 | — | MIT OR Apache-2.0 | 2026-10-22 | 1.88 | still 0.x |
| blake3 | 1.8.7 | — | CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception | 2026-08-20 | — | |
| rusqlite | 0.43.0 | — | MIT | 2026-11-15 | 1.89.0 | |
| redb | 5.0.0 | — | MIT OR Apache-2.0 | 2026-11-18 | 1.90 | new major 3 days before fetch |
| sled | 0.34.7 | 1.0.0-alpha.124 | MIT/Apache-2.0 | 2021-09-12 (crate updated 2024-10-11) | — | **stable line stagnant**; avoid |
| petgraph | 0.8.3 | — | MIT OR Apache-2.0 | 2025-09-30 | 1.64 | |
| serde | 1.0.229 | — | MIT OR Apache-2.0 | 2026-07-18 | 1.56 | |
| schemars | 1.2.3 | — | MIT | 2026-10-12 | 1.74 | |
| specta | 1.0.5 (2023-07-17) | **2.0.0-rc.28** (crate updated 2026-11-05) | MIT | | | v2 is still RC |
| tauri-specta | 1.0.2 (2023-05-18) | **2.0.0-rc.29** (updated 2026-11-13) | MIT | | | Tauri v2 support exists only in the RC line |
| ts-rs | 12.0.1 | — | MIT | 2026-01-31 | 1.78.0 | |
| candle-core | 0.11.1 | — | MIT OR Apache-2.0 | 2026-10-31 | — | |
| candle-transformers | 0.11.1 | — | MIT OR Apache-2.0 | 2026-10-31 | — | |
| llama-cpp-2 | 0.1.161 | — | MIT OR Apache-2.0 | 2026-11-03 | — | repo utilityai/llama-cpp-rs |
| tokenizers | 0.23.2 | 1.0.0-rc.5 (updated 2026-11-20) | Apache-2.0 | 2026-09-03 | — | |
| mach2 | 0.7.0 | — | BSD-2-Clause OR MIT OR Apache-2.0 | 2026-08-30 | — | |
| libproc | 0.15.0 | — | MIT | 2026-11-04 | 1.85 | |
| pprof | 0.15.0 | — | Apache-2.0 | 2025-05-27 | 1.74.0 | |
| rust-code-analysis | 0.0.25 | — | **MPL-2.0** | **2023-01-13** | — | "Tool to compute and export code metrics"; no crates release in about 3 years |

Source: [crates.io API](https://crates.io/api/v1/crates/tauri). The same endpoint pattern was used for every crate; raw output is saved in the scratchpad.

#### npm packages (registry dist-tags.latest)
| Package | Latest | License | Released | Other tags |
|---|---|---|---|---|
| @xyflow/react | 13.0.2 | MIT | 2026-11-17 | 13.0.0 on 2026-11-16; last 12.x = 12.12.1 (2026-11-02) |
| elkjs | 0.12.1 | **EPL-2.0 OR GPL-3.0-or-later** | 2026-10-28 | |
| roughjs | 4.6.6 | MIT | **2023-11-20** | stale |
| @excalidraw/excalidraw | 0.18.1 | MIT | 2026-04-20 | |
| react / react-dom | 19.3.1 | MIT | 2026-10-14 | canary 19.3.0-canary-…-20261119 |
| typescript | **7.0.2** | Apache-2.0 | 2026-07-08 | rc 7.0.1-rc (2026-06-18); beta 7.1.0-beta; next 7.1.0-dev.20261121.1; last 6.x = 6.0.3 (2026-04-16) |
| @typescript/native-preview | 7.0.0-dev.20260707.2 | Apache-2.0 | 2026-07-07 | superseded by `typescript` 7 |
| vite | 8.3.8 | MIT | 2026-11-11 | beta 8.4.0-beta.1 |
| @vitejs/plugin-react | 6.3.0 | MIT | 2026-11-09 | |
| @tauri-apps/api | 2.12.5 | Apache-2.0 OR MIT | 2026-11-16 | next 3.0.0-alpha.3 |
| @tauri-apps/cli | 2.12.6 | Apache-2.0 OR MIT | 2026-11-18 | next 3.0.0-alpha.6 |
| @tauri-apps/plugin-shell / -fs / -dialog | 2.6.0 / 2.7.1 / 2.8.4 | MIT OR Apache-2.0 | 2026-10-28 / 10-14 / 10-28 | match Rust plugin versions |
| zustand | 5.1.0 | MIT | 2026-11-06 | |
| @tanstack/react-query | 5.108.2 | MIT | 2026-11-20 | |
| tailwindcss / @tailwindcss/vite | 4.3.5 | MIT | 2026-11-06 | |
| vitest | 5.1.1 | MIT | 2026-11-18 | |
| @biomejs/biome | 2.6.8 | MIT OR Apache-2.0 | 2026-11-15 | |
| eslint | 10.15.0 | MIT | 2026-11-13 | |
| pnpm | **12.21.0** | MIT | 2026-11-13 | next-12 12.22.0; latest-11 11.28.13; latest-10 10.34.9. 11.0.0 came out 2026-04-28 and 12.0.0 on 2026-08-26 |
| web-tree-sitter | 0.27.2 | MIT | 2026-11-07 | |
| @ast-grep/napi | 0.51.3 | MIT | 2026-11-16 | |
| @sourcegraph/scip-typescript | 0.6.2 | Apache-2.0 | 2026-11-10 | |
| @sourcegraph/scip-python | 0.6.8 | MIT | 2026-10-22 | |

Source: [npm registry](https://registry.npmjs.org/@xyflow/react). The same pattern was used for each package.

#### Other registries and GitHub
- scip-go: latest v0.2.10, published 2026-11-20 — [Go proxy](https://proxy.golang.org/github.com/sourcegraph/scip-go/@latest)
- scip-java (`com.sourcegraph:scip-java_2.13`): latest 0.12.3, metadata updated 2026-04-02 — [Maven Central](https://repo1.maven.org/maven2/com/sourcegraph/scip-java_2.13/maven-metadata.xml)
- lizard (Python): 1.26.1, MIT, uploaded 2026-11-16, so it is actively maintained — [PyPI](https://pypi.org/pypi/lizard/json)
- tree-sitter GitHub releases page shows **v0.27.1** marked "Latest" (08 Oct), while crates.io and npm have 0.27.2 (2026-11-07). The page may be cached, or 0.27.2 may not have a GitHub release yet. Treat the registries as authoritative — [GitHub](https://github.com/tree-sitter/tree-sitter/releases/latest)
- pnpm GitHub releases page (via WebFetch) showed 12.3.4 as "Latest" (04 Sep), which conflicts with npm's 12.21.0 (2026-11-13). The fetched page looks stale. Treat npm as authoritative — [GitHub](https://github.com/pnpm/pnpm/releases)
- xyflow GitHub releases page (via WebFetch) showed only 12.12.0. npm has 13.0.2. The page looks stale — [GitHub](https://github.com/xyflow/xyflow/releases)
- krabsetw: C++ library. Microsoft.O365.Security.Native.ETW is a C++/CLI .NET wrapper around it. The page says "No releases published"; distribution is through NuGet packages. Supports x64 and ARM64 only, Windows 7 / 2008R2 and later. "used in production by the Office 365 Security team". The license was not visible on the fetched page — [GitHub](https://github.com/microsoft/krabsetw)
- scip spec/CLI repo: `scip-code/scip` (Apache-2.0). It lists these indexers: scip-java (sourcegraph), scip-typescript (sourcegraph), rust-analyzer (rust-lang), scip-clang, scip-ruby, scip-python, scip-dotnet (sourcegraph), scip-dart (Workiva), scip-php. The page does not explain the move from sourcegraph/scip — [GitHub](https://github.com/scip-code/scip)

### Inferences
- The scip Rust crate's repository field now points to scip-code/scip, and the GitHub page lives there. SCIP has apparently moved from the sourcegraph org to a neutral `scip-code` org. The indexers still live under sourcegraph/*. The reason for the move is not documented on the page.
- specta and tauri-specta are still at 2.0 RC, and RC releases can break APIs. ts-rs 12 is the stable alternative for generating TypeScript types (it gives types only, not typed command bindings).
- ferrisetw (last release 2024-06) is the only pure-Rust ETW consumer crate, and it looks lightly maintained. A thin wrapper over the `windows` crate's ETW APIs (`Win32_System_Diagnostics_Etw` feature) is a lower-risk fallback.
- sled: use redb (pure Rust, just reached 5.0) or rusqlite instead.

### Gaps
- No GitHub data came through the API: stars, archived flags, last-commit dates and authoritative "latest release" tags are missing for tree-sitter, ast-grep, aya, ferrisetw, sysinfo, tokio-rs/console, elkjs, tauri, lizard, rough and excalidraw, because the gh API and curl were blocked (403).
- I could not verify krabsetw's license or last-commit date. It is expected to be MIT but was not confirmed.
- I did not confirm whether the rust-analyzer `scip` subcommand changed recently.
- I did not fetch the full scip-java or scip-go release notes.

## Q2: License compatibility for an MIT/Apache-2.0 desktop app

### Takeaway
Most dependencies are permissive. Three need attention:
- **elkjs** (EPL-2.0 OR GPL-3.0-or-later): choose EPL-2.0, a weak file-level copyleft that is fine to bundle unmodified.
- **rust-code-analysis** (MPL-2.0): file-level copyleft, fine to link unmodified.
- **aya eBPF programs**: the kernel-side program usually has to declare a GPL-compatible license to use GPL-only helpers, even though the aya crates themselves are MIT/Apache.

notify and blake3 offer CC0, which is permissive.

### Cited Findings
- elkjs license field: "EPL-2.0 OR GPL-3.0-or-later" — [npm registry elkjs 0.12.1](https://registry.npmjs.org/elkjs/0.12.1)
- rust-code-analysis license: MPL-2.0 — [crates.io](https://crates.io/api/v1/crates/rust-code-analysis)
- aya, aya-ebpf, aya-log, aya-build: "MIT OR Apache-2.0" — [crates.io](https://crates.io/api/v1/crates/aya-ebpf); the README says "available under the MIT license or the Apache License 2.0" — [GitHub aya-rs/aya](https://github.com/aya-rs/aya)
- Neither the aya README nor the development-prerequisites page says anything about a GPL license section for eBPF programs — [aya book](https://aya-rs.dev/book/start/development/)
- notify is CC0-1.0. ignore is "Unlicense OR MIT". blake3 is "CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception". mach2 is "BSD-2-Clause OR MIT OR Apache-2.0". tokenizers, pprof and scip are Apache-2.0 only — [crates.io API](https://crates.io/api/v1/crates/notify)
- TypeScript is Apache-2.0. @sourcegraph/scip-typescript is Apache-2.0 and @sourcegraph/scip-python is MIT — [npm](https://registry.npmjs.org/typescript)
- lizard is MIT — [PyPI](https://pypi.org/pypi/lizard/json)

### Inferences
- From general kernel knowledge, not sourced in this session: the Linux kernel's verifier refuses GPL-only helpers (for example `bpf_probe_read*` and `bpf_trace_printk`, and most tracing helpers) unless the program's `license` section is GPL-compatible. aya eBPF programs typically declare `#[unsafe(link_section = "license")] static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";`. This would make the small eBPF object file (MIT/GPL dual) a separate GPL-compatible work loaded into the kernel. It does not affect the user-space Rust app's license. The writer should verify this against the aya book or kernel docs before stating it as fact.
- elkjs under EPL-2.0: distributing the unmodified JS bundle inside an MIT app is generally allowed. Modifications to elkjs files must be published under EPL, and EPL requires notices or source availability. Shipping the license text in third-party notices is enough for unmodified use. Alternatives are dagre (MIT; not fetched) or d3-dag.
- MPL-2.0 (rust-code-analysis): compatible with MIT/Apache distribution. Only modified MPL files must stay open.
- The Apache-2.0-only crates (tokenizers, pprof, scip) are compatible with Apache-2.0 apps. Apache-2.0 is fine for an MIT/Apache dual-licensed app, but NOTICE files must be carried.

### Gaps
- No primary source was fetched for the kernel's GPL-only helper rule or for aya's recommended LICENSE declaration.
- krabsetw's license was not confirmed.
- Model weights used with candle or llama.cpp have their own licenses. That is out of scope here.

## Q3: Integration caveats (Rust/TS)

### Takeaway
The most important caveats:
1. TypeScript 7 (Go native) ships **no programmatic API**, so typescript-eslint and other API consumers need `@typescript/typescript6`.
2. tree-sitter core 0.27 uses ABI 15 and accepts grammars down to ABI 13. The official TypeScript and Java grammars are still ABI 14 crates (0.23.x), which works.
3. tree-sitter-language-pack downloads or loads parsers dynamically, which matters for offline bundling.
4. aya's eBPF side needs nightly Rust, `rust-src` and bpf-linker.
5. krabsetw is C++ (needs FFI); ferrisetw is pure Rust but stale.
6. React Flow 13 just shipped with breaking changes but better sub-flow handling.
7. Tauri v2 capabilities must explicitly grant fs, shell and dialog permissions.

### Cited Findings

**tree-sitter ABI**
- tree-sitter 0.27.2's headers define `TREE_SITTER_LANGUAGE_VERSION 15` and `TREE_SITTER_MIN_COMPATIBLE_LANGUAGE_VERSION 13` (read from the crate tarball's C headers) — [crates.io tree-sitter 0.27.2 crate](https://crates.io/api/v1/crates/tree-sitter)
- `parser.c` `LANGUAGE_VERSION` in each grammar: tree-sitter-typescript 0.23.2 = 14, tree-sitter-java 0.23.5 = 14, tree-sitter-rust 0.24.2 = 15, tree-sitter-python 0.25.0 = 15 (read from the crate tarballs) — [crates.io](https://crates.io/api/v1/crates/tree-sitter-typescript)
- The grammar crates depend on `tree-sitter-language ^0.1` (latest 0.1.9), not on `tree-sitter` directly; tree-sitter is only a dev-dependency. This decouples grammar versions from core versions — [crates.io dependencies endpoint](https://crates.io/api/v1/crates/tree-sitter-rust/0.24.2/dependencies)
- ast-grep-core 0.51.3 depends on `tree-sitter ^0.27.0` and `tree-sitter-typescript ^0.23.2`, so it lines up with the current core — [crates.io](https://crates.io/api/v1/crates/ast-grep-core/0.51.3/dependencies)
- tree-sitter-language-pack 1.25.2 depends on `tree-sitter ^0.27` plus runtime `ureq`, `libloading`, `dirs`, `fd-lock`, `zstd` and `tar` (normal dependencies). This points to downloading or caching parsers and loading them dynamically at runtime — [crates.io](https://crates.io/api/v1/crates/tree-sitter-language-pack/1.25.2/dependencies)

**TypeScript 7 / tsgo**
- TypeScript 7.0, announced July 8, 2026, is a native port built in Go and ships as the `typescript` npm package's `tsc`. Nightlies moved to `typescript@next`, replacing `@typescript/native-preview` — [TS blog](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/)
- "While TypeScript 7.0 is here, it does not ship with an API". A new API is expected in 7.1. typescript-eslint should alias `typescript` to `@typescript/typescript6`, which provides `tsc6` and the 6.0 API — [TS blog](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/)
- 7.0 adopts 6.0's defaults: `strict` true, `types` defaults to `[]`, `rootDir` defaults to `./`. Options such as `target: es5`, `baseUrl` and `moduleResolution: node10` are now hard errors. Volar-based tooling for embedded languages (Vue, Svelte, Astro, MDX) still depends on 6.0 — [TS blog](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/)
- The typescript 7.0.2 npm package has platform-specific optional dependencies (`@typescript/typescript-linux-x64` and others), meaning a native binary — [npm](https://registry.npmjs.org/typescript)
- Coverage of the release: [InfoWorld](https://www.infoworld.com/article/4196378/go-based-typescript-7-0-arrives.html), [InfoQ](https://infoq.com/news/2026/08/typescript-7-released/)

**pnpm 12**
- In 12.3.0, `minimumReleaseAgeStrict` defaults to true when `minimumReleaseAge` is set explicitly. There is a built-in default `minimumReleaseAge` of 1440 minutes (non-strict). Build scripts are blocked by default, so wrapper installs must allow lifecycle scripts. `pnpm -r run` errors on dependency cycles (`ERR_PNPM_TASK_CYCLE`) — [pnpm releases](https://github.com/pnpm/pnpm/releases)

**aya**
- Building aya eBPF programs needs both stable and nightly Rust, with `rustup toolchain install nightly --component rust-src`. It also needs bpf-linker (the "required LLVM version and platform-specific steps change over time"), bpftool for generating kernel-type bindings, and cargo-generate for the template — [aya book](https://aya-rs.dev/book/start/development/)
- aya needs no libbpf, bcc or C toolchain, and uses BTF for compile-once-run-anywhere — [aya README](https://github.com/aya-rs/aya)
- aya crates have **MSRV 1.98.0**, the highest in the set (Tauri needs 1.90) — [crates.io](https://crates.io/api/v1/crates/aya)

**ETW (Windows)**
- krabsetw is C++, with a C++/CLI .NET wrapper. There are no Rust bindings, no GitHub releases, and distribution is through NuGet. It supports x64 and ARM64 only — [GitHub](https://github.com/microsoft/krabsetw)
- ferrisetw 1.2.0 (MIT OR Apache-2.0) is the pure-Rust option. Its last release was 2024-06-27 — [crates.io](https://crates.io/api/v1/crates/ferrisetw)
- windows 0.62.2 (2025-10-06) is the raw API option — [crates.io](https://crates.io/api/v1/crates/windows)

**SCIP indexers (external binaries)**
- scip-typescript is an npm package (needs Node.js), 0.6.2. scip-python is an npm package (Node-based, Pyright-derived), 0.6.8. scip-java is a JVM artifact on Maven (0.12.3). scip-go is a Go module (v0.2.10). The Rust indexer is rust-analyzer (rust-lang/rust-analyzer) — [npm](https://registry.npmjs.org/@sourcegraph/scip-typescript), [Maven](https://repo1.maven.org/maven2/com/sourcegraph/scip-java_2.13/maven-metadata.xml), [Go proxy](https://proxy.golang.org/github.com/sourcegraph/scip-go/@latest), [scip repo](https://github.com/scip-code/scip)
- The `scip` Rust crate 0.10.0 provides the protobuf bindings for reading and writing index files — [crates.io](https://crates.io/api/v1/crates/scip)

**elkjs in Vite**
- For browser use, load `elk-api.js` (not `elk.bundled.js`) and pass `workerUrl` pointing at `elk-worker.min.js`, or use `workerFactory`. Use from Node needs the `web-worker` package. The FAQ lists bundler problems ("g is not defined", "Can't resolve web-worker") caused by GWT transpilation, and poor modularization is an open issue — [GitHub elkjs](https://github.com/kieler/elkjs)

**React Flow 13 / sub-flows**
- 13.0.0 breaking changes:
  - `getHandleConnections` and `useHandleConnections` are removed; use `getNodeConnections`.
  - `useStore` is replaced by specialized store hooks with `useShallow`.
  - `onConnectEnd` and `onReconnectEnd` now receive `to` in flow coordinates.
  - `paneClickDistance` is replaced by `paneDragThreshold`, and `nodeClickDistance` is removed.
  - `fitView` padding defaults to 5%, and numeric padding is now in pixels.
  - The Handle geometry type is renamed `HandleBounds`.
  - React 18 is the minimum, and the UMD build is removed.
  
  The library is now compiled with the React Compiler and the store is split. Sub-flows: parent nodes no longer need to come before children in the `nodes` array (#5915), and there is a new `changeParent` on `useReactFlow` (#5939). Sub-flows still use v12's `parentId` and `extent: 'parent'`, which got fixes in 12.11.3 and 12.8.0 — [reactflow.dev whats-new](https://reactflow.dev/whats-new)

**Tauri v2 capabilities**
- Capabilities are JSON or TOML files in `src-tauri/capabilities/`, with fields `identifier`, `windows`, `permissions` (for example `core:window:default`) and optional `platforms` and `remote.urls`. All files are enabled unless they are explicitly listed in `tauri.conf.json`. Custom commands are allowed for all windows by default unless restricted through `AppManifest::commands` in build.rs. Capabilities do not protect against "overly lax scopes" — [Tauri docs](https://v2.tauri.app/security/capabilities/)
- Tauri 3.0.0-alpha.6 exists on crates.io (tauri) and npm (@tauri-apps/cli next tag), but the docs page does not mention v3 — [crates.io](https://crates.io/api/v1/crates/tauri)

**Complexity metrics**
- lizard is Python (PyPI 1.26.1, MIT, active) — [PyPI](https://pypi.org/pypi/lizard/json)
- rust-code-analysis (Mozilla, MPL-2.0) last published 0.0.25 on 2023-01-13. It is built on tree-sitter, and its old pinned tree-sitter version would likely conflict with tree-sitter 0.27 in the same binary — [crates.io](https://crates.io/api/v1/crates/rust-code-analysis)

### Inferences
- tree-sitter: pin `tree-sitter = "0.27"` and use the individual grammar crates through `tree-sitter-language`. All six grammars have ABI 14 or 15, inside the supported range 13–15, so they work together. ast-grep 0.51 uses the same core, so it can share it. For a fully offline desktop app, prefer individual grammar crates, which compile in via `cc`, over tree-sitter-language-pack. The language pack's runtime dependencies (ureq, libloading, dirs) suggest it fetches or caches parsers at runtime; check its feature flags for a static or "all bundled" mode before relying on it.
- rust-code-analysis looks effectively unmaintained on crates.io and probably pins an old tree-sitter (the dependency tree was not verified). It is safer to implement cyclomatic and cognitive complexity and NLOC directly over tree-sitter or ast-grep queries, in a lizard-like way, than to depend on it. The other option is to shell out to lizard through a bundled Python, which is heavier.
- SCIP bundling: rust-analyzer and scip-go are single static native binaries and easy to bundle as Tauri sidecars. scip-typescript and scip-python need a Node.js runtime; Node SEA or bundling node is possible but adds about 40–100 MB. scip-java needs a JVM plus the project's build tool (Gradle or Maven), so it is not realistically bundleable offline. Treat it as "use the system JDK if present". Every sidecar must be declared under `bundle.externalBin` and allowed through `shell:allow-execute` / `shell:allow-spawn` scopes in a capability file.
- aya: eBPF objects can be built at app build time (`aya-build` in build.rs with nightly and bpf-linker) and embedded with `include_bytes_aligned!`. End users then need no toolchain, but loading programs at runtime still needs root, CAP_BPF or CAP_PERFMON. Note that MSRV 1.98 forces a recent stable toolchain for the whole workspace.
- ETW: avoid krabsetw, since it would need a cxx or bindgen layer over C++ templates plus MSVC builds. Use ferrisetw, or the `windows` crate directly. Real-time ETW kernel sessions need admin rights.
- TypeScript: TS 7 works for `tsc --noEmit` type checking. Vite and esbuild/oxc strip types and do not depend on the TS API. If ESLint with typescript-eslint is used, add the `@typescript/typescript6` alias. Biome 2.6 avoids this problem because it does not use the TS API.
- elkjs in Vite: `new ELK({ workerFactory: () => new Worker(new URL('elkjs/lib/elk-worker.min.js', import.meta.url)) })`, or the `?worker` / `?url` import suffix, is the usual pattern. The "Can't resolve web-worker" issue comes up when the bundled build is imported in a browser context. This pattern was not verified in this session.
- pnpm 12 blocks build scripts by default. Packages with native postinstall steps (for example `@tauri-apps/cli` platform binaries if delivered via scripts, or esbuild) may need approval in `allowBuilds` / `onlyBuiltDependencies`. The 1440-minute minimumReleaseAge means brand-new releases (notify 9 on crates is unaffected; npm packages released in the last day are) will not install by default.

### Gaps
- The Vite + elkjs worker pattern was not verified against a primary Vite or elkjs doc.
- Whether tree-sitter-language-pack supports a fully static or offline build was not confirmed (I did not read its feature flags or README).
- rust-code-analysis's exact tree-sitter dependency version was not fetched.
- The Tauri shell sidecar scope syntax (Command Scopes page) was not fetched.
- I found no source on whether a Rust port of lizard exists. None turned up on crates.io in this session, but no dedicated search was done.

## Q4: Are pnpm and TypeScript on their latest majors? (TS 6/7, tsgo)

### Takeaway
Yes. As of 2026-11-21, pnpm's latest is **12.21.0** (major 12, out since 2026-08-26). TypeScript's latest is **7.0.2** (2026-07-08), which *is* the Go-native "tsgo" compiler now shipped as `typescript`. 7.1 is in beta (7.1.0-beta) and is expected to bring back a programmatic API. TS 6.0.3 is the last 6.x and stays the API and editor fallback through `@typescript/typescript6`.

### Cited Findings
- pnpm dist-tags: latest 12.21.0 (2026-11-13), next-12 12.22.0, latest-11 11.28.13, latest-10 10.34.9. 11.0.0 came out 2026-04-28 and 12.0.0 on 2026-08-26 — [npm registry pnpm](https://registry.npmjs.org/pnpm)
- typescript dist-tags: latest 7.0.2 (2026-07-08), rc 7.0.1-rc (2026-06-18), beta 7.1.0-beta, next 7.1.0-dev.20261121.1. 6.0.2 came out 2026-03-23 and 6.0.3 on 2026-04-16 — [npm registry typescript](https://registry.npmjs.org/typescript)
- TS 7.0 beta was announced on April 21, 2026, the RC on June 18, 2026, and the final release on July 8, 2026 — [TS 7.0 beta](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0-beta/), [TS 7.0 RC](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0-rc/), [TS 7.0](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/)
- `@typescript/native-preview` last published 7.0.0-dev.20260707.2 and is replaced by `typescript@next` — [npm](https://registry.npmjs.org/@typescript/native-preview), [TS blog](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/)

### Inferences
- FolderKnower should target `typescript@^7.0` for type checking and keep `@typescript/typescript6` only if typescript-eslint is used. Biome avoids that need.
- Use `pnpm@12` through the `packageManager` field with Corepack or the pnpm self-installer.

### Gaps
- No official pnpm 12 release blog with a consolidated breaking-change list was fetched. The Node.js minimum for pnpm 12 was not confirmed.
- I did not confirm when TS 7.1 is expected to ship stable.
