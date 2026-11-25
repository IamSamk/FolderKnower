# 05. Runtime tracing hooks

**Status: next.** Goal: turn "this function *can* call the reranker" into "this run
called it 3 times, kept 3 of 16 documents, and spent 41 ms there".

## Ladder (cheapest first, each opt-in)

1. **Sampling, no code changes.** py-spy (`--pid`, JSON output), or Python 3.15
   `profiling.sampling`. Stacks are joined to `FunctionNode` ids by file path and line
   range. Works for Python today; `perf` maps for Node with `--perf-basic-prof`.
2. **PEP 669 `sys.monitoring`.** A tiny bootstrap (`architekton-trace`) registers
   `PY_START`, `PY_RETURN`, `RAISE` and `BRANCH` events for the files in the repo
   only. The output:
   - call counts per function;
   - which branch arms ran, laid over the Workflow view (doc 11) so the "clean path"
     becomes "the path this run took";
   - which exits raised.
3. **memray live.** Allocation flame graph per function. Feeds the heap part of the
   breakdown.
4. **OpenTelemetry / OpenInference spans** (doc 07). The best source for AI pipelines.
5. **eBPF uprobes (Linux, elevated, `ark-probe`).** For native code and runtimes we
   cannot instrument. Follows `docs/security/telemetry-elevation.md`.

## DSA from runtime

`BRANCH` and `CALL` events on container methods (`append`, `pop`, `popleft`, `heappush`)
give the real operation sequence. The DSA animation (doc 10) then replays the run
instead of the source-order approximation.

## Overhead budget

Sampling stays at 100 Hz or below. `sys.monitoring` hooks are disabled per code object
after N events (`DISABLE` return), so hot loops pay once.
