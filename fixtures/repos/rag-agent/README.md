# rag-agent (fixture)

A small "ask the book" app used to demo and test Architekton's AI-pipeline,
data-structure and ideal-path views. It is never executed by the tests.

- `app/ingest.py`: load a PDF, chunk it, embed it, store it (dense) + BM25 (keywords).
- `app/retrieve.py`: hybrid retrieval, Reciprocal Rank Fusion, cross-encoder rerank.
- `app/agent.py`: LangGraph agent that decides whether to retrieve again or answer.
- `app/serve.py`: vLLM server settings (PagedAttention, prefix caching, fp8 KV cache).
- `model/tiny_gpt.py`: a tiny transformer (attention, MLP, KV cache).
- `algorithms/`: classic data structures the DSA lens animates.
- `knowledge/`: an Open Knowledge Format (OKF) bundle.
