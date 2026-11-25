# 17. Framework catalog and OKF

**Status: done.** `libraries:` in `ontology/ai-catalog.yaml` (58 entries);
`ark-core::detect_okf`.

## Named libraries

An import prefix match (`langgraph`, `@langchain/langgraph`, `sentence_transformers`)
adds a library card: name, category (agent framework, vector store, model serving,
quantization, RAG evaluation, …), one plain sentence, and the imports that proved it.

Matching is exact on the prefix boundary: `torch` matches `torch.nn` and `torch_geometric`
by design, but `torchvision` does not match `torch`. The Libraries tab groups cards by
category.

## Google Open Knowledge Format (OKF)

OKF (Google Cloud, June 2026) is a folder of Markdown files with YAML frontmatter, where
`type` is required and `index.md` / `log.md` are reserved names. It is how teams hand
curated knowledge to agents.

`detect_okf(root, files)` reports a bundle when a directory holds Markdown whose
frontmatter has `type:` **and** contains `index.md` or `log.md`. The evidence lists the
directory. The fixture `fixtures/repos/rag-agent/knowledge/` is a small OKF bundle that
the agent reads.

## Next

- An OKF view: concepts as cards, `log.md` as a timeline, links between notes drawn as
  edges.
- More formats: MCP server manifests, A2A agent cards, OpenAPI for tools, model cards
  (`README.md` with `model-index`), GGUF metadata.
