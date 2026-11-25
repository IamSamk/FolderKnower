# 15. Transformer internals and KV cache

**Status: done.** Two components:

- `TransformerBlock` (in `InsightsView.tsx`, under Memory and caching);
- `KvCalculator` (`components/insights/KvCache.tsx`), also in the Live view next to
  the machine's real RAM.

## Transformer, one token at a time

A vertical strip: Tokens → Embedding → LayerNorm → Self-attention → LayerNorm → MLP →
Logits → next token. A highlight walks down it and shows one sentence per box. Boxes
whose layer exists in the analysed code (`CausalSelfAttention`, `MLP`, `LayerNorm`,
`Embedding`) carry an "in this code" chip, so the student maps the picture onto the
repo's `tiny_gpt.py`.

The attention sentence carries the key idea: each new token's query is compared with the
keys of all earlier tokens, and their K and V are **kept in the KV cache so they are not
recomputed**. That is why the cache grows with every generated token.

## KV cache, computed

```
KV bytes = 2 (K and V) × layers × kv_heads × head_dim × tokens × bytes per value
```

| Preset | Layers | KV heads | Head dim | Note |
|---|---|---|---|---|
| Qwen3-0.6B | 28 | 8 | 128 | |
| Qwen3-4B | 36 | 8 | 128 | GQA: 32 query heads share 8 KV heads |
| Llama-3.1-8B | 32 | 8 | 128 | |
| Mistral-7B v0.1 | 32 | 8 | 128 | sliding window 4096 |
| GPT-2 small | 12 | 12 | 64 | plain multi-head attention |

Example: Qwen3-4B at fp16 is 2 × 36 × 8 × 128 × 2 B = **144 KB per token**, so one 8k
request holds about 1.1 GB of KV.

## Controls

- Context window (512 → 131k), concurrent requests, and average fill of the window.
- KV cache type (fp16, fp8/q8_0, q4_0) and weight storage (fp16, 8-bit, 4-bit Q4_K_M).
- **PagedAttention:** 16-token blocks, allocated on demand. The view shows what
  reserving the full window per request would cost, and what paging saves.
- **Prefix caching:** a shared prefix of N tokens is stored once, not once per request.
- **Sliding window and attention sinks:** the KV cap becomes W (+4 sinks), and a token
  strip animates the rolling buffer (kept, sink, evicted).

A stacked bar (weights + KV) is drawn against a red dashed line at this computer's RAM
(Live view), with the plain sentence "This does not fit in this computer's memory" when
it does not.
