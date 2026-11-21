# Deterministic graphs beat models: Architekton's Phase 1 blueprint

The 2025–2026 evidence points one way. **Precise, deterministic structure should be Architekton's core product, and models should be thin optional layers on top.** Static call-graph tools still beat every one of 24 LLMs tested. PyCG reached 87.3% soundness against 62.6% for the best LLM ([arXiv 2410.00603](https://arxiv.org/html/2410.00603v2)). LLMs predict Big-O correctly only about 29% of the time ([arXiv 2503.15242](https://arxiv.org/html/2503.15242v2)). Small 1.7–4B models gain +17–21 points from 1–2-hop graph tools, while embeddings add nothing (-1.1) ([arXiv 2610.33958](https://arxiv.org/html/2610.33958v1)). AST-aware chunking (cAST) adds +1.8 to +4.3 Recall@5 with no model at all ([arXiv 2506.15655](https://arxiv.org/html/2506.15655v2)).

The concrete program for Architekton follows from that:

- **Indexing:** cAST chunking, a four-layer lexical-to-graph lookup keyed on SCIP symbols, and Merkle-tree incremental re-indexing.
- **Call graph:** provenance-tagged edges, with sampled stack edges promoted to "dynamic-observed".
- **Zoom levels:** a seeded Leiden hierarchy with "force" nodes for the middle levels.
- **Model layer:** a nearest-prototype role classifier on CodeRankEmbed (MIT, 137M) and grounded narration from Qwen3-4B-Instruct-2507 (Apache-2.0). Output goes through JSON-schema decoding whose node-id enum is built from the retrieved subgraph.
- **Telemetry helper:** a separately installed, narrowly scoped privileged helper per OS. On Linux that is a systemd service with only CAP_BPF and CAP_PERFMON. On Windows it is a service behind an ACL'd named pipe. On macOS it is an SMAppService daemon with an XPC code-signing requirement.

Several items stay unverified: the effect of the aya GPL license section, kernel-lockdown behavior, and candle support for Qwen3.5 and Gemma 4. They are flagged where they appear.

## Lexical anchors into SCIP-precise 1-hop graphs win repo-scale search

**The 2025–2026 schemas have converged** on a typed heterogeneous graph:

- **Nodes:** directory/package, file, class, function, and sometimes attribute.
- **Edges:** contain, import, call, and inherit.
- **Lookup:** a layered lexical index on fully qualified names.

LocAgent's four-layer sparse index runs in a fixed order: exact FQN, then a short-name multimap, then BM25 over entity IDs, then a chunk-to-entity inverted index. Paired with type-aware BFS, it reached **94.16% file Acc@5 on SWE-Bench-Lite** and cut cost from about $0.66 to $0.09 per example ([arXiv 2503.09089](https://arxiv.org/html/2503.09089)). CGM's ablation shows the retrieval front end carries the result. Removing the reranker drops SWE-bench Lite from 43.00% to 18.33%, and removing the whole Graph RAG stage drops it to 9.67% ([arXiv 2505.16901](https://arxiv.org/html/2505.16901v2)). LARGER takes ordinary lexical hits, "snaps" them to graph nodes, and expands with confidence-filtered K-hop neighborhoods. That adds **+13.9 file Acc@5 on LocBench** and also adds test-to-source and doc-to-source edges ([arXiv 2605.16352](https://arxiv.org/html/2605.16352)).

**Depth matters less than precision.** Each extra hop between an issue and the gold function multiplies the odds of correct localization by about 0.47–0.55, and the graph gain sits almost entirely at hops 0–1 ([arXiv 2610.33958](https://arxiv.org/html/2610.33958v1)). That study had to discard names with more than five candidate targets because it resolved calls by name. SCIP avoids exactly this problem, so Architekton's SCIP-resolved edges are a real advantage over the research baselines. The fuller 2026 systems also show the value of fusion. In SuperAGI's "Code Isn't Memory", an index combining vectors, a graph and BM25 over identifiers raised localization acc@5 from **44.3 to 84.5** and resolve rate from **41.9 to 50.4** (p=0.003) ([arXiv 2606.22417](https://arxiv.org/pdf/2606.22417)). In Agent Retrieval Bench, a vectorless RepoMap signal had the best gold-context-per-token yield (BCY@8k 0.3788). Reciprocal-rank fusion with embeddings raised MRR from 0.2296 to 0.2713 ([arXiv 2607.24882](https://arxiv.org/html/2607.24882v1)).

**cAST is the reference chunker.** It parses with Tree-sitter, emits a whole file when it fits the budget, and otherwise greedily merges adjacent siblings under a budget of **2000 non-whitespace characters**, recursing into any node that is too large. The merge step matters: split-only chunking hurt every retriever ([arXiv 2506.15655](https://arxiv.org/html/2506.15655v2)). No 2025–2026 successor improves on its algorithm. Its gains were measured only with embedding retrievers, not BM25.

**Non-code files must be first-class.** In MULocBench, 23.5% of fixes touch tests, 23.5% touch docs and 15.2% touch config. Even the best file-level Acc@5 is 35.2% ([arXiv 2509.25242](https://arxiv.org/html/2509.25242)). LongCodeBench shows that dumping a whole repository into long context fails: Claude 3.5 Sonnet falls from 29% to 3% as context grows ([arXiv 2505.07897](https://arxiv.org/abs/2505.07897v2)).

For hierarchy maintenance, academic coverage is thin. SuperAGI uses Merkle-tree diffs but did not evaluate that path ([arXiv 2606.22417](https://arxiv.org/pdf/2606.22417)). Stack graphs remain the principled model: each file produces a self-contained fragment that never needs re-analysis when other files change ([arXiv 2211.01224](https://arxiv.org/abs/2211.01224v1)). Bottom-up summarization along the AST hierarchy matters because summarizing a whole file in one prompt missed 9–24% of entities ([arXiv 2501.07857](https://arxiv.org/html/2501.07857v1)). Nothing in this literature builds Architekton's upper tiers, System and subsystem clusters, above the directory level. That level is a differentiator, and nothing in the literature checks it yet.

## Deterministic call graphs need provenance, not ML pruning

**Learned pruning is the wrong investment.** An ICSE 2026 study hand-checked 348 edges that earlier work had "assumed false" and found 313 of them were true. With validated labels, baseline precision is already **0.97**, and pruning moves it only to 0.96–0.98 ([preprint](https://mohpydev.github.io/papers/call-graph-pruning-preprint.pdf); [DOI 10.1145/3744916.3773247](https://doi.org/10.1145/3744916.3773247)). The real gap is recall, and the most effective fixes are hybrids:

- **Approximate interpretation:** Jelly force-executes modules to collect hints, raising JS call-edge recall from **75.9% to 88.1%** at a cost of about 1.5% precision. The pre-pass averages 4.5 s per program ([PLDI 2024 paper](https://www.cs.au.dk/~amoeller/papers/approx/paper.pdf)).
- **Runtime entry points:** a 5-minute dynamic run supplied extra entry points that grew mean Android call graphs from **50,626 to 65,534 nodes**. Taint flows rose from 105 to 152, and 10 of 10 sampled new flows were true ([FSE 2025 IVR](https://jacquesklein2302.github.io/papers/2025-FSE-IVR-call_graph.pdf)).
- **Ranked candidates:** for genuinely unresolved JS call sites, the GNN in GRAPHIA puts the correct target in its top 5 **72%** of the time ([arXiv 2506.18191](https://arxiv.org/abs/2506.18191)). That supports showing ranked candidates rather than drawing silent edges.

No 2025–2026 call-graph benchmark exists for Go or Rust, and no study measures SCIP-index call-graph accuracy specifically. Both are open gaps.

**The 5-level zoom should be a seeded Leiden hierarchy.** Guided community detection adds weighted synthetic "force" nodes for domain, data stores and end-to-end tests to an API call graph before running Leiden. Sweeping the weights produced 2,048 decompositions, of which **108 were Pareto-optimal**. On Train Ticket, a balanced decomposition reached NCR 0.56, while unconstrained clustering violated every boundary (DMR = DAVR = 1.0) ([Univ. Vienna PDF](https://eprints.cs.univie.ac.at/8461/1/paper_140.pdf)). ArchAgent uses LLMs only on top of AST reference graphs, splitting the repo into token-balanced groups of ⌈T/M⌉ with about 10% overlap. It reported F1 0.966 against 0.860 for DeepWiki, but on only 8 repos rated by the authors' side ([arXiv 2601.13007](https://arxiv.org/pdf/2601.13007)). LD-Leiden addresses incremental re-clustering after edits, but it was seen only by title ([arXiv 2502.18497](https://arxiv.org/abs/2502.18497)). No 2025–2026 paper evaluates dominator-tree or k-core collapsing for visualization.

**Complexity should come from deterministic rules, with optional empirical checks.** BigO(Bench)'s labeling recipe agreed with human labels on 92% of time samples and 84% of space samples. It scales inputs, profiles each run, fits each complexity class with NNLS, and prefers the simpler class on near-ties ([arXiv 2503.15242](https://arxiv.org/html/2503.15242v2)). CodeComplex supplies 4,900 Java and 4,900 Python programs labeled with worst-case complexity, usable as an offline regression set ([EMNLP 2025 Findings](https://aclanthology.org/2025.findings-emnlp.1069/); [arXiv 2401.08719](https://arxiv.org/abs/2401.08719)).

## Telemetry fusion maps samples to Tree-sitter ranges through SCIP

**The OpenTelemetry eBPF profiler is the reference agent design:**

- **Unwinding:** native stacks via `.eh_frame`, needing no frame pointers or debug info.
- **Runtimes:** interpreter frames for Python, V8, JVM, .NET, Ruby, PHP, Perl and Erlang.
- **Overhead:** test limits of 1% CPU and 250 MB memory.
- **Status:** it emits the **alpha** OTel Profiles signal, and its minimum kernel keeps rising (5.10 or later for newer commits).

Source: [GitHub](https://github.com/open-telemetry/opentelemetry-ebpf-profiler).

Frame-pointer unwinding mis-attributes samples that land in function prologues. These are about **5.2% of x86 samples** and at least 6.0% on aarch64 ([Red Hat](https://developers.redhat.com/articles/2024/10/30/limitations-frame-pointer-unwinding)). SFrame is the kernel path forward. Its deferred-unwind base landed in Linux 6.17, with userspace support in glibc 2.42 and binutils 2.45 ([Phoronix](https://www.phoronix.com/news/Linux-6.17-Deferred-Unwinder)). The arm64 and s390 patches were still under review in 2026 ([LKML](https://lkml.org/lkml/2026/1/27/1225)).

No 2025–2026 paper fuses eBPF stack samples with AST or SCIP call graphs, so Architekton's attribution pipeline is novel engineering. Native frames resolve PC to symbol to file:line through DWARF, including inlined frames. Interpreted frames already arrive as file, function and line. Either way, the next step is a lookup in an interval tree of Tree-sitter function byte and line ranges, where the innermost enclosing node wins, followed by a join to the SCIP symbol. Edges seen in samples but missing from the static graph become "dynamic-only" edges, and sampled stack roots become reachability entry points, following the FSE 2025 pattern above. Async attribution (Tokio, goroutines, asyncio) and ETW-to-source mapping had **no verified 2025–2026 source** in these notes.

## Grounded small models: constrained citation is assembled, not proven

**No paper tests 1.5–4B models explaining code from a verified graph.** The design has to be assembled from three separate lines of work:

- **Repo knowledge graphs:** RANGER ([arXiv 2509.25257](https://huggingface.co/papers/2509.25257)) and PKG, which reports up to 20% pass@1 gains ([arXiv 2601.20810](https://huggingface.co/papers/2601.20810)).
- **Entity-tracing hallucination detection for code summaries:** ETF, with about 10K samples ([arXiv 2410.14748](https://huggingface.co/papers/2410.14748)).
- **JSON-schema constrained decoding:** the field has standardized on JSON Schema ([arXiv 2501.10868](https://huggingface.co/papers/2501.10868)). Schema-key wording alone "can substantially affect accuracy" ([arXiv 2604.14862](https://huggingface.co/papers/2604.14862)).

Building the `node_ids` enum dynamically from the retrieved subgraph guarantees that the model cites only real IDs. It does not guarantee the claims are faithful, so an ETF-style post-check is still required.

**For embeddings, licenses decide more than benchmark scores.** jina-code-embeddings-0.5b scores 78.41% across 25 code benchmarks at 494M parameters, but it is **CC-BY-NC-4.0** ([Jina blog](https://jina.ai/news/jina-code-embeddings-sota-code-retrieval-at-0-5b-and-1-5b)). Two permissive models lead instead:

- **CodeRankEmbed:** 137M parameters, MIT, **CoIR NDCG@10 60.1**, ahead of CodeSage-Large's 59.4 at 1.3B ([HF card](https://huggingface.co/nomic-ai/CodeRankEmbed)).
- **Qwen3-Embedding-0.6B:** Apache-2.0, 73.49% on Jina's suite, with an official GGUF ([HF](https://hf.co/Qwen/Qwen3-Embedding-0.6B)).

The coordinator's direct Hub fetch adds two items the subagent notes lacked. **google/embeddinggemma-2** is Apache-2.0 (created 2026-09-14; [arXiv 2610.37364](https://huggingface.co/papers/2610.37364)), unlike the gated Gemma-licensed embeddinggemma-300m, but its parameter count (about 270M per derivative cards) and candle support are unverified. **ibm-granite/granite-embedding-small-english-r2** is a 47.7M ModernBERT model under Apache-2.0 ([arXiv 2508.21085](https://arxiv.org/abs/2508.21085)), but its code-retrieval score is unverified.

**For narration, Qwen3-4B-Instruct-2507 is the strongest verified fit.** It is Apache-2.0, non-thinking, has 262K context and a 2.50 GB Q4_K_M, and scores IFEval 83.4 and BFCL-v3 61.9 ([HF card](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507)). Two 2026 Apache-2.0 models are reasonable to wait on rather than adopt. Qwen3.5-4B uses the `qwen3_5` architecture, which has no candle module. Gemma 4 E4B is about 8B parameters in total, with a roughly 5 GB unsloth Q4_K_M, though an official QAT q4_0 GGUF exists. For **role classification**, no 2025–2026 model or benchmark exists. The practical path is a Prototypical-Networks-style nearest-prototype classifier over embeddings ([arXiv 1703.05175](https://huggingface.co/papers/1703.05175)), fused with graph features. Its accuracy is unmeasured.

## Least privilege means a separate helper with a frozen command surface on every OS

**Linux.** On kernel 5.8 or later, tracing needs **CAP_BPF + CAP_PERFMON**, not CAP_SYS_ADMIN. Add CAP_SYS_RESOURCE only before 5.11. Debian/Ubuntu set `perf_event_paranoid=3`, which forces CAP_SYS_ADMIN unless the value is lowered to 2 ([Grafana Beyla](https://grafana.com/docs/beyla/latest/requirements-and-security/security.md)). BPF token (6.9+) is designed for user-namespace delegation by container managers, so it does not fit a desktop helper ([Canonical LXD](https://documentation.ubuntu.com/lxd/en/latest/_sources/explanation/bpf.md.txt)). Yama `ptrace_scope` 2–3 blocks ptrace-based fallbacks ([kernel.org](https://docs.kernel.org/_sources/admin-guide/LSM/Yama.rst.txt)). CAP_BPF plus CAP_PERFMON still permits kernel memory reads, so the helper should be treated as root-equivalent for confidentiality. Unverified items: the exact systemd hardening directives, SO_PEERPIDFD's kernel version (believed 6.5), and lockdown's `confidentiality` mode blocking `bpf_probe_read_kernel`.

**Windows.** Performance Log Users can create real-time sessions and enable providers, but they lack **TRACELOG_ACCESS_KERNEL_LOGGER**, so kernel events need an administrator or SYSTEM service ([Geoff Chappell](https://geoffchappell.com/notes/windows/etw/security.htm)). A non-admin SystemTraceProvider session is possible only after granting the profile privilege and calling EventAccessControl on both GUIDs ([Microsoft Learn](https://learn.microsoft.com/en-ca/Windows/win32/etw/configuring-and-starting-a-systemtraceprovider-session)). Named-pipe client PIDs are spoofable, so the helper should authorize by token and SID through impersonation ([Project Zero](https://projectzero.google/2019/09/windows-exploitation-tricks-spoofing.html)).

**macOS.** libproc works without privilege for the user's own processes. `task_for_pid` is effectively for debuggers only ([Apple DTS](https://developer.apple.com/forums/thread/734461)). Endpoint Security brings three costs at once: a restricted entitlement, root, and user-granted Full Disk Access ([Apple forums 791996](https://developer.apple.com/forums/thread/791996)). The helper path is an `SMAppService.daemon`, which needs approval in System Settings ([theevilbit](https://theevilbit.github.io/posts/smappservice)). It should authenticate the app with `xpc_connection_set_peer_code_signing_requirement` rather than PID-based `SecCodeCheckValidity`, which is exposed to TOCTOU ([Apple forums 773573](https://developer.apple.com/forums/thread/773573)).

**Inside the app**, Tauri v2 capabilities scope commands per window label. Custom commands are open to all windows unless restricted through `AppManifest::commands`, and capabilities do not protect against Rust-side bugs or lax scopes ([Tauri docs](https://v2.tauri.app/security/capabilities/)).

## Research Synthesis Table

| Paper/Source (arXiv id + URL) | Year/Venue | Key finding (with numbers) | Exact change for Architekton |
|---|---|---|---|
| cAST, [arXiv 2506.15655](https://arxiv.org/html/2506.15655v2) | 2025, EMNLP Findings | Split-then-merge over Tree-sitter with a 2000 non-whitespace-char budget. RepoEval R@5 +1.8/+4.3; SWE-bench Pass@1 +2.7; split-only hurts | Implement cAST in Rust over the existing CST. Chunk ID = blake3(file hash, AST node path, grammar version, budget). Default budget 2000 |
| LocAgent, [arXiv 2503.09089](https://arxiv.org/html/2503.09089) | 2025, ACL | 4-layer sparse index plus typed BFS. File Acc@5 94.16% (Lite); cost cut about 86% | "Jump to entity" path: SCIP FQN map, then short-name multimap, then BM25 over FQNs, then chunk-to-entity index. Expose `traverse(dir, hops≤2, kinds, rels)` |
| CGM, [arXiv 2505.16901](https://arxiv.org/html/2505.16901v2) | 2025, NeurIPS | 7 node and 5 edge types. Lite 43.00%; without reranker 18.33%; without Graph RAG 9.67% | Add TEXTFILE and ATTRIBUTE node kinds. Emit a deterministic signature-only "file skeleton" per file for L3 preview and reranking |
| RepoGraph, [arXiv 2410.14684](https://arxiv.org/html/2410.14684v2) | 2025, ICLR | Ego-graphs with k=1–2, filtering stdlib and third-party. +32.8% relative on Lite | Flag every edge `internal`/`external` from SCIP package info. Hide external edges by default |
| LARGER, [arXiv 2605.16352](https://arxiv.org/html/2605.16352) | 2026, preprint | Lexical hits snapped to nodes, then K-hop expansion. LocBench file Acc@5 +13.9 | Text-search hits resolve to their enclosing node and show its 1-hop neighborhood. Add heuristic test↔source and doc↔source edges |
| Small agents and code graphs, [arXiv 2610.33958](https://arxiv.org/html/2610.33958v1) | 2026, preprint | Each hop gives odds ×0.47–0.55. Graph tools +17–21 at 1.7–4B; embeddings -1.1 | Make the SCIP-exact 1-hop caller/callee panel the primary interaction. Ship embeddings as optional, not the default search |
| Code Isn't Memory, [arXiv 2606.22417](https://arxiv.org/pdf/2606.22417) | 2026, preprint | Vector + graph + BM25 index. Acc@5 44.3→84.5; resolve 41.9→50.4. Merkle incremental path not evaluated | Merkle tree over the working copy. On a `notify` event, re-hash the path to root and re-index only changed leaves. Add a `stale` overlay bit on dependents |
| Stack graphs, [arXiv 2211.01224](https://arxiv.org/abs/2211.01224v1) | 2022/23, EVCS (background) | Per-file self-contained fragments; unchanged files are never re-analyzed | Immutable per-file artifacts keyed by content hash plus indexer version. Re-run the cross-file join only when a file's import/export surface hash changes |
| Agent Retrieval Bench, [arXiv 2607.24882](https://arxiv.org/html/2607.24882v1) | 2026, preprint | RepoMap BCY@8k 0.3788 (best). RRF raises MRR 0.2296→0.2713 | "Export context": signatures ranked by SCIP in-degree to fit a token budget. RRF(lexical, graph, optional dense). Build an edit2ripple impact view from reverse refs |
| MULocBench, [arXiv 2509.25242](https://arxiv.org/html/2509.25242) | 2025, preprint | 23.5% of fixes in tests, 23.5% in docs, 15.2% in config. Best file Acc@5 35.2% | Index docs, configs, CI and assets as first-class L3 nodes linked to code |
| Hierarchical summarization, [arXiv 2501.07857](https://arxiv.org/html/2501.07857v1) | 2025, ICSE LLM4Code | Whole-file summarization missed 9–24% of entities. Segment→file→package is complete by construction | Optional summaries strictly bottom-up. Cache key = (child-summary hashes, model ID, prompt version) |
| LongCodeBench, [arXiv 2505.07897](https://arxiv.org/abs/2505.07897v2) | 2025, preprint | Claude 3.5 drops 29%→3% at long context | Never feed the SLM whole files or repos. Cap the serialized subgraph at about 4–8K tokens |
| LLMs vs static CG, [arXiv 2410.00603](https://arxiv.org/html/2410.00603v2) | 2025, journal (venue unstated) | PyCG soundness 87.3% vs 62.6% for the best of 24 LLMs. HeaderGen 91.8% vs 26.2% | No LLM in call-graph construction. Edge provenance enum: `scip-exact`, `ts-name-heuristic`, `dynamic-observed`, `ranked-candidate` |
| Is CG pruning effective?, no arXiv ([DOI](https://doi.org/10.1145/3744916.3773247)) | 2026, ICSE | 313 of 348 "false" edges were true. Validated precision 0.97; pruning reaches 0.96–0.98 | Do not ship a learned pruner. Spend the effort on recall (runtime hints, candidates) |
| Approx. interpretation (Jelly), no arXiv ([PDF](https://www.cs.au.dk/~amoeller/papers/approx/paper.pdf)) | 2024, PLDI (background) | Recall 75.9→88.1% at -1.5% precision. Pre-pass about 4.5 s | Optional JS/TS backend: invoke Jelly as a sidecar (license unverified). Treat telemetry caller→callee pairs as equivalent "hints" |
| GRAPHIA, [arXiv 2506.18191](https://arxiv.org/abs/2506.18191) | 2025, preprint | Unresolved JS sites: top-1 >42%, top-5 72% | Show a top-5 ranked candidate list for unresolved sites using deterministic features (arity, property name, import proximity). Exclude candidates from aggregates |
| Runtime entry points, no arXiv ([PDF](https://jacquesklein2302.github.io/papers/2025-FSE-IVR-call_graph.pdf)) | 2025, FSE IVR | 5-min run: nodes 50,626→65,534; taint flows 105→152 | Add sampled stack roots as reachability entry points. Draw sampled edges absent from the static graph as dashed "dynamic-only" edges |
| Guided Leiden, no arXiv ([PDF](https://eprints.cs.univie.ac.at/8461/1/paper_140.pdf)) | c.2025–26, venue unverified | Force nodes (domain, data, test, call weight). 2,048 solutions, 108 Pareto; NCR 0.56 | L2 subsystem clusters = seeded Leiden over call+import edges plus force nodes (directory, package, test co-coverage, telemetry co-occurrence). Expose wd/wp/wu/wc as sliders |
| ArchAgent, [arXiv 2601.13007](https://arxiv.org/pdf/2601.13007) | 2026, preprint | F1 0.966 vs 0.860 DeepWiki (8 repos, author-side raters) | Optional cluster naming only. Partition each cluster into ⌈T/M⌉ token-balanced groups with about 10% overlap |
| BigO(Bench), [arXiv 2503.15242](https://arxiv.org/html/2503.15242v2) | 2025, preprint | LLM time pass@1 29.2. NNLS labeler agrees with humans 92% (time), 84% (space) | Static Big-O rule engine (loop kinds, recurrences, library cost table) with a "why" trace. Optional "verify empirically" NNLS fit over {1, log n, n, n log n, n², n³, 2ⁿ} |
| CodeComplex, [arXiv 2401.08719](https://arxiv.org/abs/2401.08719) | 2025, EMNLP Findings | 4,900 Java and 4,900 Python worst-case-labeled programs | CI regression suite for the Big-O rule engine. Report a confusion matrix |
| OTel eBPF profiler, no arXiv ([GitHub](https://github.com/open-telemetry/opentelemetry-ebpf-profiler)) | 2025–26, OSS | `.eh_frame` unwinding, 8+ runtimes, <1% CPU / 250 MB limits, alpha Profiles signal | Copy its leaf-frame `.eh_frame` fallback. Map frames through a Tree-sitter interval tree to SCIP symbols. Use the 1% CPU budget as a daemon SLO |
| Frame-pointer limits, no arXiv ([Red Hat](https://developers.redhat.com/articles/2024/10/30/limitations-frame-pointer-unwinding)) | 2024 (background) | About 5.2% of x86 samples fall in prologues | Flag leaf PCs within 8 bytes of a symbol start. Prefer DWARF/SFrame unwinding for the leaf frame |
| RANGER, [arXiv 2509.25257](https://huggingface.co/papers/2509.25257) | 2025, preprint | Variable-level repo KG; Cypher for entity queries, MCTS for natural-language queries; beats Qwen3-8B-embedding baselines | Attach descriptions and embeddings to graph nodes. Route entity-shaped queries to exact lookup, never to the SLM |
| ETF, [arXiv 2410.14748](https://huggingface.co/papers/2410.14748) | 2024 (background) | Traces summary entities back to code; about 10K-sample dataset | After generation, verify that every cited node exists and every claimed edge exists in the graph. Drop or flag failures |
| JSONSchemaBench, [arXiv 2501.10868](https://huggingface.co/papers/2501.10868); Schema-key wording, [arXiv 2604.14862](https://huggingface.co/papers/2604.14862) | 2025; 2026 | Constrained decoding has standardized on JSON Schema. Key wording shifts accuracy in both directions | llama.cpp grammar with `evidence_node_ids: enum[<retrieved ids>]`. A/B test key names in the eval harness |
| CodeRankEmbed / CoRNStack, [arXiv 2412.01007](https://arxiv.org/abs/2412.01007) | 2024–25 | 137M, CoIR 60.1 vs 59.4 for CodeSage-L (1.3B) | Default embedder for function nodes and the role-prototype classifier |
| Prototypical Networks, [arXiv 1703.05175](https://huggingface.co/papers/1703.05175) | 2017 (background) | Classify by distance to class-mean embeddings | Role classifier = cosine to role prototypes (5–20 seeds per role plus NL descriptions), fused with fan-in/out, I/O calls and test path via logistic regression. SLM adjudicates only low-confidence cases |

## Verified dependency table (registries fetched 2026-11-21)

GitHub API access was blocked, so star counts, archive status and last-commit dates are **unverified**. The npm, crates.io, Go proxy, Maven and PyPI registries are treated as authoritative. Sources: [crates.io API](https://crates.io/api/v1/crates/tauri), [npm registry](https://registry.npmjs.org/@xyflow/react), [TS 7 blog](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/), [React Flow what's new](https://reactflow.dev/whats-new), [aya book](https://aya-rs.dev/book/start/development/), [elkjs](https://github.com/kieler/elkjs).

| Dependency | Version | License | Caveat / decision |
|---|---|---|---|
| tauri / @tauri-apps/cli | 2.12.7 / 2.12.6 (3.0.0-alpha.6 exists) | Apache-2.0 OR MIT | Stay on v2. MSRV 1.90. Custom commands are open to all windows unless restricted via `AppManifest::commands` |
| tree-sitter / web-tree-sitter | 0.27.2 | MIT | ABI 15, accepts grammars down to ABI 13. Pin it, because grammar version is part of the chunk-ID determinism |
| tree-sitter-{rust,python,javascript,go} | 0.24.2 / 0.25.0 / 0.25.0 / 0.25.0 | MIT | ABI 15 |
| tree-sitter-{typescript,java} | 0.23.2 / 0.23.5 | MIT | ABI 14 (works); about 2 years old |
| tree-sitter-language-pack | 1.25.2 | MIT | Depends on ureq and libloading, which suggests runtime download. **Avoid for offline use**; offline mode unverified |
| scip (crate) | 0.10.0 | Apache-2.0 | Repo moved to `scip-code/scip` (reason undocumented) |
| scip-typescript / scip-python | 0.6.2 / 0.6.8 | Apache-2.0 / MIT | Need Node (+40–100 MB sidecar, an estimate) |
| scip-go / scip-java | v0.2.10 / 0.12.3 | — (not fetched) | scip-go is a static binary. scip-java needs a system JDK plus the build tool, so make it optional |
| ast-grep-core | 0.51.3 | MIT | Shares tree-sitter ^0.27 |
| aya / aya-ebpf / aya-build | 0.16.0 / 0.3.0 / 0.2.1 | MIT OR Apache-2.0 | **MSRV 1.98** (sets the workspace floor). Nightly + rust-src + bpf-linker at build time. eBPF object likely needs a GPL-compatible `license` section (**unverified**) |
| ferrisetw | 1.2.0 (2024-06) | MIT OR Apache-2.0 | Stale. Wrap `windows` 0.62.2 `Win32_System_Diagnostics_Etw` as the fallback. Avoid krabsetw (C++, not on crates.io, license unverified) |
| sysinfo / libproc / mach2 | 0.40.4 / 0.15.0 / 0.7.0 | MIT / MIT / BSD-2 OR MIT OR Apache | Unprivileged tier |
| tokio / rayon / ignore | 1.54.1 / 1.12.0 / 0.4.34 | MIT / MIT OR Apache / Unlicense OR MIT | — |
| notify | 9.0.0 (released on fetch day) | CC0-1.0 | **Use 8.x** until 9 matures |
| blake3 / gix | 1.8.7 / 0.90.0 | CC0 OR Apache variants / MIT OR Apache | gix is still 0.x |
| redb / rusqlite | 5.0.0 (3 days old) / 0.43.0 | MIT OR Apache / MIT | Prefer rusqlite, or pin redb 5 after a soak. **Avoid sled** (stable line from 2021) |
| petgraph | 0.8.3 | MIT OR Apache-2.0 | Leiden implementation not in petgraph. Write a seeded one in-house (not verified elsewhere) |
| specta / tauri-specta | 2.0.0-rc.28 / rc.29 | MIT | RC only. Use ts-rs 12.0.1 for stable types |
| candle-core / -transformers | 0.11.1 | MIT OR Apache-2.0 | Has qwen3, quantized_qwen3, nomic_bert, modernbert, jina_bert, gemma3/4. **No qwen3_5, no gte** |
| llama-cpp-2 | 0.1.161 | MIT OR Apache-2.0 | Grammar/JSON-schema decoding path |
| tokenizers | 0.23.2 (1.0.0-rc.5) | Apache-2.0 | Carry the NOTICE file |
| rust-code-analysis | 0.0.25 (2023) | MPL-2.0 | **Do not use**: stale, likely old tree-sitter. Implement cyclomatic/cognitive complexity over tree-sitter queries |
| @xyflow/react | 13.0.2 | MIT | Breaking changes (`getNodeConnections`, `paneDragThreshold`, `HandleBounds`). Sub-flows no longer need parents ordered first. New `changeParent` |
| elkjs | 0.12.1 | **EPL-2.0 OR GPL-3.0+** | Elect EPL-2.0, ship unmodified, load `elk-api.js` plus a worker |
| roughjs | 4.6.6 (2023) | MIT | Stale but small |
| react | 19.3.1 | MIT | — |
| typescript | 7.0.2 (Go native) | Apache-2.0 | **No programmatic API**. Use Biome 2.6.8, or alias `@typescript/typescript6` for typescript-eslint |
| vite / vitest / eslint / tailwind | 8.3.8 / 5.1.1 / 10.15.0 / 4.3.5 | MIT | — |
| pnpm | 12.21.0 | MIT | Build scripts blocked by default. 1440-min `minimumReleaseAge` default |
| lizard (PyPI) | 1.26.1 | MIT | Reference implementation only; do not bundle Python |

## Model shortlist

The params and licenses below come from the coordinator's direct HF Hub fetch, reconciled with slm_models.md. Where the two disagree, the Hub data wins. The notes and the Hub agree on every license listed. Sources: [Qwen3-4B-Instruct-2507](https://hf.co/Qwen/Qwen3-4B-Instruct-2507), [CodeRankEmbed](https://huggingface.co/nomic-ai/CodeRankEmbed), [candle models](https://docs.rs/candle-transformers/latest/candle_transformers/models/).

| Repo id | Params | License | GGUF / candle support | Role |
|---|---|---|---|---|
| nomic-ai/CodeRankEmbed | 137M | MIT | Community GGUF (Q8_0, Q4_K_M) / candle `nomic_bert` (custom_code) | **Default embedder** and role-prototype backbone. Query prefix required |
| Qwen/Qwen3-Embedding-0.6B | 596M | Apache-2.0 | Official GGUF (Q8_0 0.64 GB) / candle `qwen3` | High-quality embedding tier |
| ibm-granite/granite-embedding-small-english-r2 | 47.7M | Apache-2.0 | GGUF unverified / candle `modernbert` | Ultra-light tier and fine-tune base for the role classifier. **Code score unverified** |
| google/embeddinggemma-2 | ~270M (unverified) | Apache-2.0 | unsloth GGUF, ONNX / candle **unverified** | Watch list. Multimodal, created 2026-09 |
| Qwen/Qwen3-4B-Instruct-2507 | 4.02B | Apache-2.0 | unsloth Q4_K_M 2.50 GB / candle `qwen3`, `quantized_qwen3` | **Default grounded narrator**. IFEval 83.4, 262K context |
| Qwen/Qwen3-1.7B | 2.03B | Apache-2.0 | Q4_K_M 1.11 GB / candle `qwen3` | Low-RAM narrator (≤8 GB machines) |
| microsoft/Phi-4-mini-instruct | 3.84B | MIT | Q4_K_M 2.49 GB / candle `phi3` | MIT-only alternate narrator |
| Qwen/Qwen2.5-Coder-1.5B-Instruct | 1.54B | Apache-2.0 | Official GGUF 1.12 GB / candle `qwen2` | Code-tuned low-RAM option and zero-shot role adjudicator |
| HuggingFaceTB/SmolLM3-3B | 3.08B | Apache-2.0 | ggml-org Q4_K_M 1.92 GB / candle coverage unverified | Fully open alternate |
| ibm-granite/granite-4.0-micro | 3.4B | Apache-2.0 | Official signed GGUF 2.10 GB / candle `granitemoehybrid` | Alternate. Hybrid vs dense status unverified |
| Qwen/Qwen3.5-4B / -2B | 4.66B / 2.27B | Apache-2.0 | unsloth GGUF / **no candle qwen3_5** | Watch list. Multimodal; would force llama.cpp |
| google/gemma-4-E4B-it / E2B-it | 8B / 5.1B total (~4B / ~2B effective) | Apache-2.0 | Official QAT q4_0 GGUF (E4B) / candle `gemma4` coverage of E-variants unverified | Watch list. About 2× Qwen3-4B memory |
| **Avoid:** jina-code-embeddings-0.5b/1.5b, jina-embeddings-v5-text-nano, SFR-Embedding-Code | 494M / 1.54B / — / 434M | **CC-BY-NC-4.0** | — | Non-commercial |
| **Avoid:** Qwen2.5-Coder-3B-Instruct; embeddinggemma-300m; gemma-3-4b-it; Llama-3.2-3B | 3.09B / 303M / 4.3B / 3.21B | Qwen Research (NC) / Gemma (gated) / Gemma / Llama 3.2 CL | — | License friction |
| microsoft/bitnet-embedding-270m / -0.6b | 270M / 0.6B | **Not shown** | GGUF | Unverified. Do not adopt until the license is confirmed |

## Exact changes for Architekton

The changes above add up to a dependency order, so Phase 2 can be sequenced. The **storage substrate** comes first. It holds immutable per-file artifacts, meaning symbols, local def/ref and cAST chunks, keyed by blake3 of the content plus grammar and indexer versions. A Merkle directory tree sits over them, and a `stale` overlay marks dependents awaiting the cross-file join. Re-run SCIP only when build-relevant files change, and let Tree-sitter carry interim per-file updates. That hybrid is an inference, not something a paper tested.

The **graph schema** gains:

- TEXTFILE, config, doc and ATTRIBUTE nodes
- test↔source and doc↔source edges
- an internal/external flag on edges
- a four-value provenance enum on every call edge

The **query layer** defaults to the LocAgent-style FQN cascade and SCIP-exact 1-hop panels. RRF fusion comes next, and embeddings come last and are optional. The **zoom hierarchy** runs as follows:

| Level | Contents |
|---|---|
| L1 | Build-manifest packages |
| L2 | Seeded Leiden with force nodes |
| L3 | Files and skeletons |
| L4 | Types |
| L5 | Functions, with Big-O badges and "why" traces |

The **telemetry daemon** ships as a separate signed artifact with three tiers:

| Tier | What it covers |
|---|---|
| Unprivileged | Own-user processes via sysinfo/libproc |
| Opt-in elevated (Linux) | systemd service with `AmbientCapabilities=CAP_BPF CAP_PERFMON`, embedded BPF objects only, SO_PEERCRED auth |
| Opt-in elevated (Windows) | Virtual-account service, named pipe with `PIPE_REJECT_REMOTE_CLIENTS` and a DACL of user SID plus SYSTEM, impersonation-based auth |
| Opt-in elevated (macOS) | `SMAppService.daemon` with a peer code-signing requirement pinning Team ID and bundle ID, plus launch constraints |

All elevated tiers share one rule set:

- a fixed command set
- argv and env off by default
- bounded ring buffers that drop rather than block
- scope set by a root-owned config, never by the client

The Tauri app itself never elevates. Telemetry commands are restricted to a single window label via `AppManifest::commands`.

## Conclusion

The research changes Architekton's risk profile more than its feature list. The parts that looked hardest, precise cross-language structure and runtime attribution, are where the evidence favors deterministic engineering Architekton already has: SCIP precision, Tree-sitter ranges and seeded clustering. The parts that look easiest, "add a small model", are where the evidence is thinnest. No paper validates grounded narration at 1.5–4B. Role classification has no benchmark. Embeddings did not help small agents in the one direct study. Phase 2 should therefore build an offline evaluation harness before tuning any model. It would replay LocBench, MULocBench and Agent Retrieval Bench queries for retrieval, CodeComplex for Big-O, and ETF-style entity tracing for narration faithfulness.

The second implication is that the privileged helper, not the model, is Architekton's largest security liability. CAP_BPF plus CAP_PERFMON is root-equivalent for confidentiality, PIDs are spoofable on Windows, and macOS helpers can keep running after a user disables them. The helper's command surface should be designed and reviewed as if it were a kernel API, because in effect it is one.
