# 03. Live telemetry (unprivileged sampler)

**Status: done.** Crate `crates/ark-telemetry`, Tauri module
`apps/desktop/src-tauri/src/telemetry.rs`, view `components/live/LiveView.tsx`.

## Sampler

`Sampler` holds one `sysinfo::System` between refreshes because CPU % needs two
samples. `refresh()` updates the following, never faster than
`sysinfo::MINIMUM_CPU_UPDATE_INTERVAL`:

- memory and swap;
- per-core CPU;
- for every process: memory, CPU, disk I/O and tasks (threads), with the command line
  filled once.

| API | Returns |
|---|---|
| `system()` | total/used/available memory, swap, per-core usage, CPU brand, physical cores, OS, process count |
| `processes(limit, SortBy)` | top N rows, threads folded into their process (`thread_kind().is_none()`) |
| `process(pid)` | row, descendants (BFS on parent links), tree memory, Linux breakdown, thread names |
| `find(needle, limit)` | processes whose name or command line matches |

## Desktop commands

`telemetry_system`, `telemetry_processes(limit, sort)`, `telemetry_process(pid)`,
`telemetry_find(needle)` and `telemetry_runtimes`. They share one `Mutex<Option<Sampler>>`
managed by Tauri, created lazily on first call. The Live view polls every 1.5 s and asks
for runtimes every third tick.

In a plain browser (the web demo), `lib/telemetry.ts` simulates a believable machine
and the view says **Simulated numbers** in a banner. Simulated data is never shown
unlabelled.

## Privacy and privilege

- Same-user processes only; no admin rights, no kernel modules.
- Nothing is sent anywhere; the model-server probes hit `127.0.0.1` only, with a 350 ms
  timeout.
- Deeper tracing (eBPF, ETW kernel providers) stays in opt-in `ark-probe`.

## Next

- Pin a process: follow the process that runs the analysed repository (match `cwd` or
  command line against the repo root) and show its memory next to the architecture view.
- Per-function attribution: join sampled stacks (doc 05) to `FunctionNode.liveTelemetry`.
- GPU: NVML on Linux and Windows, Metal performance counters on macOS. Read-only and
  unprivileged where the driver allows.
