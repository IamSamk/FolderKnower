# 06. Model server introspection

**Status: done.** Parsers: `ark-telemetry::runtime`. Fetching:
`telemetry_runtimes` in the desktop app.

| Server | Endpoint probed | What we show |
|---|---|---|
| Ollama | `127.0.0.1:11434/api/ps` | each loaded model: size, VRAM share, context length, quantization, parameter size, expiry |
| llama-server | `:8080` / `:8081` `/health`, `/metrics`, `/slots` | prompt and generated token totals, requests processing, per-slot `n_ctx`, busy slots |
| vLLM | `:8000/metrics` | KV cache usage %, prefix cache hit rate (hits / queries), running and waiting requests, token totals |

`parse_prometheus` is a small text-format parser (name, labels, value). `family()` sums
a metric across label sets, which matters for multi-model vLLM.

When a server answers but a field is missing (llama-server started without
`--metrics`), the card says so in a note instead of showing zero.

## Student framing

- **KV cache gauge:** "Keys and values of every token the running requests have seen.
  Near 100% the server pauses or evicts requests."
- **VRAM vs RAM bar** for Ollama: the part of the model that did not fit on the GPU runs
  slower on the CPU.

## Next

- A port list setting, plus auto-discovery by process name (doc 03 `find`).
- LM Studio (`/v1/models`), SGLang (`/metrics`) and TGI (`/metrics`, `tgi_*`).
- A history chart: KV usage against requests running over the last 5 minutes.
