# Repository-Scale Code Search, AST-Aware Chunking, and Hierarchical Index Management (2025–2026)

Scope note: notes target FolderKnower (desktop, offline, deterministic; Tree-sitter + SCIP; hierarchy System tiers -> subsystem clusters -> files -> functions). Every item below was fetched or seen in search results with the URL given. "Background" = pre-2025. Venue claims taken only from an ACL Anthology / proceedings URL that appeared in search results are marked "(venue per listing URL)".

## AST-aware chunking (cAST and successors): split/merge algorithm details, gains over line chunking

### Takeaway
cAST (2025) is the clearest reference algorithm: a recursive, top-down split-then-merge over the Tree-sitter AST, sized in non-whitespace characters (max ~2000), which beats fixed-size line chunking by +1.8 to +4.3 Recall@5 on RepoEval and +2.7 Pass@1 on SWE-bench. The algorithm is fully deterministic and needs no model, so it fits FolderKnower directly.

### Cited Findings
- **cAST: Enhancing Code Retrieval-Augmented Generation with Structural Chunking via Abstract Syntax Tree.** Yilin Zhang, Xinran Zhao, Zora Zhiruo Wang, Chenyang Yang, Jiayi Wei (Augment Code), Tongshuang Wu (CMU). arXiv:2506.15655 (v2 Oct 2025); EMNLP 2025 Findings (venue per listing URL aclanthology.org/2025.findings-emnlp.430). — [arXiv](https://arxiv.org/html/2506.15655v2); [ACL Anthology](https://aclanthology.org/2025.findings-emnlp.430)
  - Algorithm: parse with tree-sitter. If the whole file fits the budget, emit one chunk. Otherwise descend to the children. Greedily merge adjacent sibling nodes into a chunk while it stays within the limit. Any single node over the limit starts a new chunk and is split recursively into its own children. — [arXiv](https://arxiv.org/html/2506.15655v2)
  - Size metric: non-whitespace characters, not lines, so chunk size is comparable across languages and formatting styles. Max chunk = 2000 chars in experiments; the sensitivity analysis peaked at 2000–2500. — [arXiv](https://arxiv.org/html/2506.15655v2)
  - Gains vs fixed-size chunking: RepoEval Recall@5 +2.4 / +4.3 / +1.8 (BGE / GIST / CodeSage); Precision@5 +2.1 / +3.3 / +1.2; RepoEval Pass@1 (StarCoder2) average +5.5; SWE-bench Pass@1 +2.7 (Claude 3.7 Sonnet, CodeSage retriever); SWE-bench Pass@8 +3.0 (Gemini 2.5 Pro, BGE); CrossCodeEval averaged over Python/Java/C#/TS: code EM +2.9, identifier EM +3.0. — [arXiv](https://arxiv.org/html/2506.15655v2)
  - Ablation: split-only (no merge) hurt both retrieval and generation for every retriever, so the merge of small siblings matters. — [arXiv](https://arxiv.org/html/2506.15655v2)
  - **FolderKnower change:** implement cAST split-then-merge over the existing Tree-sitter CST, with a non-whitespace-char budget (default 2000), and key each chunk by (file content hash, AST node path) so chunk IDs stay deterministic.
- **Code Isn't Memory: A Structural Codebase Index Inside a Coding Agent** (2026) also builds chunks from Tree-sitter ASTs alongside definitions, references and call edges. It does not publish a chunking algorithm beyond that. — [arXiv 2606.22417](https://arxiv.org/pdf/2606.22417)
- **Hierarchical Repository-Level Code Summarization for Business Applications Using Local LLMs** (Dhulshette, Shah, Kulkarni; TCS Research; arXiv:2501.07857; ICSE 2025 LLM4Code workshop per [researchr listing](https://conf.researchr.org/details/icse-2025/llm4code-2025-papers/29/Hierarchical-Repository-Level-Code-Summarization-for-Business-Applications-Using-Loca)). It uses AST segmentation (functions, variables, constructors, enums, interfaces) as its units, a form of syntactic chunking for summarization rather than retrieval. Interfaces are kept whole at file level. — [arXiv](https://arxiv.org/html/2501.07857v1)

### Inferences
- cAST is deterministic given a fixed grammar version and budget. Pinning the Tree-sitter grammar versions in FolderKnower's build is what keeps chunk boundaries reproducible across machines.
- Because merging stops at sibling boundaries, chunks nest inside the function/class hierarchy FolderKnower already has. Each chunk can therefore hang under exactly one graph node, and no chunk spans two functions unless both are small siblings.

### Gaps
- I found no 2025–2026 successor paper that clearly improves on cAST's algorithm, for example with overlap, signature or context headers, or learned boundaries. The searches surfaced only cAST itself. A targeted search, such as "AST chunking 2026", could close this.
- cAST reports retrieval gains with embedding retrievers only. Its effect on BM25 or on symbol-based lookup is not reported.

## Repository-level graph indexes / code knowledge graphs: node/edge schemas, hierarchical summaries

### Takeaway
The 2025–2026 consensus schema is a typed heterogeneous graph: directory/package, file, class, function (sometimes attribute), with edges contain, import, invoke/call, inherit/extends (sometimes implements, source-to-test, source-to-doc). It is paired with a layered lexical index on fully qualified names and k-hop (usually 1–2) neighborhood expansion. The 2026 evidence says graph tools help most at 0–1 hops, and that lexical anchors into the graph plus fusion with dense retrieval beat any single retriever.

### Cited Findings
- **LocAgent: Graph-Guided LLM Agents for Code Localization.** Zhaoling Chen, Xiangru Tang, Gangda Deng, Fang Wu, Jialong Wu, Zhiwei Jiang, Viktor Prasanna, Arman Cohan, Xingyao Wang (Yale, USC, Stanford, All Hands AI). arXiv:2503.09089; ACL 2025 long (venue per listing URL 2025.acl-long.426). — [arXiv](https://arxiv.org/html/2503.09089); [ACL listing](https://preview.aclanthology.org/iwcs-25-ingestion/2025.acl-long.426.pdf)
  - Nodes: directory, file, class, function. Edges: contain, import, invoke, inherit. Python only. — [arXiv](https://arxiv.org/html/2503.09089)
  - Hierarchical sparse index, queried top-down: (1) an entity-ID index on fully qualified names such as `src/utils.py:MathUtils.calculate_sum`; (2) a dictionary from short name to all matching nodes; (3) BM25 over entity IDs for fuzzy matches; (4) an inverted index from code chunks to entities, for things like globals. — [arXiv](https://arxiv.org/html/2503.09089)
  - Tools: SearchEntity (returns fold, preview or full snippets), TraverseGraph (type-aware BFS with direction, hops, entity-type and relation-type filters), RetrieveEntity. — [arXiv](https://arxiv.org/html/2503.09089)
  - Results: SWE-Bench-Lite file Acc@5 is 94.16% with Claude-3.5 and 92.70% with a fine-tuned Qwen2.5-32B; cost falls from about $0.66 to about $0.09 per example (about 86%). — [arXiv](https://arxiv.org/html/2503.09089)
  - **FolderKnower change:** add the four-layer lookup (FQN map from SCIP symbols -> short-name multimap -> BM25 over FQNs -> chunk-to-entity inverted index) as the default "jump to entity" path, and expose a typed, hop-limited BFS query.
- **Code Graph Model (CGM): A Graph-Integrated LLM for Repository-Level SE Tasks.** Hongyuan Tao, Ying Zhang, Zhenhao Tang, et al. (Ant Group, ShanghaiTech, ZJU, SJTU). arXiv:2505.16901; NeurIPS 2025 (venue per [proceedings listing](https://proceedings.neurips.cc/paper_files/paper/2025/hash/178ae4ba29022eb7bf509c2e27bc8ab8-Abstract-Conference.html)). Code: CodeFuse-CGM. — [arXiv](https://arxiv.org/html/2505.16901v2)
  - 7 node types: REPO, PACKAGE, FILE, TEXTFILE, CLASS, FUNCTION, ATTRIBUTE. 5 edge types: contains, calls, extends, imports, implements (Java only). — [arXiv](https://arxiv.org/html/2505.16901v2)
  - Agentless Graph RAG pipeline: a Rewriter (extracts entities and keywords), a Retriever (anchor nodes by string and semantic match, 1-hop expansion, connected to the root), a Reranker (10 files by query and file name, then 5 by file-skeleton relevance), and a Reader (the graph-aware LLM). — [arXiv](https://arxiv.org/html/2505.16901v2)
  - Results: SWE-bench Lite 43.00% (Qwen2.5-72B, first among open-weight models); SWE-bench Verified 50.40%; SWE-bench-java Verified 14.29%. Ablation: dropping the Reranker gives 18.33%, and dropping the whole Graph RAG front end gives 9.67%. — [arXiv](https://arxiv.org/html/2505.16901v2)
  - **FolderKnower change:** add TEXTFILE and ATTRIBUTE node kinds (for configs, docs and fields), and emit a deterministic "file skeleton" (signatures only) per file node for cheap reranking and preview.
- **RepoGraph: Enhancing AI Software Engineering with Repository-level Code Graph.** Siru Ouyang, Wenhao Yu, Kaixin Ma, et al. (UIUC, Tencent AI Seattle, Rice, Notre Dame). arXiv:2410.14684 (v1 Oct 2024, v2 Mar 2025); ICLR 2025 (venue per [proceedings listing](https://proceedings.iclr.cc/paper_files/paper/2025/hash/4a4a3c197deac042461c677219efd36c-Abstract-Conference.html)). — [arXiv](https://arxiv.org/html/2410.14684v2)
  - Line-level graph: tree-sitter parse; lines tagged def or ref; builtins, stdlib and third-party relations filtered out; edges are invoke (def->ref) and contain (def->internals); retrieval returns k-hop ego-graphs (k=1, 2) around a search term. — [arXiv](https://arxiv.org/html/2410.14684v2)
  - SWE-bench Lite: average 32.8% relative improvement; Agentless + GPT-4o went from 27.33% to 29.67%; RAG baseline from 2.67% to 5.33%. — [arXiv](https://arxiv.org/html/2410.14684v2)
  - **FolderKnower change:** tag edges as project-internal or external (stdlib, third-party), using SCIP package info, and hide external edges by default in ego-graph views.
- **CodeRAG: Supportive Code Retrieval on Bigraph for Real-World Code Generation** (v2 retitled **GraphCodeAgent: Dual Graph-Guided LLM Agent for Retrieval-Augmented Repo-Level Code Generation**). Jia Li, Xianjie Shi, Kechi Zhang, Lei Li, Ge Li, et al. (PKU, HKU, Beihang). arXiv:2504.10046 (Apr 2025). — [arXiv v1](https://arxiv.org/html/2504.10046v1); [v2](https://arxiv.org/html/2504.10046v2)
  - DS-code graph: nodes Module, Class, Method, Function; edges Import, Contain, Inherit, Call, Similarity (embedding cosine). A requirement graph is built from docstrings or LLM descriptions. Stored in Neo4j, with nodes pointing to source rather than copying it. — [arXiv v1](https://arxiv.org/html/2504.10046v1)
  - DevEval (1,825 samples, 117 repos): GPT-4o Pass@1 goes from 17.24 (no RAG) to 58.14, against 40.43 for embedding RAG. The paper has internal number inconsistencies: the abstract gives a 37.79 gain for Gemini-Pro, while the table gives 39.79. — [arXiv v1](https://arxiv.org/html/2504.10046v1)
  - **FolderKnower change:** store source as (file hash, byte range) references on nodes rather than copying text into the graph DB (the same storage idea).
- **LARGER: Lexically Anchored Repository Graph Exploration and Retrieval.** Yuntong Hu, Tongli Su, Liang Zhao (Emory), Bowen Zhu, Hasibul Haque. arXiv:2605.16352 (May 2026, preprint). — [arXiv](https://arxiv.org/html/2605.16352)
  - Graph: directories, files, classes, functions; edges for containment, imports, calls, plus cross-artifact source-to-test and source-to-doc. Lexical hits from the agent's normal search are aligned to graph nodes (an active set), then expanded by confidence-filtered K-hop neighborhoods, with per-anchor additive utility. Commit-aware alignment keeps the graph current across snapshots. No separate graph DB. — [arXiv](https://arxiv.org/html/2605.16352)
  - Results: LocBench file Acc@5 +13.9 (tuned) or +11.8 (fixed hyperparameters) over the strongest baseline; up to +12.2 Recall@5; the paper also reports beating Codex on SWE-Atlas Codebase QA and Test Writing. — [arXiv](https://arxiv.org/html/2605.16352)
  - **FolderKnower change:** add test<->source and doc<->source edges (by deterministic heuristics: path or name conventions and imports in test files), and make grep/text hits "snap" to their enclosing graph node and then show its 1-hop neighborhood.
- **How Do Small Coding Agents Use Code Graphs and Embeddings?** Saurav Bellam (UT Austin). arXiv:2610.33958 (25 Oct 2026). — [arXiv](https://arxiv.org/html/2610.33958v1)
  - Graph: files, classes, functions; edges contain, import, invoke, inherit; name-based call resolution, dropping names with more than 5 candidate targets. — [arXiv](https://arxiv.org/html/2610.33958v1)
  - Each extra hop of distance from the issue to the gold function multiplies the odds of correct localization by about 0.47–0.55. Graph tools (find_entity plus a 1–2-hop neighbors query) added +8.1 points for Qwen3-4B, while embeddings added -1.1. The graph gain is concentrated at hops 0–1. The structured-tool gain peaks at 1.7–4B models (+17–21) and shrinks at 32B (+4–5). Over 79k episodes, 500 tasks. — [arXiv](https://arxiv.org/html/2610.33958v1)
  - **FolderKnower change:** prioritize precise, SCIP-resolved 1-hop caller/callee/containment views over deep traversal UIs. The precision of name resolution matters more than depth (this study's heuristic resolution had to discard ambiguous names, which SCIP avoids).
- **SweRank: Software Issue Localization with Code Ranking.** Revanth Gangi Reddy et al. (UIUC, Salesforce, KAIST). arXiv:2505.07849; ICLR 2026 (as stated in the PDF). — [arXiv PDF](https://arxiv.org/pdf/2505.07849)
  - Bi-encoder retriever (137M or 7B) plus listwise LLM reranker over function-level units. SWE-Bench-Lite function Acc@10 is 88.69%, against 77.37% for LocAgent+Claude-3.5. LocBench function Acc@10 is 71.25%, against 59.29%. Cost is about $0.011–0.015 per example. — [arXiv PDF](https://arxiv.org/pdf/2505.07849)
  - **FolderKnower change:** the 137M SweRankEmbed-small class of model is small enough to ship as an optional offline local embedder over function nodes. Inference with fixed weights on CPU is deterministic in practice, but float nondeterminism across hardware should be tested (uncertain).

### Inferences
- FolderKnower's file -> function layers plus SCIP's precise def/ref already exceed what most of these papers build, since they mostly rely on Python `ast` and name-based call resolution. The missing pieces relative to the 2025–2026 schemas are TEXTFILE/config nodes, attribute/field nodes, test<->source and doc<->source edges, and an internal/external edge flag.
- None of the papers builds FolderKnower's upper tiers (System tiers -> subsystem clusters). Their top level is the directory/package hierarchy. Deterministic clustering above files is therefore not covered by this literature and is a differentiator.

### Gaps
- RepoHyper, GraphCoder and CodexGraph (2024, background) were not fetched in this pass, so no verified details are given.
- I found no 2025–2026 Kotlin-specific repository-graph paper.
- Agent4cs (alphaxiv 2607.01425, multi-agent summarization for large hierarchical codebases, 2026) appeared in search results only and was not fetched. — [alphaxiv](https://www.alphaxiv.org/abs/2607.01425)

## Incremental indexing of large monorepos: content-addressed/Merkle trees, incremental SCIP, stack-graphs, Glean, file-watcher strategies

### Takeaway
Academic coverage is thin. The only 2026 paper found (SuperAGI's "Code Isn't Memory") uses Merkle-tree diffs over the working copy to re-index only affected chunks, but did not evaluate that path or specify hashing and watching. The principled foundation for incremental name resolution is still stack graphs (background, 2022/2023): per-file, self-contained graph fragments that never need re-analysis of unchanged files.

### Cited Findings
- **Code Isn't Memory** (Ishaan Bhola, Adithyan Krishnan, Sravanth Kurmala, Mukunda NS; SuperAGI Research; arXiv:2606.22417, 21 Jun 2026, preprint). — [arXiv](https://arxiv.org/pdf/2606.22417)
  - Three-part index: vector (chunk embeddings), graph (definitions plus call edges), and BM25 over identifiers. Hybrid `codebase_search` (fuse, rerank, dedup) plus a `codebase_graph` callers/callees tool. A local overlay drops deleted paths and flags stale ones. — [arXiv](https://arxiv.org/pdf/2606.22417)
  - Incremental: built once per repo, then updated by Merkle-tree diffs over the working copy, so only affected chunks are re-indexed. The paper says this path was not exercised in the study and does not name a hash function or watcher. — [arXiv](https://arxiv.org/pdf/2606.22417)
  - Results (91 instances in Go, Java and Python; Claude Opus 4.7; 3 seeds): with the index on vs off, localization acc@5 is 84.5 vs 44.3 (+39.6 pp, p<0.0001); resolve rate 50.4 vs 41.9 (+7.9 pp, p=0.003); turns 28.3 vs 36.2; $/solved $2.30 vs $2.84. Per-cell cost difference is not significant. — [arXiv](https://arxiv.org/pdf/2606.22417)
  - **FolderKnower change:** maintain a Merkle tree (dir node hash = hash of sorted child (name, hash) pairs; file leaf = content hash). On a watcher event, recompute the path to the root, diff subtrees, and re-run Tree-sitter, chunking and SCIP-derived edges only for changed leaves. Add a "stale" overlay flag for nodes whose dependencies changed but have not yet been re-indexed.
- **Stack graphs: Name resolution at scale** (background). Douglas A. Creager, Hendrik van Antwerpen (GitHub). arXiv:2211.01224 (Nov 2022); DOI 10.4230/OASIcs.EVCS.2023.8 (EVCS 2023). — [arXiv](https://arxiv.org/abs/2211.01224v1); [Dagstuhl](https://drops.dagstuhl.de/entities/document/10.4230/OASIcs.EVCS.2023.8)
  - Each file produces a self-contained graph fragment built without seeing other files, so already-processed file versions never need re-analysis. Since most commits touch few files, storage and compute for keeping indexes current drop sharply. — [arXiv](https://arxiv.org/abs/2211.01224v1)
  - Rust crate: [stack-graphs on docs.rs](https://docs.rs/stack-graphs/^0.10).
  - **FolderKnower change:** keep per-file artifacts (symbols, local def/ref, chunk list) keyed by content hash and immutable. Do cross-file linking as a separate, cheap join pass that is redone only for files whose import or export surface changed.
- **LARGER** (2026) mentions "commit-aware alignment" to keep its graph current across repository snapshots, without algorithmic detail in what I read. — [arXiv](https://arxiv.org/html/2605.16352)

### Inferences
- A content-addressed design (per-file artifacts keyed by blob hash, matching git's blob SHA when the tree is clean) gives determinism for free: the same file bytes and the same grammar/indexer version produce the same artifact. This suggests including the indexer and grammar versions in the cache key.
- SCIP indexers are typically whole-project tools. Treating SCIP output as a per-document artifact (SCIP is organized by Document) and re-running SCIP only when a project's build-relevant files change, while using Tree-sitter for fast per-file updates in between, is a plausible hybrid. This is my inference, not from a cited paper.

### Gaps
- I found no 2025–2026 peer-reviewed paper on incremental SCIP, Glean, or file-watcher strategies for code indexes. These are documented mostly in engineering docs and blogs, which this pass did not verify.
- A JetBrains blog post (Nov 2026) on a semantic code search RAG pipeline appeared in search but was not fetched. — [JetBrains blog](https://blog.jetbrains.com/ai/2026/11/building-a-rag-pipeline-for-semantic-code-search-a-developer-diary-and-field-notes-part-2/)

## Hierarchical repository summarization / compression (repo maps, hierarchical code summarization 2025)

### Takeaway
2025 work builds summaries bottom-up (segment -> file -> package -> repo), with higher levels consuming only the summaries of the level below. Separately, a vectorless "RepoMap" signal (path, symbol and reference graph) wins on token-budgeted context yield in a 2026 benchmark. Deterministic structural compression (skeletons, repo maps) is therefore competitive without LLMs.

### Cited Findings
- **Hierarchical Repository-Level Code Summarization for Business Applications Using Local LLMs** (Dhulshette, Shah, Kulkarni; arXiv:2501.07857; ICSE 2025 LLM4Code per researchr listing). — [arXiv](https://arxiv.org/html/2501.07857v1); [researchr](https://conf.researchr.org/details/icse-2025/llm4code-2025-papers/29/Hierarchical-Repository-Level-Code-Summarization-for-Business-Applications-Using-Loca)
  - Pipeline: AST segments are summarized with local LLMs (Llama-3 8B, Starchat2 15B, Codestral 22B); segment summaries are ordered by position into file summaries; package summaries are built from file summaries only; then repo summaries. — [arXiv](https://arxiv.org/html/2501.07857v1)
  - Motivation result: direct whole-file summarization missed about 11% of functions and 24% of variables (Llama-3.2), or about 9% and 11% (GPT-4). The segment-first approach covers all segments by construction. Domain grounding raised domain-specificity from 0.568 to 0.637. Best function summaries: Codestral ROUGE-L 0.577, BERTScore 0.81. — [arXiv](https://arxiv.org/html/2501.07857v1)
  - **FolderKnower change:** if optional local-LLM summaries are added, generate them strictly bottom-up along the existing hierarchy, cache each by (child-summary hashes plus model ID plus prompt version), and regenerate only when a child hash changes (Merkle-style summary invalidation).
- **Agent Retrieval Bench: Evaluating Repository Context Retrieval for Coding Agents.** Bowen Qin (NUS), Yi Xie (PKU). arXiv:2607.24882 (2026; the exact date is not in the text, and references suggest about May 2026). — [arXiv](https://arxiv.org/html/2607.24882v1)
  - RepoMap (a vectorless path, symbol, reference and graph signal) has the best BCY@8k (0.3788, gold context per token budget) and leads trace2code (Recall@20 0.8366). Reciprocal-rank fusion of Qwen3-8B embeddings and RepoMap raises MRR from 0.2296 to 0.2713 and Recall@20 from 0.7070 to 0.7331. — [arXiv](https://arxiv.org/html/2607.24882v1)
  - **FolderKnower change:** provide a deterministic "repo map" export (per-file signature skeleton ranked by SCIP reference in-degree, fit to a token budget), and use RRF to combine lexical, graph and optional embedding rankings.
- Other 2025 summarization papers seen in search but not fetched: **RepoSummary: Feature-Oriented Summarization and Documentation Generation for Code Repositories** (arXiv:2510.11039) — [arXiv](https://arxiv.org/html/2510.11039v1); **Commenting Higher-level Code Unit: Full Code, Reduced Code, or Hierarchical Code Summarization** (arXiv:2503.10737) — [arXiv](https://arxiv.org/html/2503.10737v1).

### Inferences
- The 2501.07857 result (whole-file summarization drops 9–24% of entities) argues for FolderKnower's per-node summaries over whole-file prompts. The structural skeleton can be computed deterministically, and any LLM text added as an optional layer on top.

### Gaps
- Quantitative results for RepoSummary and 2503.10737 were not verified (not fetched).
- I found no 2025–2026 paper that evaluates Aider-style repo maps (PageRank over a symbol graph) directly, beyond Agent Retrieval Bench's RepoMap baseline.

## Benchmarks measuring localization / retrieval (SWE-bench variants, LongCodeBench, CoIR, RepoQA, LocBench)

### Takeaway
For evaluating FolderKnower's retrieval: LocBench (560 post-Oct-2024 issues) and MULocBench (1,100 issues including non-code locations) measure file, class and function Acc@k. Agent Retrieval Bench (2026) adds token-budget yield (BCY@B) and abstention. SWE-bench Lite and Verified are saturating for file-level localization (above 90% Acc@5), while MULocBench shows below 40% when docs, config and tests count.

### Cited Findings
- **Loc-Bench / LocBench** (from the LocAgent paper, 2025): 560 samples (242 bug, 150 feature, 139 performance, 29 security), issues after Oct 2024, at most 5 files and 10 functions changed; Python. — [arXiv](https://arxiv.org/html/2503.09089)
- **MULocBench: A Benchmark for Localizing Code and Non-Code Issues in Software Projects.** Zejun Zhang, Jian Wang, Qingyun Yang, et al. arXiv:2509.25242 (2025). — [arXiv](https://arxiv.org/html/2509.25242)
  - 1,100 issues from 46 Python projects. Locations: code 80.8%, test 23.5%, docs 23.5%, config 15.2%, asset 4.4%. Metrics: Acc@1/5 at file, class and function level, plus P/R/F1. — [arXiv](https://arxiv.org/html/2509.25242)
  - Best file Acc@5 is 35.2% (LocAgent+Claude 3.5); best function Acc@5 is 22.0%. "Even at the file level... below 40%". — [arXiv](https://arxiv.org/html/2509.25242)
  - **FolderKnower change:** index non-code files (docs, configs, CI, assets) as first-class nodes linked to code, because about 20–24% of real fixes touch them.
- **Agent Retrieval Bench** (2026): 427 samples, 25 repos; tasks code2test, comment2context, trace2code, edit2ripple, plus no-gold controls; metrics Recall@5/10/20, MRR, BCY@B, PES. Agents miss gold entirely on 27–35% of samples; no ranker improves selective success on natural no-gold cases. — [arXiv](https://arxiv.org/html/2607.24882v1)
  - **FolderKnower change:** use edit2ripple-style "what else must change" queries (reverse dependencies via SCIP refs) as a built-in impact-analysis view, and evaluate it on this benchmark.
- **SWE-bench Lite / Verified localization numbers:** LocAgent 94.16% file Acc@5 (Lite) — [arXiv](https://arxiv.org/html/2503.09089); SweRank 88.69% function Acc@10 (Lite), 71.25% (LocBench) — [arXiv](https://arxiv.org/pdf/2505.07849); small-agent depth study on SWE-bench Verified (500 tasks) — [arXiv](https://arxiv.org/html/2610.33958v1).
- **LongCodeBench: Evaluating Coding LLMs at 1M Context Windows.** Stefano Rando, Luca Romani, Alessio Sampieri, Luca Franco, John Yang, Yuta Kyuragi, Fabio Galasso, Tatsunori Hashimoto. arXiv:2505.07897 (May 2025). Tasks LongCodeQA and LongSWE-Bench. Claude 3.5 Sonnet drops from 29% to 3% and Qwen2.5 from 70.2% to 40% as context grows. — [arXiv](https://arxiv.org/abs/2505.07897v2)
  - **FolderKnower change:** this supports precise structural retrieval over dumping the whole repo into long context, so the tool should export tight, budgeted subgraph context.
- **CoIR: A Comprehensive Benchmark for Code Information Retrieval Models.** arXiv:2407.02883 (2024); ACL 2025 long (venue per listing URL aclanthology.org/2025.acl-long.1072). A code-retrieval model benchmark; details were not fetched in this pass. — [arXiv](https://arxiv.org/html/2407.02883v3); [ACL Anthology](https://aclanthology.org/2025.acl-long.1072)
- **SWE Context Bench** (arXiv:2602.08316, 2026) and the **SWE-Bench leaderboard dissection** (arXiv:2506.17208) appeared in search results but were not fetched. — [SWE Context Bench](https://arxiv.org/pdf/2602.08316); [Dissecting SWE-Bench](https://arxiv.org/html/2506.17208v2)

### Inferences
- An offline, deterministic evaluation harness for FolderKnower could replay LocBench, MULocBench and Agent Retrieval Bench queries against its lexical+graph index and report file/function Acc@5 and BCY@8k, without an LLM in the loop.

### Gaps
- RepoQA (2024, background) was not verified in this pass.
- I did not verify sizes or metrics for CoIR, SWE Context Bench, SWE-bench Pro, or Multi-SWE-bench.
- Exact LongCodeBench context lengths and instance counts were not in the fetched abstract.
