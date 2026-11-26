# 14. Agent loops

**Status: done** (static). Fixture: `fixtures/repos/rag-agent/app/agent.py`.

## What an agent loop is, for a student

1. The model reads the conversation and the tool list.
2. It decides: answer now, or call a tool with arguments.
3. The tool runs; its result is appended to the conversation.
4. Back to 1, until the model answers or a step limit is hit.

## Recognition

| Element | Evidence |
|---|---|
| Decider (router) | LangGraph `StateGraph` + `add_conditional_edges`; `should_continue`-style functions; LangChain `AgentExecutor`; OpenAI/Anthropic `tools=[...]` |
| Tools | `@tool` functions, `bind_tools([...])`, `Tool(...)` |
| LLM | chat model constructors (`ChatOllama`, `ChatOpenAI`, `Anthropic().messages.create`) |
| Loop | the edge Tool → LLM, flagged `loopBack`, drawn red and dashed with a moving dash |

Retrieval exposed as a tool (the `search_book` pattern) makes the system **Agentic
RAG**: the model decides when to retrieve, so retrieval sits inside the loop.

## Small decision models

The "decision model" in the loop can be small. Qwen3-0.6B, or a fine-tuned classifier,
can route between "answer", "retrieve" and "tool X". Doc 21 covers how FolderKnower
itself uses small models, and how the view will label a router model by size when the
code names one.

## Next

- Max-iteration and recursion-limit settings shown on the loop edge.
- Multi-agent graphs: subgraphs as nested lanes (LangGraph `add_node(subgraph)`,
  CrewAI, AutoGen).
