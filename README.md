# Architekton

Offline-first, deterministic system-design teacher and Excalidraw-style codebase visualizer (Tauri 2 + Rust + React).

The project is in the planning stage. Start with:
- [`plan.md`](plan.md): the living spec (Rev 2)
- [`docs/phase1-checklist.md`](docs/phase1-checklist.md): Phase 1 execution checklist
- [`docs/research/synthesis.md`](docs/research/synthesis.md): 2025–2026 research synthesis
- [`docs/security/telemetry-elevation.md`](docs/security/telemetry-elevation.md): least-privilege telemetry design
- [`docs/architecture/semantic-zoom.excalidraw`](docs/architecture/semantic-zoom.excalidraw): architecture sketch (open at excalidraw.com)

## Development

```sh
cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test
cargo deny check                     # license allowlist + advisories
bash scripts/deny-negative-test.sh   # proves an LGPL crate is rejected
pnpm install && pnpm lint            # Biome (pnpm 12 via packageManager)
```

Toolchain is pinned in `rust-toolchain.toml` (Rust 1.95, edition 2024).
