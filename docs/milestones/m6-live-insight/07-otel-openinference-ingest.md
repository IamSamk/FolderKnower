# 07. OpenTelemetry GenAI and OpenInference ingest

**Status: next.**

Static recognition (doc 12) knows the *shape* of a RAG system and its literal settings.
Traces know the *run*: how many chunks came back, their scores, what the reranker kept,
which tool the agent chose, and cache-read tokens.

## Plan

1. **Receiver.** An OTLP/HTTP receiver bound to `127.0.0.1:4318`, inside the desktop
   app, off by default. A "Connect traces" button shows the two environment lines to
   paste: `OTEL_EXPORTER_OTLP_ENDPOINT` and the OpenInference instrumentor.
2. **Mapping** span → `AiStage` (by role, then by code location when
   `code.filepath`/`code.lineno` exist):

   | Span kind / operation | AiRole |
   |---|---|
   | EMBEDDING / `embeddings` | embedder |
   | RETRIEVER / `retrieval` | retriever |
   | RERANKER | reranker |
   | LLM / `chat` | llm |
   | TOOL / `execute_tool` | tool |
   | AGENT / `invoke_agent` | agent |

3. **Overlay** on the AI pipeline view:
   - per-stage latency and call count;
   - retrieved document list with scores;
   - reranker input → output (16 → 3);
   - token usage with the cached share highlighted.

4. **Replay:** "Play a request" uses a real trace instead of the static tour.

## Storage

Last 200 traces in memory; optional SQLite in the app data dir. Content attributes
(prompts, documents) are dropped unless the user ticks "keep text", because traces can
hold private data.
