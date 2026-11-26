# M12: every search path

A retrieval system often searches several ways at once. The analyzer used to keep one stage
per role, so a second keyword search, or the graph search next to it, disappeared.

## What ships

- One stage per function that runs its own search: vector (`<=>`), full-text (`ts_rank`,
  `plainto_tsquery`, `tsvector`) and graph (Cypher `MATCH (x:Label)`). All feed fusion.
- Hand-made agent loops (`run_agent…`, `…agent_loop`), `register("name", "description", …)`
  tool registries, and `/chat/completions` clients written with plain HTTP.
- Reranker model ids including `Qwen…-Reranker` names.
- The Flow map draws one arrow into and one out of the search group, so arrows never cross
  the boxes.

## Rules

- A box needs code that does the step. A model named only in an experiment, a test or a
  comment draws nothing.
- Indexing and chat-memory functions run similar queries and are not search paths.
- Postgres full-text search is not labelled BM25.

## Not yet

- An explicit "is the evidence sufficient?" step is not recognised as its own box; it shows
  as the agent loop's "search again" arrow.
- Which searches a particular request actually runs is static knowledge only: a leg switched
  off by configuration is still drawn.
- Search legs are found from query text inside functions whose names say they search. A leg
  with an unusual name is missed.
- Library calls and hand-written queries for the same role: the library wins.
