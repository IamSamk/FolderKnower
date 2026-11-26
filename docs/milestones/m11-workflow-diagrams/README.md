# M11: workflow diagrams

The System view has a third map, **Flow**, next to System and Memory. It draws boxes
in phases with arrows, the way an architecture sheet is drawn by hand, but every box comes
from something the analysis found and carries the `path:line` that proves it.

## What ships (0.9.0)

| Kind | Built from | Shows |
|---|---|---|
| Request | an example run (`exampleRuns`) | one action step by step, grouped by the container each step runs in |
| AI pipeline | `insights.ai` stages | offline ingest and index, then online: parallel searches, fusion, rerank, prompt, model, tool loop |
| Memory and context | `memoryMap`, `insights.memoryTechniques`, AI stages | what is kept and where, what one model call is made of with its budget, caches and the KV cache |

Simple shows titles. Standard adds one sentence. Deep adds settings, the full explanation and
the proof lines (click to open the code). The feature picker narrows the AI pipeline and the
request flow to one feature.

## Rules that keep it honest

- A box exists only when a stage, hop, placement or technique exists for it.
- Settings shown (`chunk_size=800`, `num_ctx=8192`, `enable_prefix_caching=True`) are literals
  read from the call. A rule such as "fold again over 2,000 chars" appears only once the code
  has it as a readable constant; until then no box claims it.
- Arrows are the order the analysis found (a run's hops, the retrieval chain). They are drawn
  around boxes, never through them.

## Not yet

- Business rules that live in plain code (fold validators, summary-of-summary limits) need
  constant extraction (`NAME = 2000` next to the function that uses it) or the optional
  narrator model reading the function. Both are planned, with proof lines.
- Diagram playback (zoom through the boxes like the example run).

## Other kinds that fit the same builder

From facts already in the graph: sequence view of an example run, LangGraph agent graph
(nodes and edges are read), data lineage (ingest to tables to routes), module layering,
network topology with ports and volumes, API surface map.

Needs new analysis first: auth flow (login, token, refresh), background jobs and queues
(workers, cron, brokers), event and pub-sub flow, state machines (enums and transitions),
CI and release pipeline (workflow files), environment and config map (variable to consumer),
training loop and evaluation, permission matrix.
