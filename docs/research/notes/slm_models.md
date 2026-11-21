# Graph-grounded SLMs, code embeddings, code role classifiers and small open-weight models for offline desktop use (2025–2026)

Research date: 2026-11-21. Model facts (parameter counts, licenses, architectures, GGUF file sizes) come from the Hugging Face Hub API through the HF MCP tools (`hub_repo_details`, `hf_fs find`), unless another source is cited. Parameter counts are as the Hub reports them, which includes embedding parameters.

## Q1. Which 2025–2026 papers show that grounding in code graphs or KGs reduces hallucination? What techniques do they use (subgraph serialization, citations to node ids, JSON-schema constrained decoding)?

### Takeaway
I found no 2025–2026 paper that tests *small* models (1.5B–4B) grounded in a *verified code graph* for *code explanation*. The closest work is on repository knowledge graphs used for retrieval and code generation (RANGER, KG-based repo codegen, Programming Knowledge Graph). Separate lines of work cover decoding-time graph grounding (GRAD), entity-tracing hallucination detection for code summaries (ETF), and JSON-schema constrained decoding (JSONSchemaBench, schema-key wording). The design "serialize a verified subgraph, make the model cite node ids, constrain output with a JSON schema" has to be assembled from these pieces. No single paper validates it.

### Cited Findings
**Repository and code knowledge graphs for retrieval and generation**
- RANGER (Sep 2025, arXiv 2509.25257) builds a knowledge graph of a whole repository. It captures "hierarchical and cross-file dependencies down to the variable level" and adds textual descriptions and embeddings to graph nodes. Entity queries are answered by Cypher lookups and natural-language queries by MCTS-guided graph exploration. On CodeSearchNet and RepoQA it beats retrieval baselines that use embeddings from models such as Qwen3-8B. On CrossCodeEval, RANGER combined with BM25 gives the highest exact match among the RAG methods tested — [HF Papers 2509.25257](https://huggingface.co/papers/2509.25257)
- "Knowledge Graph Based Repository-Level Code Generation" (May 2025, arXiv 2505.14394) represents repositories as graphs that capture structural and relational information. It uses hybrid retrieval to track inter-file modular dependencies and reports that it "significantly outperforms the baseline" on EvoCodeBench. The summary gives no exact numbers — [HF Papers 2505.14394](https://huggingface.co/papers/2505.14394)
- Programming Knowledge Graph, PKG (v2 Jan 2026, arXiv 2601.20810) splits code and text into fine-grained nodes and prunes the tree to make retrieval more precise. It reduces hallucinations with a re-ranker that also considers non-RAG solutions. It reports up to 20% pass@1 gains on HumanEval/MBPP and a 34% improvement over baselines on MBPP. Replication code is at github.com/iamshahd/ProgrammingKnowledgeGraph — [HF Papers 2601.20810](https://huggingface.co/papers/2601.20810)
- Earlier related systems: CodexGraph (2024) links LLMs to code graph databases through graph query interfaces — [HF Papers 2408.03910](https://huggingface.co/papers/2408.03910). GraphGen4Code (2020) is a toolkit for building code knowledge graphs — [HF Papers 2002.09440](https://huggingface.co/papers/2002.09440)
- "Code Summarization Beyond Function Level" (Feb 2025) finds that code summarization models benefit from class-level and repository-level context — [HF Papers 2502.16704](https://huggingface.co/papers/2502.16704)

**Graph-based hallucination mitigation and detection**
- GRAD, Graph-Retrieved Adaptive Decoding (Nov 2025, arXiv 2511.03900), is a decoding-time method that needs no retraining. It builds a sparse token-transition graph from the next-token logits of a small retrieved corpus, then fuses the max-normalized graph logits with the model's logits. It reports up to 9.7% higher intrinsic accuracy, 8.6% lower hallucination rates and 6.9% greater correctness than greedy decoding. These results are on QA benchmarks, not code — [HF Papers 2511.03900](https://huggingface.co/papers/2511.03900)
- ETF, the Entity Tracing Framework (Oct 2024, arXiv 2410.14748), combines static analysis with LLMs to detect hallucinations in code summaries. It introduces a dataset of about 10K samples for hallucination detection in code summarization. Its core idea is to trace the code entities that a summary mentions back to the source, which maps directly to checking that cited node ids exist — [HF Papers 2410.14748](https://huggingface.co/papers/2410.14748)
- "LLM Hallucinations in Practical Code Generation" (repository level) proposes a taxonomy of hallucinations and validates a RAG-based mitigation — [HF Papers 2409.20550](https://huggingface.co/papers/2409.20550)
- Hallucination-evaluation work that uses KGs: MultiHal (2025) is a multilingual benchmark for KG-grounded hallucination evaluation — [HF Papers 2505.14101](https://huggingface.co/papers/2505.14101). GraphEval (2024) combines KGs with NLI to detect and correct hallucinations — [HF Papers 2407.10793](https://huggingface.co/papers/2407.10793)

**Citations and attribution**
- LAQuer (Jun 2025) localizes spans of generated output back to source material for user-directed attribution — [HF Papers 2506.01187](https://huggingface.co/papers/2506.01187)
- CaLM (2024) uses smaller LMs to check that grounded responses agree with the sources they cite, without fine-tuning. This supports using a small model as a verifier — [HF Papers 2406.05365](https://huggingface.co/papers/2406.05365)
- "Why RAG Fails: A Graph Perspective" (May 2026) analyzes RAG failures through attribution graphs and attributes them to poor integration of external evidence — [HF Papers 2605.14192](https://huggingface.co/papers/2605.14192)

**Constrained and structured decoding**
- JSONSchemaBench, "Generating Structured Outputs from Language Models: Benchmark and Studies" (Jan 2025, Geng, Moskal, Nori et al.), finds that constrained decoding frameworks have standardized on JSON Schema. It benchmarks how compliant and efficient these frameworks are — [HF Papers 2501.10868](https://huggingface.co/papers/2501.10868)
- "Schema-Key Wording as an Instruction Channel" (Sep 2026, arXiv 2604.14862) finds that under constrained decoding, changing only the wording of schema keys "can substantially affect accuracy" across seven LMs, in both directions. Prompt-level and schema-level instructions interact non-additively. The study uses GSM8K and Math500, and the evidence is stronger on GSM8K — [HF Papers 2604.14862](https://huggingface.co/papers/2604.14862)
- TruncProof (Jun 2026) is grammar-constrained JSON generation that guarantees syntactically valid JSON within a token limit, using LL(1) parser properties — [HF Papers 2605.13076](https://huggingface.co/papers/2605.13076)
- Grammar-Constrained Decoding for structured NLP tasks without fine-tuning (2023) is foundational — [HF Papers 2305.13971](https://huggingface.co/papers/2305.13971)

### Inferences
- A defensible design that combines the work above: (a) retrieve a subgraph from the verified code graph (RANGER/PKG-style hybrid of entity lookup and embedding retrieval); (b) serialize it with stable node ids; (c) make the SLM emit JSON whose `claims[].node_ids` must come from the given ids, by building the JSON-schema enum dynamically from the retrieved subgraph ids; (d) after generation, check that every referenced entity exists and that its edges are consistent (ETF-style entity tracing). Step (c) turns "cite only real nodes" into a grammar guarantee rather than a hope. Note that this guarantees only that cited ids are valid ids, not that a claim is faithful to its node.
- Because schema-key wording changes accuracy (2604.14862), keys such as `"evidence_node_ids"` or `"summary_grounded_in_callees"` act as instructions and are worth A/B testing.
- GRAD-style logit fusion is an optional extra for llama.cpp or candle, since both expose logits. It is unproven for code.

### Gaps
- I found no paper that measures hallucination rates of 1.5B–4B models explaining code with versus without verified-graph grounding. A team building this would likely need its own evaluation; ETF's dataset is a candidate.
- I did not read the full texts of RANGER, PKG or 2505.14394, so I could not confirm their exact subgraph serialization formats (triples, adjacency lists or Cypher results). The findings above rely on the abstracts.
- I found no 2025–2026 paper that pairs node-id citation constraints with JSON-schema enums specifically for code.

## Q2. Which embedding model gives the best CoIR / code-retrieval score per parameter? Which run in candle and which in llama.cpp?

### Takeaway
For code retrieval quality per parameter, **jina-code-embeddings-0.5b** leads: 78.41% overall across 25 code benchmarks at 494M parameters, close to voyage-code-3. It is licensed **CC-BY-NC-4.0**, though, which rules it out for an open-source app distributed for any use. The best permissively licensed options are **nomic-ai/CodeRankEmbed** (137M, MIT, CoIR 60.1, nomic_bert architecture supported in candle) and **Qwen/Qwen3-Embedding-0.6B** (596M, Apache-2.0, 73.49% on the Jina 25-benchmark suite, qwen3 architecture in candle, official GGUF). **EmbeddingGemma-300m** uses the Gemma license and is gated, so treat it with caution.

### Cited Findings
**Shortlist (verified on HF Hub)**

| Repo id | Params (Hub) | Arch | License | Context | Code score | GGUF |
|---|---|---|---|---|---|---|
| `jinaai/jina-code-embeddings-0.5b` | 494M | qwen2 (from Qwen2.5-Coder-0.5B) | **CC-BY-NC-4.0** | 32,768 max (examples use 8,192) | 78.41% overall / 78.72% MTEB-Code avg | `jinaai/jina-code-embeddings-0.5b-GGUF` (Q8_0 0.53 GB, IQ4_XS 0.35 GB, F16/BF16 0.99 GB) |
| `jinaai/jina-code-embeddings-1.5b` | 1,544M | qwen2 (from Qwen2.5-Coder-1.5B) | **CC-BY-NC-4.0** | 32,768 | 79.04% overall / 78.94% MTEB-Code | `jinaai/jina-code-embeddings-1.5b-GGUF` (Q8_0 1.65 GB, IQ4_XS 0.90 GB) |
| `Qwen/Qwen3-Embedding-0.6B` | 596M | qwen3 | Apache-2.0 | unknown (not verified) | 73.49% overall / 74.69% MTEB-Code (Jina's table) | `Qwen/Qwen3-Embedding-0.6B-GGUF` (Q8_0 0.64 GB, f16 1.20 GB) |
| `nomic-ai/CodeRankEmbed` | 137M | nomic_bert | MIT | 8,192 | CoIR NDCG@10 60.1; CSN MRR 77.9 | community only, e.g. `brandtcormorant/CodeRankEmbed-Q8_0-GGUF`, `brandtcormorant/CodeRankEmbed-Q4_K_M-GGUF` |
| `nomic-ai/nomic-embed-code` | 7,071M | qwen2 (from Qwen2.5-Coder-7B) | Apache-2.0 | not verified | not verified here | `nomic-ai/nomic-embed-code-GGUF` (Q4_K_M 4.38 GB) |
| `jinaai/jina-embeddings-v2-base-code` | 161M | JinaBERT (ALiBi) | Apache-2.0 | 8,192 (ALiBi) per Jina docs, not re-verified | CoIR 58.4; CSN 67.2 (per CodeRankEmbed card) | ONNX in repo; GGUF not verified |
| `ibm-granite/granite-embedding-english-r2` | 149M | modernbert | Apache-2.0 | not verified | not verified | not verified |
| `google/embeddinggemma-300m` | 303M | gemma3_text | **Gemma license, gated** | not verified (gated card) | not verified | not found in `unsloth/embeddinggemma-300m-GGUF` |
| `codesage/codesage-small-v2` | not shown | custom CodeSage (custom_code) | Apache-2.0 | not verified | v1 small CoIR 54.4 (CodeRankEmbed card) | no |
| `Salesforce/SFR-Embedding-Code-400M_R` | 434M | "new" (GTE-style, custom_code) | **CC-BY-NC-4.0** | not verified | not verified | ONNX in repo |

Sources: [jina-code-embeddings-0.5b](https://hf.co/jinaai/jina-code-embeddings-0.5b), [jina-code-embeddings-1.5b](https://huggingface.co/jinaai/jina-code-embeddings-1.5b), [Qwen3-Embedding-0.6B](https://hf.co/Qwen/Qwen3-Embedding-0.6B), [CodeRankEmbed](https://huggingface.co/nomic-ai/CodeRankEmbed), [nomic-embed-code](https://hf.co/nomic-ai/nomic-embed-code), [jina-embeddings-v2-base-code](https://hf.co/jinaai/jina-embeddings-v2-base-code), [granite-embedding-english-r2](https://hf.co/ibm-granite/granite-embedding-english-r2), [embeddinggemma-300m](https://hf.co/google/embeddinggemma-300m), [codesage-small-v2](https://hf.co/codesage/codesage-small-v2), [SFR-Embedding-Code-400M_R](https://hf.co/Salesforce/SFR-Embedding-Code-400M_R). GGUF file sizes come from `hf_fs find` on each GGUF repo.

**Benchmark details**
- Jina's 25-benchmark code retrieval table reports Overall AVG and MTEB Code AVG: jina-code-embeddings-1.5b 79.04 / 78.94; jina-code-embeddings-0.5b 78.41 / 78.72; voyage-code-3 79.23 / 79.84; gemini-embedding-001 77.38 / 76.48; jina-embeddings-v4 (3.8B) 74.11 / 74.87; Qwen3-Embedding-0.6B 73.49 / 74.69. Nomic-embed-code and CodeSage are not in this table — [Jina blog](https://jina.ai/news/jina-code-embeddings-sota-code-retrieval-at-0-5b-and-1-5b)
- Jina code embeddings use last-token pooling, Matryoshka dimensions 128/256/512/1024/1536 (1.5B default 1536), and five task prefix pairs (nl2code, code2code, code2nl, code2completion, qa). For example, an nl2code query starts with "Find the most relevant code snippet given the following query:" — [HF card 1.5b](https://huggingface.co/jinaai/jina-code-embeddings-1.5b)
- Jina says "1-4 bit GGUF quantizations" exist for both sizes — [Jina blog](https://jina.ai/news/jina-code-embeddings-sota-code-retrieval-at-0-5b-and-1-5b). The Hub listing confirms IQ1–IQ4, Q8_0, F16 and BF16 files, but **no Q4_K_M** file.
- CodeRankEmbed card, CoIR NDCG@10 / CSN MRR: CodeRankEmbed 137M 60.1 / 77.9; CodeSage-Small 130M 54.4 / 64.9; CodeSage-Base 356M 57.5 / 68.7; CodeSage-Large 1.3B 59.4 / 71.2; Jina-Code-v2 161M 58.4 / 67.2; Voyage-Code-002 56.3 / 68.5; OpenAI Ada-002 45.6 / 71.3. Queries require the prefix "Represent this query for searching relevant code" — [HF card](https://huggingface.co/nomic-ai/CodeRankEmbed)
- CoIR is the reference benchmark — [HF Papers 2407.02883](https://huggingface.co/papers/2407.02883). CORE-Bench (Jun 2026) is a newer benchmark aimed at code retrieval for agentic coding, including repository-level search — [HF Papers 2606.11864](https://huggingface.co/papers/2606.11864)
- EmbeddingGemma 2 (Oct 2026, multimodal) has been announced — [HF Papers 2610.37364](https://huggingface.co/papers/2610.37364). I did not verify a model repo or license.

**Runtime support**
- candle-transformers 0.11.1 includes modules `bert`, `jina_bert`, `nomic_bert`, `modernbert`, `qwen2`, `qwen3`, `quantized_qwen3`, `gemma3`, `quantized_gemma3`, `stella_en_v5`, `xlm_roberta`, `nvembed_v2`, `mistral`. There is **no `gte` module** — [docs.rs candle_transformers::models](https://docs.rs/candle-transformers/latest/candle_transformers/models/)
- HF Text Embeddings Inference (TEI), which is built on candle, lists support for Qwen3 (e.g. Qwen3-Embedding-0.6B), Gemma3 (embeddinggemma-300m), ModernBERT, NomicBERT (nomic-embed-text-v1.5/v2-moe), JinaBERT including `jinaai/jina-embeddings-v2-base-code`, Qwen2 (gte-Qwen2), Alibaba GTE (gte-large-en-v1.5, snowflake-arctic-embed-m-v2.0) and Mistral. It also lists `voyageai/voyage-4-nano` (340M, Qwen3 architecture) — [TEI supported models](https://huggingface.co/docs/text-embeddings-inference/supported_models)

### Inferences
- **Candle path:** CodeRankEmbed (nomic_bert, MIT, 137M) and Qwen3-Embedding-0.6B (qwen3, Apache-2.0) fit candle's architecture list directly. Jina-code-embeddings run on candle's `qwen2` model with last-token pooling, but the NC license blocks them. Jina-embeddings-v2-base-code works with `jina_bert`. Granite-embedding-english-r2 works with `modernbert`. These are architecture matches; I did not test pooling or tokenizer wiring for each model.
- **llama.cpp path:** official GGUFs exist for Qwen3-Embedding-0.6B (Q8_0 639 MB) and jina-code-embeddings. Community GGUFs exist for CodeRankEmbed (nomic-bert, which llama.cpp supports for nomic-embed-text). Nomic-embed-code at 4.4 GB (Q4_K_M) is too heavy for a lightweight desktop app.
- **Per-parameter ranking:** CodeRankEmbed reaches CoIR 60.1 at 137M, the best CoIR per parameter among permissive models in the CodeRankEmbed table. Qwen3-Embedding-0.6B is stronger in absolute terms (Jina table) but 4x larger. The two tables use different benchmark suites, so their numbers cannot be compared directly.
- **Recommendation for an Apache/MIT desktop app:** use CodeRankEmbed as the default (small, MIT, candle-native) and Qwen3-Embedding-0.6B as the higher-quality option. Avoid jina-code-embeddings and SFR-Embedding-Code (CC-BY-NC), and flag EmbeddingGemma (Gemma terms, gated download).

### Gaps
- I found no single leaderboard that scores CodeRankEmbed, Qwen3-Embedding-0.6B, EmbeddingGemma-300m, granite-embedding-r2 and voyage-4-nano on the same CoIR suite. The live MTEB leaderboard was not fetched.
- The arXiv HTML of the Jina paper (2508.21290) was rate-limited twice, so the per-task table is not here.
- I could not read EmbeddingGemma's card because it is gated, so its context length and code scores are unverified. Its GGUF availability is also unverified: `unsloth/embeddinggemma-300m-GGUF` had no Q4_K_M file, and I did not check other GGUF repos.
- I did not verify the license or HF repo of `voyageai/voyage-4-nano`. It appears only in the TEI list.
- I did not verify the context length of Qwen3-Embedding-0.6B or nomic-embed-code.

## Q3. Which 1.5B–4B SLM is best for grounded narration and license-compatible with an open-source desktop app?

### Takeaway
**Qwen/Qwen3-4B-Instruct-2507** (Apache-2.0, 256K context, non-thinking only, Q4_K_M 2.50 GB, candle `qwen3`/`quantized_qwen3`) is the strongest verified choice for grounded narration with structured output: IFEval 83.4, LiveCodeBench v6 35.1, MultiPL-E 76.8. A low-RAM tier would be **Qwen3-1.7B** (Q4_K_M 1.11 GB). The 2026 releases **Qwen3.5-4B/2B** and **Gemma 4 E2B/E4B** are now Apache-2.0, but candle support is missing or unclear for them, and they are multimodal and heavier. **Avoid or flag:** Gemma 3 (Gemma license), Llama 3.2 (Llama 3.2 Community License), and Qwen2.5-Coder-3B (`license: other`, the Qwen research license). Qwen2.5-Coder-1.5B is Apache-2.0.

### Cited Findings
**Shortlist (verified on HF Hub)**

| Repo id | Params (Hub) | Arch | License | Q4_K_M GGUF (size) | candle module |
|---|---|---|---|---|---|
| `Qwen/Qwen3-4B-Instruct-2507` | 4,023M | qwen3 | Apache-2.0 | `unsloth/Qwen3-4B-Instruct-2507-GGUF` 2.50 GB | qwen3, quantized_qwen3 |
| `Qwen/Qwen3-1.7B` | 2,032M | qwen3 | Apache-2.0 | `unsloth/Qwen3-1.7B-GGUF` 1.11 GB | qwen3, quantized_qwen3 |
| `Qwen/Qwen3.5-4B` (2026-03) | 4,660M | qwen3_5, image-text-to-text | Apache-2.0 | `unsloth/Qwen3.5-4B-GGUF` 2.74 GB | **no qwen3_5 module listed** |
| `Qwen/Qwen3.5-2B` (2026-03) | 2,274M | qwen3_5 | Apache-2.0 | `unsloth/Qwen3.5-2B-GGUF` 1.28 GB | **no qwen3_5 module listed** |
| `google/gemma-4-E4B-it` (2026) | 7,996M total (effective-4B "E" design) | gemma4, any-to-any | Apache-2.0 | `unsloth/gemma-4-E4B-it-GGUF` 4.98 GB | `gemma4` module exists; coverage of E-variants unverified |
| `google/gemma-4-E2B-it` (2026) | 5,123M total | gemma4 | Apache-2.0 | `unsloth/gemma-4-E2B-it-GGUF` 3.11 GB | same as above |
| `Qwen/Qwen2.5-Coder-3B-Instruct` | 3,086M | qwen2 | **other (Qwen research license)** | `Qwen/Qwen2.5-Coder-3B-Instruct-GGUF` 2.10 GB | qwen2, quantized_qwen2 |
| `Qwen/Qwen2.5-Coder-1.5B-Instruct` | 1,544M | qwen2 | Apache-2.0 | `Qwen/Qwen2.5-Coder-1.5B-Instruct-GGUF` 1.12 GB | qwen2, quantized_qwen2 |
| `microsoft/Phi-4-mini-instruct` | 3,836M | phi3 | MIT | `unsloth/Phi-4-mini-instruct-GGUF` 2.49 GB | phi3, quantized_phi3 |
| `HuggingFaceTB/SmolLM3-3B` | 3,075M | smollm3 | Apache-2.0 | `ggml-org/SmolLM3-3B-GGUF` (SmolLM3-Q4_K_M.gguf) 1.92 GB | a `smol` module exists; smollm3 coverage unverified |
| `ibm-granite/granite-4.0-micro` | 3,403M | granitemoehybrid | Apache-2.0 | `ibm-granite/granite-4.0-micro-GGUF` 2.10 GB (signed .sig) | granitemoehybrid |
| `google/gemma-3-4b-it` | 4,300M | gemma3 (multimodal) | **Gemma license, gated** | `unsloth/gemma-3-4b-it-GGUF` 2.49 GB | gemma3, quantized_gemma3 |
| `google/gemma-3n-E4B-it` | 7,850M total | gemma3n | **Gemma license, gated** | not checked | no gemma3n module listed |
| `meta-llama/Llama-3.2-3B-Instruct` | 3,213M | llama | **Llama 3.2 Community License, gated** | `bartowski/Llama-3.2-3B-Instruct-GGUF` 2.02 GB | llama, quantized_llama |

Sources: HF Hub details for each repo ([Qwen3-4B-Instruct-2507](https://hf.co/Qwen/Qwen3-4B-Instruct-2507), [Qwen3-1.7B](https://hf.co/Qwen/Qwen3-1.7B), [Qwen3.5-4B](https://hf.co/Qwen/Qwen3.5-4B), [Qwen3.5-2B](https://hf.co/Qwen/Qwen3.5-2B), [gemma-4-E4B-it](https://hf.co/google/gemma-4-E4B-it), [gemma-4-E2B-it](https://hf.co/google/gemma-4-E2B-it), [Qwen2.5-Coder-3B-Instruct](https://hf.co/Qwen/Qwen2.5-Coder-3B-Instruct), [Qwen2.5-Coder-1.5B-Instruct](https://hf.co/Qwen/Qwen2.5-Coder-1.5B-Instruct), [Phi-4-mini-instruct](https://hf.co/microsoft/Phi-4-mini-instruct), [SmolLM3-3B](https://hf.co/HuggingFaceTB/SmolLM3-3B), [granite-4.0-micro](https://hf.co/ibm-granite/granite-4.0-micro), [gemma-3-4b-it](https://hf.co/google/gemma-3-4b-it), [gemma-3n-E4B-it](https://hf.co/google/gemma-3n-E4B-it), [Llama-3.2-3B-Instruct](https://hf.co/meta-llama/Llama-3.2-3B-Instruct)). GGUF sizes come from `hf_fs find`. Candle module list: [docs.rs candle-transformers 0.11.1](https://docs.rs/candle-transformers/latest/candle_transformers/models/).

**Benchmarks**
- Qwen3-4B-Instruct-2507 (from its card): 4.0B parameters (3.6B non-embedding), 36 layers, GQA 32 query / 8 KV heads, 262,144-token native context, non-thinking only. Scores: LiveCodeBench v6 (25.02–25.05) 35.1; MultiPL-E 76.8; Aider-Polyglot 12.9; IFEval 83.4; BFCL-v3 61.9; MMLU-Pro 69.6; Arena-Hard v2 43.4. The original Qwen3-4B non-thinking scores were LiveCodeBench 26.4, MultiPL-E 66.6 and IFEval 81.2. The card says llama.cpp, Ollama, LM Studio and MLX-LM support Qwen3 — [HF card](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507)
- Phi-4-mini-instruct (3.8B, MIT), from its card: Arena Hard 32.8, BigBench Hard CoT 70.4, MMLU 67.3, MMLU-Pro 52.8, GPQA 25.2, TruthfulQA MC2 66.4. In the same table Llama-3.2-3B-Ins scores Arena Hard 17.0 / MMLU-Pro 39.2, and Qwen2.5-3B-Ins scores 32.0 / 44.7 — [HF card](https://huggingface.co/microsoft/Phi-4-mini-instruct). I did not reach the code rows (HumanEval etc.) of the card.
- Gemma 4 was released under Apache-2.0, a change from earlier Gemma terms — [VentureBeat](https://venturebeat.com/technology/google-releases-gemma-4-under-apache-2-0-and-that-license-change-may-matter); [The Register, 2026-04-02](https://www.theregister.com/2026/04/02/googles_gemma_4_open_weights/). The HF Hub tags `google/gemma-4-E4B-it` and `-E2B-it` as `license:apache-2.0`.

### Inferences
- **Best fit for grounded narration:** Qwen3-4B-Instruct-2507. It has the best instruction-following and tool/JSON behavior (IFEval, BFCL) of the verified Apache/MIT models. Its non-thinking design keeps latency low, the 256K context fits serialized subgraphs, and it runs on both candle and llama.cpp. Its GBNF/JSON-schema constrained decoding is a llama.cpp feature, not a model feature, so it applies to any of these GGUFs.
- **Fallback for low RAM:** Qwen3-1.7B (1.1 GB at Q4_K_M) or Qwen2.5-Coder-1.5B-Instruct (Apache, code-tuned).
- **MIT alternative:** Phi-4-mini-instruct, at the same Q4_K_M size as Qwen3-4B.
- **2026 candidates:** Qwen3.5-4B is plausibly stronger, but it is a vision-language architecture (`qwen3_5`) without a candle module. That would tie a candle-first app to llama.cpp, or require porting the model. Gemma 4 E4B is Apache-2.0 now but its Q4_K_M is about 5 GB, about twice Qwen3-4B's.
- **Licenses:** Gemma 3/3n terms carry use-policy flow-down obligations, and Llama 3.2 terms add attribution and acceptable-use requirements. Both complicate bundling the weights in an OSS app. The Qwen2.5-Coder-3B research license is the trap in the Qwen family: the 1.5B and 7B models are Apache-2.0, but the 3B model is not.

### Gaps
- I did not collect code benchmarks (HumanEval/MBPP/LiveCodeBench) on a common harness for Qwen3-1.7B, SmolLM3-3B, granite-4.0-micro, Gemma 4 E2B/E4B or Qwen3.5-2B/4B, so they are unknown here.
- I found no benchmark of grounded narration or faithfulness for these SLMs on code-graph inputs.
- I did not verify whether candle's `gemma4` module supports the E2B/E4B per-layer-embedding variants, or whether `smol` covers SmolLM3. I also did not verify llama.cpp support for the `qwen3_5` and `gemma4` architectures directly; unsloth GGUFs exist, which strongly suggests llama.cpp runs them.
- Granite 4.0 micro is tagged `granitemoehybrid`. Whether this specific micro model is dense or hybrid-Mamba was not verified.

## Q4. What lightweight classifiers exist for function roles (small fine-tuned encoders, zero-shot via embeddings and prototypes)?

### Takeaway
I found no 2025–2026 paper or off-the-shelf model specifically for classifying function or method roles (e.g. controller, repository, util, test, entry point) with lightweight models. The practical approach is a nearest-prototype classifier over code embeddings, following Prototypical Networks: average the embeddings of a few labeled examples per role, then classify by cosine similarity. The embedder would be CodeRankEmbed or Qwen3-Embedding-0.6B. Graph features from the code graph can be added, such as fan-in/fan-out, I/O calls and test-file location.

### Cited Findings
- Prototypical Networks classify by distance to per-class mean embeddings — the basis of training-free few-shot classification — [HF Papers 1703.05175](https://huggingface.co/papers/1703.05175)
- Training-free prototype calibration (TEEN) adds new classes few-shot by fusing new prototypes with weighted base prototypes. This applies to adding user-defined roles incrementally — [HF Papers 2312.05229](https://huggingface.co/papers/2312.05229)
- Jina code embeddings expose a `code2nl` task prefix pair, and Matryoshka truncation down to 128 dimensions with "minimal performance loss." That suits cheap prototype matching, but the license is NC — [HF card](https://huggingface.co/jinaai/jina-code-embeddings-1.5b)
- RANGER adds textual descriptions and embeddings to graph nodes. This is the same node-level representation a role classifier would use — [HF Papers 2509.25257](https://huggingface.co/papers/2509.25257)

### Inferences
- Pipeline: embed each function as signature, docstring and body with a `code2nl`-style prefix. Compare it against role prototypes built from (a) 5–20 labeled examples per role and/or (b) natural-language role descriptions embedded as queries. Then fuse the similarity with structural features from the graph using a tiny logistic regression. All of this runs in milliseconds on CPU in candle.
- For higher accuracy, fine-tune a small encoder (ModernBERT-based granite-embedding-english-r2 at 149M, or CodeRankEmbed at 137M) with a classification head on a few thousand weakly labeled functions. Weak labels can come from directory and naming heuristics such as `tests/`, `*Controller`, `*Repository` and `main`.
- An SLM with a JSON-schema enum of roles can serve as a fallback zero-shot labeler or adjudicator for low-confidence cases.

### Gaps
- I found no benchmark dataset for function or method role classification ("method stereotypes") from 2025–2026. The HF papers search for "method role classification stereotypes code" returned only social-stereotype papers. Earlier software-engineering work on method stereotypes, such as Dragan et al., exists, but I did not search for it or verify it here.
- I found no accuracy numbers for embedding-prototype role classification on code.
