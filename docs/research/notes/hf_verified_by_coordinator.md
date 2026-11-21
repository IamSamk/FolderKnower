# HF Hub verification (coordinator, fetched 2026-11-21 via HF MCP)
Embeddings:
- Qwen/Qwen3-Embedding-0.6B — 596M, qwen3, Apache-2.0, official GGUF repo Qwen/Qwen3-Embedding-0.6B-GGUF (Apache-2.0). arXiv 2506.05176.
- google/embeddinggemma-2 — created 2026-09-14, Apache-2.0, multimodal embedding, arXiv 2610.37364; GGUF unsloth/embeddinggemma-2-GGUF; ONNX onnx-community/embeddinggemma-2-ONNX. (param count not shown; derivative cards say ~270M)
- google/embeddinggemma-300m — 303M, license: gemma (custom), gated.
- nomic-ai/CodeRankEmbed — 137M, nomic_bert (custom_code), MIT. arXiv 2412.01007 (CoRNStack).
- nomic-ai/nomic-embed-code — 7.07B, Apache-2.0 (too large for desktop default).
- jinaai/jina-code-embeddings-0.5b — 494M qwen2, CC-BY-NC-4.0 (NON-COMMERCIAL: avoid as default). arXiv 2508.21290.
- jinaai/jina-embeddings-v2-base-code — 161M bert, Apache-2.0, ONNX + community GGUF.
- jinaai/jina-embeddings-v5-text-nano — CC-BY-NC-4.0 (2026-01).
- ibm-granite/granite-embedding-small-english-r2 — 47.7M ModernBERT, Apache-2.0 (updated 2026-01). arXiv 2508.21085.
- microsoft/bitnet-embedding-270m / -0.6b — 2026-07, GGUF, license not shown in listing.
SLMs:
- Qwen/Qwen3.5-2B (2.27B) and Qwen/Qwen3.5-4B (4.66B) — Apache-2.0, released ~2026-03, arch qwen3_5 (multimodal LM class). Check llama.cpp/candle arch support before committing.
- Qwen/Qwen3-4B-Instruct-2507 — 4.02B, Apache-2.0 (2025-08). Mature GGUF + candle qwen3 support.
- Qwen/Qwen3-1.7B — 2.03B, Apache-2.0.
- Qwen/Qwen2.5-Coder-1.5B-Instruct — Apache-2.0; Qwen2.5-Coder-3B-Instruct — license "other" (Qwen Research, non-commercial) → avoid.
- google/gemma-4-E2B-it (5.1B total, ~2B effective) / gemma-4-E4B-it (8B total, ~4B effective) — Apache-2.0 (Gemma 4 moved to Apache), official QAT GGUF google/gemma-4-E4B-it-qat-q4_0-gguf.
- microsoft/Phi-4-mini-instruct — 3.84B, MIT.
- HuggingFaceTB/SmolLM3-3B — 3.08B, Apache-2.0.
- ibm-granite/granite-4.0-micro — 3.4B, granitemoehybrid (Mamba hybrid), Apache-2.0.
