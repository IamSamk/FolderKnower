# 16. Memory techniques catalog

**Status: done.** `ontology/ai-catalog.yaml` under `memory:`, matched in
`ark-patterns/src/ai.rs`.

Each technique is matched by a call, a keyword argument or an identifier, with its
import required. It is shown under its **real name** with the setting as written in the
code ("enable_prefix_caching=True in serve.py line 4").

| Name | Engine | Matched by |
|---|---|---|
| PagedAttention KV cache | vLLM | `LLM(...)` / `AsyncLLMEngine`, `gpu_memory_utilization`, `block_size` |
| Automatic prefix caching | vLLM | `enable_prefix_caching` |
| KV cache quantization | vLLM / llama.cpp | `kv_cache_dtype`, `type_k` / `type_v` |
| Context window (n_ctx) and GPU offload | llama.cpp | `n_ctx`, `n_gpu_layers` |
| KV cache implementation | transformers | `cache_implementation` (static, offloaded, quantized) |
| Rolling cache with attention sinks (StreamingLLM) | transformers | `SinkCache` |
| Weight quantization | transformers / bitsandbytes | `BitsAndBytesConfig`, `load_in_4bit`, GPTQ / AWQ |
| Memory-efficient attention | transformers | `attn_implementation` (flash_attention_2, sdpa) |
| Fused attention kernel | PyTorch | `scaled_dot_product_attention` |
| Prompt caching (cache_control) | Anthropic API | identifier `cache_control` |
| Prompt caching (automatic prefix reuse) | OpenAI API | `prompt_cache_key`, `prompt_cache_retention` |
| Rolling conversation window | LangChain | `ConversationBufferWindowMemory`, `trim_messages` |
| Semantic response cache | LangChain / GPTCache | `set_llm_cache`, `SemanticCache` |
| Gradient checkpointing | transformers / torch | `gradient_checkpointing_enable`, `checkpoint` |
| Mixed precision | torch | `autocast`, `GradScaler`, `bf16=True` |
| CUDA caching allocator release | torch | `torch.cuda.empty_cache` |
| Model residency (keep_alive) and context size | Ollama | `keep_alive`, `num_ctx` |

## Adding one

Add a YAML entry (`id`, `name`, `engine`, `match` / `kwarg` / `ident`, `requires`,
`explain`), then a test in `crates/ark-patterns/src/tests.rs`. No Rust change is needed.
