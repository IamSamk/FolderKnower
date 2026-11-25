# 12. AI architecture recognition

**Status: done.** `ontology/ai-catalog.yaml` and `crates/ark-patterns/src/ai.rs`.

## Catalog

YAML, loaded once (`LazyLock`). Each component entry has:

- `id`, `role` and `library`;
- `match`: regexes over `receiver.callee`;
- `requires`: import prefixes that must be present, which prevents `Chroma` matching a
  random class;
- `params`: keyword names to copy, and `positional` for the first-argument model name;
- `explain`: one student sentence.

The roles cover loader, splitter, embedder, vector store, keyword index, retriever,
fusion, reranker, prompt, router, LLM, tool, agent, memory, cache, guardrail, evaluator,
model server, tokenizer, neural layer and trainer. Entries exist for LangChain,
LlamaIndex, Haystack, DSPy, sentence-transformers, Chroma, FAISS, Qdrant, pgvector,
rank_bm25, LangGraph, the OpenAI and Anthropic SDKs, Ollama, vLLM, llama-cpp-python,
transformers and torch.nn.

## Beyond calls

- Python `@tool` decorators become Tool stages named after the function.
- Custom `torch.nn.Module` subclasses become neural layer stages. Names like
  `CausalSelfAttention`, `MLP` or `Block` get a tailored explanation.
- LangGraph `add_node("agent", ...)` names go into the router's `nodes` param.
- Repeated layer types record `uses: 3`.

## Assembly

Edges follow the canonical data flow:

- the main chain: Loader → Splitter → Embedder → VectorStore → Retriever → Fusion →
  Reranker → Prompt → LLM;
- a keyword branch: Splitter → KeywordIndex → Fusion;
- memory, cache and guardrail edges into the LLM;
- the **agent loop**: LLM → decider (router or agent) → Tool → LLM, with the closing
  edge flagged `loopBack`.

The system kind takes the strongest match: AgenticRag > Agent > HybridRag > Rag >
ModelServing > ModelTraining > Inference. Each kind has a one-line headline.

## Accuracy guard

A stage exists only when the call is in the code **and** its import is present. Params
are literal values only; a variable shows as its name (`d_model`), never a guess.
