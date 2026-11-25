# 02. Research synthesis and sources

Research done before building (2026-11). Each line says what we took from it.

## Observability standards for AI systems

- **OpenTelemetry GenAI semantic conventions** (status: Development).
  Operation names `retrieval`, `embeddings`, `execute_tool`, `invoke_agent`, `chat`.
  Attributes include `gen_ai.retrieval.top_k`, `gen_ai.retrieval.documents` (id, score),
  `gen_ai.usage.cache_read.input_tokens` and `cache_write`.
  We map our `AiRole` to these names so a runtime trace can be laid over the static
  pipeline (doc 07). https://opentelemetry.io/docs/specs/semconv/gen-ai/
- **OpenInference** (Arize). Span kinds LLM, EMBEDDING, RETRIEVER, RERANKER, TOOL, AGENT,
  CHAIN. Attributes:
  - `retrieval.documents.N.document.{id,score,content}`
  - `reranker.{input_documents,output_documents,top_k,model_name}`
  - `embedding.model_name`
  - `llm.token_count.prompt_details.cache_read`

  This is the richest source of real chunk counts and reranker cuts.
  https://github.com/Arize-ai/openinference

## Model servers

- **vLLM** Prometheus metrics: `vllm:kv_cache_usage_perc`, `vllm:prefix_cache_hits` and
  `vllm:prefix_cache_queries`, `vllm:num_requests_running` and `vllm:num_requests_waiting`.
  Parsed in `ark-telemetry::runtime::parse_vllm_metrics`. https://docs.vllm.ai/
- **llama.cpp llama-server**:
  - `/metrics` (only with `--metrics`) reports `llamacpp:prompt_tokens_total`,
    `llamacpp:tokens_predicted_total` and `llamacpp:requests_processing`.
  - `/slots` reports `n_ctx` and `is_processing` per slot.
  - Flags `-ctk` and `-ctv` set the KV cache type; `--cache-prompt` and `--context-shift`
    control prompt reuse and context shifting.

  https://github.com/ggml-org/llama.cpp/tree/master/tools/server
- **Ollama** `/api/ps`: loaded models with `size`, `size_vram`, `context_length`,
  `expires_at`. https://github.com/ollama/ollama/blob/main/docs/api.md

## KV cache and attention memory

- PagedAttention (Kwon et al., SOSP 2023). The KV cache lives in fixed-size blocks; this
  avoids reserving the whole window per request. https://arxiv.org/abs/2309.06180
- StreamingLLM attention sinks (Xiao et al., 2023). Keep the first few tokens plus a
  rolling window. https://arxiv.org/abs/2309.17453
- Mistral 7B sliding-window attention with a rolling buffer cache (W = 4096).
  https://arxiv.org/abs/2310.06825
- Model shapes for the calculator (layers, KV heads, head dim) come from each model's
  published `config.json` on the Hugging Face Hub.
- Provider prompt caching:
  - Anthropic: `cache_control` breakpoints.
    https://docs.claude.com/en/docs/build-with-claude/prompt-caching
  - OpenAI: automatic prefix caching. https://platform.openai.com/docs/guides/prompt-caching

## Runtime hooks (doc 05)

- PEP 669 `sys.monitoring`, a low-overhead event API (Python 3.12+).
  https://peps.python.org/pep-0669/
- Python 3.15 `profiling.sampling`, a sampling profiler in the standard library.
- memray live mode. https://github.com/bloomberg/memray
- py-spy (sampling without code changes). https://github.com/benfred/py-spy
- eBPF uprobes through Aya (Rust). Opt-in, elevated, in the separate `ark-probe`.
  https://aya-rs.dev

## Memory accounting

- Linux `/proc/<pid>/smaps_rollup` gives RSS, PSS, anonymous, file-backed, shmem and
  swap. `/proc/<pid>/status` gives VmPeak, VmHWM, VmStk and VmLck.
  https://www.kernel.org/doc/html/latest/filesystems/proc.html
- The Windows working set differs from RSS. We label the field "resident (RSS) or
  working set".
- `sysinfo` 0.40 is the cross-platform per-process sampler (no elevation for same-user
  processes).

## Data-structure recognition

- MemPick (Haller, Slowinska, Bos, 2013) detects lists, trees and graphs from heap shape
  at runtime.
- Howard (Slowinska, Stancescu, Bos, NDSS 2011) recovers data structures from binaries.

Both inform doc 09. Statically we use usage shape (push/pop pairs, left/right fields,
visited sets), which is cheaper and works on source.

## Visualization and scale

- Transformer Explainer (Georgia Tech Polo Club), a reference for teaching attention
  step by step. https://poloclub.github.io/transformer-explainer/
- cosmos.gl 3.6: GPU force layout for 100k+ nodes. https://github.com/cosmograph-org/cosmos
- flatbush 4.8: static R-tree for hit testing. https://github.com/mourner/flatbush
- @tanstack/react-virtual 3.14: list virtualization. https://tanstack.com/virtual
- goblin 0.10 and object 0.41: ELF/PE/Mach-O parsing for the binary view (doc 19).

## Formats

- Google Cloud **Open Knowledge Format (OKF)**, June 2026. A folder of Markdown files
  with YAML frontmatter where `type` is required; `index.md` and `log.md` are reserved
  names. https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf

## Verified versions (2026-11-24)

sysinfo 0.40.4, goblin 0.10.7, object 0.41.0, memmap2 0.9.12, rstar 0.14.0,
@cosmos.gl/graph 3.6.0, @tanstack/react-virtual 3.14.28, motion 14.14.0,
flatbush 4.8.0, @xyflow/react 13.0.3.
