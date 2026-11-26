# Changelog

Notes for each version are shown in the in-app update pop-up and on the website's releases page.

## [0.5.0] - 2026-11-26

### System (new first tab)
- Reads docker-compose files, Dockerfiles and nginx configs. Each container is a box with its image, ports, memory limit and GPU, and your code sits inside the container that builds it. The links between boxes come from env URLs, depends_on or proxy_pass.
- Memory split: how the compose limits divide your RAM, plus Redis maxmemory, Ollama keep-alive and parallelism, and vLLM GPU share.
- Example run: press Play, the screen preview types a question and presses the button, and the camera follows it through the proxy, the route, your memory/cache code, the tables, the AI pipeline and the containers it calls, then back. Each step shows the line that proves it.
- Without a compose file, the boxes are your code tiers.

### Grounded AI tool
- `architekton run <repo>` returns the same example run as JSON (or `--text`) so a model can narrate it using only proven hops.

### UI map
- Screens on the left, where each element goes on the right (backend route with its tables and container, browser-only, or not wired), joined by wires. "Play run" jumps to the System view.

### DSA tools (new sidebar)
- Structures in your code, structures hidden inside your tools (vector index graph, inverted index, top-k heap, agent state graph, ring buffer, LRU, KV block table, route trie, B-tree), real-life problems mapped to structures and to functions in your project, and slow spots.

### Recognition
- Hybrid search: sparse vectors (SPLADE, fastembed), Qdrant prefetch + RRF, Weaviate hybrid, Postgres full-text. Redis capped history and TTL caches.

## [0.4.0] - 2026-11-26

### How it works (new tab)
- AI pipeline: recognizes RAG, hybrid search, agents and agentic RAG, and draws them in lanes with the real settings from your code (chunk size, overlap, embedding model, top-k, reranker, model, context window). "Play a request" walks a question through every step.
- Data structures: finds stacks, queues, heaps, trees, graphs, recursion, memoization and binary search, and shows each one as a small animation.
- Memory and caching: names the techniques your code uses (PagedAttention, prefix caching, KV cache quantization, attention sinks, prompt caching), with a transformer walk-through and a KV cache calculator.
- Libraries: about 58 known AI and data libraries explained in one line each, plus Google OKF knowledge folders.

### Workflow button
- At every zoom level: the clean path through a function in green, decisions in amber, possible error exits in red, with a step-by-step player.

### Live (new tab)
- Memory, every CPU core, top processes with trends, one process's memory explained, its threads, and local model servers (Ollama, llama-server, vLLM) with KV cache usage. No admin rights needed.

### Demo
- New AI demo project on the welcome screen: RAG + agent + a tiny GPT.

## [0.3.2] - 2026-11-25

### Updates
- New "Check for updates" button in the top bar. Tells you when you are already on the latest version.
- This release is the first one delivered to 0.3.1 through the new update pop-up.

### Website
- Download buttons always point at the newest release, with file sizes, read live from GitHub.
- New Releases page with every version's notes and installers.
- New Code signing policy page explaining what is signed and how to verify a download.

## [0.3.1] - 2026-11-25

### Updates
- Auto-update now actually ships: releases include a signed `latest.json` and per-platform signatures, so installed apps find new versions on their own.
- New update pop-up with release notes, a real download progress bar, Later and Skip this version.
- Checks 4 seconds after launch and every 6 hours. Packages are signature-checked before install.
- Windows installs updates in passive mode (progress only, no wizard).

### Trust
- Every release lists `SHA256SUMS.txt` and carries GitHub build-provenance attestations.
- Release pipeline ready for free Windows signing (SignPath Foundation) and GPG-signed Linux AppImages.
