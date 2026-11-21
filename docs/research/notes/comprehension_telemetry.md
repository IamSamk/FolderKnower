# Deterministic Program Comprehension, Call-Graph Construction/Compression, Static Complexity Estimation, and Static+Dynamic Telemetry Fusion (2025–2026)

Scope note: these notes target "Architekton", an offline desktop tool. It (a) builds call graphs from Tree-sitter/SCIP, (b) estimates Big-O and cyclomatic/cognitive complexity from ASTs, (c) collapses large call graphs into a 5-level semantic zoom, and (d) maps OS telemetry (per-thread CPU, eBPF/ETW syscalls, stack samples) back to source functions. Items are 2025–2026 unless marked [BACKGROUND]. Items marked [TITLE-ONLY] were found in search results, but their contents could not be fetched or verified.

## Q1. Call-graph construction accuracy for JS/TS/Python/Rust/Go (2025–2026), LLM-assisted construction and pruning, benchmarks

### Takeaway
Static tools still beat LLMs at call-graph construction. On Python, PyCG and HeaderGen lead every one of 24 LLMs by a wide margin, and on JS, Jelly leads on soundness. The best 2025–2026 gains come from hybrids: a short dynamic or approximate-execution pass that feeds hints into a deterministic static analysis (Jelly approximate interpretation: recall 75.9% to 88.1%), and learned ranking for *unresolved* call sites (GNN top-5 hit rate of 72%). ML/LLM *pruning* of over-approximate edges is much less effective than earlier papers claimed (ICSE 2026). For Architekton, the priority order is: deterministic resolution first, cheap runtime hints second, ranked candidates (never silent edges) for leftovers.

### Cited Findings
- **"An Empirical Study of Large Language Models for Type and Call Graph Analysis in Python and JavaScript"**, Venkatesh, Sunil, Sabu, Mir, Reis, Bodden (Paderborn). arXiv 2410.00603 (v2 c. 2025; journal version per the ResearchGate "published" listing; venue not stated in the fetched text). The study evaluated 24 LLMs on the SWARM-CG/SWARM-JS benchmark suite. **Python PyCG benchmark:** PyCG reached 84.9% completeness, 87.3% soundness and 569/599 exact matches, against 60.3%, 62.6% and 518 for the best LLM (mistral-large). **HeaderGen (flow-sensitive) benchmark:** HeaderGen reached 90.9% completeness and 91.8% soundness, against 31.1% and 26.2% for the best LLM. **SWARM-JS (126 snippets):** Jelly reached 38.8% completeness, 67.4% soundness and 82.2% exact match, against 40.4%, 42.8% and 76.8% for mistral-large. TAJS failed on 102 of 126 cases because it does not support modern ECMAScript. LLMs did better on *type inference*: gpt-4o got 806 of 860 TypeEvalPy annotations right. — [arXiv 2410.00603v2](https://arxiv.org/html/2410.00603v2); [Bodden PDF](https://bodden.de/pubs/pss+2025empirical.pdf)
- **"Call Me Maybe: Enhancing JavaScript Call Graph Construction using Graph Neural Networks" (GRAPHIA)**, arXiv 2506.18191, June 2025 (venue not stated). The paper treats call-graph construction as link prediction on a program graph built from syntactic plus semantic edges (6.6M structural and 386K semantic edges). It trains on imperfect labels: static edges from existing tools plus dynamic edges from tests. The benchmark is 50 JS libraries with 163K call edges (150K static, 13K dynamic). For *unresolved* call sites, the correct target ranks first in more than 42% of cases and within the top 5 in 72%. — [arXiv 2506.18191](https://arxiv.org/abs/2506.18191)
- **"Is Call Graph Pruning Really Effective?"**, ICSE 2026 (Rio de Janeiro), DOI 10.1145/3744916.3773247. Prior ML pruning work treated dynamic call-graph edges as ground truth. When the authors hand-checked 348 NJR1 edges that had been "assumed false", 313 were actually true. With validated labels, baseline precision is already 0.97, and pruning moves it only to 0.96–0.98. Under the old assumed labels, pruning raised precision from 0.54 to 0.64–0.67 but cut coverage to 0.61–0.63. The best precision gain was about 9 points (fine-tuned codeBERT), at a cost of about 12 points of coverage. Random Forest precision fell from 82% to 60% on the manually labeled holdout set. The paper releases a new dataset of 41,952 labeled edges from four Java programs. A PU-learning variant reached 92% precision at 63% coverage. — [Preprint PDF](https://mohpydev.github.io/papers/call-graph-pruning-preprint.pdf); [DOI](https://doi.org/10.1145/3744916.3773247); [Zenodo artifact](https://zenodo.org/records/18217482)
- [BACKGROUND, PLDI 2024] **"Reducing Static Analysis Unsoundness with Approximate Interpretation"** (Møller et al., Aarhus; implemented in Jelly). A pre-analysis force-executes modules and functions, using proxy objects for unknown values. It records *read hints* and *write hints*, which map dynamic property accesses to allocation sites. Two extra constraint rules then consume these hints in a context-insensitive, flow-insensitive points-to analysis. **Results:** across 36 projects, call-edge recall rose from 75.9% to 88.1% while precision dropped by about 1.5%; senchalabs/connect went from 40.1% to 98.0%. Across 141 programs, call edges grew 55.1% and resolved call sites 17.7%. **Cost:** the pre-analysis takes 0.6–51 s per program (average 4.5 s, about 3.2 ms per function), and static analysis time rose less than 10% for 76 of 141 programs. Twenty programs more than doubled in time and nine ran out of memory. — [Paper PDF](https://www.cs.au.dk/~amoeller/papers/approx/paper.pdf); [PLDI page](https://pldi24.sigplan.org/details/pldi-2024-papers/48/Reducing-Static-Analysis-Unsoundness-with-Approximate-Interpretation); [Jelly GitHub](https://www.github.com/cs-au-dk/jelly)
- [BACKGROUND] An npm package of Jelly's call-graph output exists as `@nuanced-dev/jellycg`, which suggests Jelly is being embedded in third-party tooling. — [socket.dev](https://socket.dev/npm/package/@nuanced-dev/jellycg)
- [BACKGROUND, ICSE 2023] **JARVIS** is an application-centered, demand-driven, flow-sensitive Python call-graph generator positioned as a PyCG successor. Its numbers were not re-verified here. — [arXiv 2305.05949](https://arxiv.org/html/2305.05949v3); [project page](https://pythonjarvis.github.io/)
- [TITLE-ONLY] **"InferCG: Enhancing Python Call Graph Generation via Static Analysis and Large Language Models"**, ACM (DOI 10.1145/3799990; likely a 2025–2026 journal such as TOSEM, unverified). It is a hybrid static+LLM Python call-graph generator. The fetch failed, so no numbers are available. — [ACM DL](https://dl.acm.org/doi/10.1145/3799990)
- [TITLE-ONLY] A Paderborn thesis, "LLMs for Call Graph Construction: Benchmarking and …", and a dissertation, "Advances in Python Call-Graph Construction and Type …", appear to extend SWARM-CG. — [thesis PDF](https://digital.ub.uni-paderborn.de/hs/content/titleinfo/8212050/full.pdf); [dissertation PDF](https://digital.ub.uni-paderborn.de/hs/content/titleinfo/8295440/full.pdf)
- [BACKGROUND, ISSTA 2024] "Total Recall? How Good Are Static Call Graphs Really?" measures the recall of static call graphs against dynamic ones. — [ACM DL](https://dl.acm.org/doi/10.1145/3650212.3652114)
- [BACKGROUND, 2024] "Static JavaScript Call Graphs: a Comparative Study" (arXiv 2405.07206). — [arXiv](https://arxiv.org/pdf/2405.07206)
- [BACKGROUND, CC 2024] "A Context-Sensitive Pointer Analysis Framework for Rust and Its Application to Call Graph Construction" (Rupta). This is the main Rust call-graph reference found; no 2025–2026 Rust- or Go-specific call-graph benchmark turned up. — [ACM DL](https://dl.acm.org/doi/10.1145/3640537.3641574)
- [TITLE-ONLY, IEEE S&P 2025] "Redefining Indirect Call Analysis with KallGraph" covers C/kernel indirect calls. — [IEEE CSDL](https://www.computer.org/csdl/proceedings-article/sp/2025/223600c734/26hiV5YjNXG)
- [TITLE-ONLY, arXiv 2609.33547, 2026] "Neuro-Symbolic Indirect-Call Analysis under Opaque Pointers". — [arXiv](https://arxiv.org/pdf/2609.33547)

### Inferences
- **Edge provenance and confidence.** A Tree-sitter/SCIP call graph is roughly a name- and type-based resolver, close in spirit to CHA, or to PyCG-style assignment graphs for Python. Architekton should tag every edge with its provenance: `scip-exact` (SCIP symbol occurrence resolves to a definition), `ts-name-heuristic`, `dynamic-observed`, or `ranked-candidate`. The UI should then let users filter by confidence. The ICSE 2026 result suggests deterministic edges are already high-precision when validated, so a learned pruner is not worth shipping.
- **Unresolved JS/Python call sites.** Show a ranked top-k candidate list (as GRAPHIA does) rather than adding edges, and keep candidates out of complexity aggregation by default. A deterministic stand-in for GRAPHIA is to rank candidates by arity match, property-name match, import proximity and file distance; no GNN is needed for an offline tool.
- **Runtime hints from existing telemetry.** This mirrors approximate interpretation. Since Architekton already captures runtime stack samples (d), observed caller→callee pairs from those samples can serve as "hints" that promote ranked candidates to `dynamic-observed` edges. That closes the loop between features (a) and (d).
- **Using Jelly directly.** For JS/TS, consider shipping or optionally invoking Jelly (Node-based, MIT-licensed; license should be verified) as a higher-precision backend over the Tree-sitter baseline.

### Gaps
- I found no 2025–2026 benchmark comparing call-graph tools for **Go or Rust**. The Go standard (x/tools callgraph: CHA, RTA, VTA) was not re-checked in this pass.
- I could not fetch InferCG's numbers.
- I found no study that measures call-graph accuracy from **SCIP indexes** (scip-typescript, scip-python) specifically.

## Q2. Graph summarization, compression and clustering for architecture recovery (2025–2026)

### Takeaway
The 2025–2026 trend combines deterministic structure (AST reference graphs, modularity clustering) with optional LLM labeling and refinement. LLMs are used to *name and refine* clusters, not to replace the graph. Guided Leiden (adding synthetic "force" nodes for domains, data stores and test cases) is a concrete, deterministic recipe for multi-criteria clustering. Architekton's 5-level zoom should be a deterministic hierarchy, built with Leiden plus constraints, with LLM naming as an optional layer on top.

### Cited Findings
- **"ArchAgent: Scalable Legacy Software Architecture Recovery with LLMs"**, arXiv 2601.13007, January 2026 (Pan, Mao, Ma, Ling; HiThink Research and USTC; venue not stated). **Pipeline:**
  - AST-based reference graphs at file, class and function levels feed a File Summarizer.
  - "Adaptive grouping" splits the repository into G = ⌈T/M⌉ token-balanced groups, traversed in DFS order with about 10% overlap between adjacent groups. This is partitioning, not ACDC- or Bunch-style clustering.
  - The Readme Generator traces call chains downstream from entry points.
  - An Architect agent emits partial Mermaid diagrams that are merged into one.

  **Results:** mean F1 was 0.966 against 0.860 for DeepWiki (8 repos, 30 engineers, p = 0.0036). Adding dependency context improved F1 by 0.11 for Qwen3-32B. The paper does **not** use MoJoFM, a2a or c2c, the benchmark is small, and the ratings come from the authors' side. — [arXiv 2601.13007](https://arxiv.org/pdf/2601.13007)
- **"Using Guided Community Detection to Improve Existing Microservice Designs"** (Univ. Vienna; ResearchGate dates it to c. 2025–2026, venue unverified). The method runs Leiden modularity maximization on an API call graph. It adds weighted synthetic nodes and edges as "granularity forces":
  - a domain node per service (weight wd)
  - a node per data store, linked to the APIs that read or write it (wp)
  - a node per end-to-end test case, linked to the APIs it calls (wu)
  - extra weight on existing cross-service call edges (wc)

  Combined with a sweep over maximum community size (2–10 APIs per cluster), the weight permutations produced 2,048 solutions, 108 of them Pareto-optimal. The study system was Train Ticket (39 services, 133 cross-service calls). A balanced solution reached NCR 0.56 (about half the network calls) with DMR 0.31 and DAVR 0.10. Unconstrained brute force reached NCR 0.17 but violated every boundary (DMR = DAVR = 1.0). The graph combines static parsing with dynamic end-to-end test traces. — [PDF](https://eprints.cs.univie.ac.at/8461/1/paper_140.pdf); [ResearchGate](https://www.researchgate.net/publication/399373320_Using_Guided_Community_Detection_to_Improve_Existing_Microservice_Designs)
- [TITLE-ONLY] **"Semantic-Enhanced Automatic Refinement of Architecture Recovery Results Using LLMs"**, arXiv 2607.23774 (2026). It uses LLMs to refine the output of classical recovery tools. — [arXiv](https://arxiv.org/pdf/2607.23774)
- [TITLE-ONLY] **"Software Architecture Recovery Augmented With Semantics"** (2025). — [ResearchGate](https://www.researchgate.net/publication/396583765_Software_Architecture_Recovery_Augmented_with_Semantics)
- [TITLE-ONLY] **"Towards LLM-Assisted Architecture Recovery for Real-World ROS 2 Systems"**, arXiv 2605.20055 (2026). — [arXiv](https://arxiv.org/pdf/2605.20055)
- [TITLE-ONLY] **"SAKE: Software Architectural Knowledge Evaluation Benchmark for LLMs"**, arXiv 2606.29520 (2026). — [arXiv](https://arxiv.org/html/2606.29520v1)
- [TITLE-ONLY] **"LD-Leiden: Local Parallel Community Detection in Large Dynamic Networks"**, arXiv 2502.18497 (2025). It does incremental Leiden updates on changing graphs, which is relevant to re-clustering after code edits. — [arXiv](https://arxiv.org/abs/2502.18497)
- [BACKGROUND, 2021] "Extracting Software Modules as Communities" (Sas) applies community detection to module extraction. — [CEUR](https://ceur-ws.org/Vol-2912/paper4.pdf)

### Inferences
- **Deterministic semantic-zoom recipe** (L5 to L1):
  - **L1 (top):** packages/crates/modules from the build manifest.
  - **L2:** Leiden communities, with a fixed seed so runs are deterministic, on a weighted graph of call edges plus import edges plus "force" nodes. The force nodes are directory, package, test-case co-coverage and (optionally) hot-path co-occurrence from telemetry, following the Guided Leiden recipe.
  - **L3:** files.
  - **L4:** classes and types.
  - **L5:** functions.

  Re-running Leiden on each community gives a recursive hierarchy, or the CPM objective can be used with a resolution sweep. Layout jitter can be kept low with a fixed seed, plus LD-Leiden-style incremental updates after edits.
- **Exposed weights.** Surface the force weights (wd, wp, wu, wc) as user sliders. The Vienna paper shows that different weightings produce materially different, Pareto-trade-off decompositions.
- **Token-balanced partitions for LLM summaries.** ArchAgent's ⌈T/M⌉ grouping with about 10% overlap is useful if Architekton later adds optional local-LLM cluster summaries. Each Leiden cluster becomes a token-budgeted unit for the summarizer.
- **Evaluation.** Use MoJoFM, a2a and c2c (classical architecture-recovery metrics) against package structure as a sanity baseline. The 2026 LLM papers mostly use human-rated F1 instead, which is not reproducible offline.

### Gaps
- I found no 2025–2026 paper evaluating **dominator-tree or k-core collapsing** specifically for call-graph visualization. Searches returned nothing focused, so these remain engineering heuristics with no new evidence.
- No 2025–2026 MoJoFM comparison of Leiden against Louvain, ACDC or LIMBO on software graphs was found.

## Q3. Static complexity (Big-O) inference: LLM, symbolic and dynamic approaches (2025–2026)

### Takeaway
LLMs remain poor at Big-O prediction: the best time-complexity pass@1 on BigO(Bench) is about 29%. The BigO(Bench) *labeling framework* is itself a deterministic, empirical recipe (input-size scaling, profiling, non-negative least-squares fit, bias toward simpler classes) that matched human labels on 92% of a time sample. For Architekton, the most defensible design is AST loop-nesting and recursion patterns as a static prior, optionally confirmed by a BigO(Bench)-style empirical fit when the user can run the function, with neither presented as a proof.

### Cited Findings
- **"BigO(Bench): Can LLMs Generate Code with Controlled Time and Space Complexity?"**, FAIR at Meta and Inria, arXiv 2503.15242 (2025; no peer-reviewed venue stated in v2). **Dataset:** 3,105 problems and 1,190,250 Python solutions from Code Contests. The time test set has 311 problems, 640 solutions and 11 classes; the space test set has 308 problems, 636 solutions and 5 classes. **Labeling framework:**
  - It generates larger synthetic inputs from one sample input, using several size-expansion strategies.
  - Runs are sandboxed with Bubblewrap; runtime is measured with cProfile and memory with tracemalloc.
  - A non-negative least-squares fit is made per complexity class, and the class with minimal residual wins, with a bias toward simpler classes.
  - Results are aggregated across input subsets, targeting worst-case behavior.

  It agreed with human labels on 92% of 50 samples for time and 84% for space. **LLM results:** the best time-complexity *prediction* pass@1 was 29.2 (DeepSeek-R1 Llama 70B), and space pass@1 was 25.6. — [arXiv 2503.15242](https://arxiv.org/html/2503.15242v2); [project](https://facebookresearch.github.io/BigOBench); [leaderboard](https://facebookresearch.github.io/BigOBench/leaderboard.html)
- **"CodeComplex: Dataset for Worst-Case Time Complexity Prediction"**, Baik et al., *Findings of EMNLP 2025* (pp. 19616–19638). It contains 4,900 Java programs and an equal number of Python programs labeled with worst-case time complexity. The landing page did not report model accuracy or the class list; the arXiv version is 2401.08719, and the GitHub repo tests state-of-the-art models. — [ACL Anthology](https://aclanthology.org/2025.findings-emnlp.1069/); [arXiv 2401.08719](https://arxiv.org/abs/2401.08719); [GitHub](https://github.com/sybaik1/CodeComplex)
- [TITLE-ONLY] "Where Do LLMs Still Struggle? An In-Depth Analysis of Code Generation Benchmarks", arXiv 2511.04355 (2025). — [arXiv](https://arxiv.org/html/2511.04355v1)

### Inferences
- **Static Big-O rule engine** over Tree-sitter ASTs. Classify loops as counted (`for i in range(n)`, `for (i=0;i<n;i++)`), halving or doubling (`i*=2`, `n//=2`, which gives O(log n)), or collection iteration (O(|c|)). Multiply for nesting and add for sequence. Treat recursion as a recurrence: one self-call on n−1 gives O(n), two self-calls on n/2 match the Master theorem, and two calls on n−1 give O(2ⁿ). Map known library calls through a cost table (e.g. `sort` is O(n log n), `list.index` and `in list` are O(n), `dict` get is O(1)). Emit a confidence and a "why" trace per function. CodeComplex (Java/Python, worst-case labels) can serve as an **offline regression test set** for this engine; class confusion should be measured there.
- **Optional "Verify empirically" action.** Port the BigO(Bench) recipe: generate inputs at sizes such as 2^k, time them, fit the candidate classes {1, log n, n, n log n, n², n³, 2ⁿ} with NNLS, and prefer the simpler class on near-ties. Architekton can also reuse its own telemetry here: per-function CPU time from stack samples, regressed against a user-supplied size metric, gives an "observed complexity" badge.
- **No LLM for the primary Big-O label.** About 29% accuracy is far below what deterministic rules can likely reach on typical loop-nest code, though this is not measured here.

### Gaps
- I found no 2025–2026 *deterministic* loop-bound or cost-analysis paper (e.g. new work in the KoAT/Loopus/CoFloCo tradition) in this pass; that literature should be searched separately.
- CodeComplex's per-model accuracy numbers were not on the landing page.

## Q4. Static + dynamic fusion: eBPF profiling, unwinding, symbolization, mapping samples to AST functions, async attribution

### Takeaway
The OpenTelemetry eBPF profiler is the reference agent design. It uses `.eh_frame`-based native unwinding (no frame pointers or debug info needed on the host), unwinds interpreters for Python, Node/V8, JVM, .NET, Ruby, PHP, Perl and Erlang, keeps overhead under a 1% CPU / 250 MB test limit, and now emits the alpha OTel Profiles signal. Frame pointers came back in Ubuntu 24.04 and Fedora, but they mis-attribute samples in function prologues (about 5.2% of samples on x86) and in hand-written assembly. SFrame is the kernel path forward: deferred-unwind infrastructure landed in Linux 6.17, glibc 2.42 and binutils 2.45 support SFrame, and arch patches were still in review in 2026. Research on static+dynamic call-graph fusion (FSE 2025 IVR) shows that runtime-observed entry points greatly enlarge static call graphs.

### Cited Findings
- **OpenTelemetry eBPF profiler** (GitHub, 2025–2026):
  - **Unwinding and runtimes:** it unwinds native code via `.eh_frame`, so it needs neither DWARF debug info nor frame pointers. It produces mixed stacks from the kernel through system libraries into Hotspot JVM, Python, Ruby, PHP, Node.js/V8, Perl, Erlang and .NET, with no agents, instrumentation or restarts.
  - **Overhead:** the test limits are 1% CPU and 250 MB memory, and the agent typically runs well below them.
  - **Status:** it implements the **alpha OTel Profiles signal** and describes itself as "functional but work-in-progress / evolving". Backends include devfiler (testing only) and Pyroscope.
  - **Kernel support:** the minimum kernel keeps rising; newer commits may require 5.10 or later. — [GitHub](https://github.com/open-telemetry/opentelemetry-ebpf-profiler); [DeepWiki on stack deltas](https://deepwiki.com/open-telemetry/opentelemetry-ebpf-profiler/6.2-elf-processing-and-go-runtime-support)
- [BACKGROUND, 2024] **Frame pointers.** Brendan Gregg's "The Return of the Frame Pointers" documents distributions re-enabling frame pointers, and Ubuntu 24.04 LTS enables them by default. — [Gregg blog](https://brendangregg.com/blog/2024-03-17/the-return-of-the-frame-pointers.html); [Ubuntu blog](https://ubuntu.com/blog/ubuntu-performance-engineering-with-frame-pointers-by-default)
- [BACKGROUND, Oct 2024] **Red Hat, "Limitations of frame pointer unwinding":**
  - Frame pointers cost about 1–2% in performance.
  - They are wrong in prologues, epilogues and PLT stubs. Samples in the first 8 bytes of an x86 function were about 5.2% of all samples (at least 6.0% on aarch64), so the innermost caller can be skipped.
  - Hand-written glibc string and memory routines break frame-pointer chains.
  - Alternatives: eu-stacktrace (elfutils 0.192), SFrame, and future hardware shadow stacks. — [Red Hat Developers](https://developers.redhat.com/articles/2024/10/30/limitations-frame-pointer-unwinding)
- **SFrame status, 2025–2026.** Linux 6.17 merged the deferred user-space unwinding infrastructure (Rostedt), which is the base for SFrame. x86 and s390 support plus perf and ftrace integration were still pending at merge time. Userspace SFrame support shipped in glibc 2.42 and binutils 2.45. — [Phoronix](https://www.phoronix.com/news/Linux-6.17-Deferred-Unwinder). s390 SFrame user-space unwinding was at v4 in January 2026, and arm64 SFrame patches were posted in April 2026, so both were still under review. — [LKML s390 v4](https://lkml.org/lkml/2026/1/27/1225); [arm64 v1](https://ratatoskr.run/lkml/2026/04/3518075/t)
- **"Do you have 5 min? Improving Call Graph Analysis with Runtime Information"**, FSE 2025 (Ideas, Visions and Reflections track), Univ. Luxembourg (Klein et al.). The method runs an Android app for 5 minutes under instrumentation (AppRunner plus the Monkey UI exerciser). It takes the root nodes of the dynamic call graph (callbacks, framework-invoked methods) as *extra entry points* for the static Soot/SPARK call graph. Average graph size grew from 50,626 to 65,534 nodes (median from 25,899 to 46,307). Static analysis time rose from 367 s to 444 s on average. FlowDroid taint flows rose from 105 to 152, and 10 of 10 sampled new flows were true positives. The study used 78 usable apps out of 100, in Java/Android. The paper cites prior work that CHA misses at least 40% of methods. — [PDF](https://jacquesklein2302.github.io/papers/2025-FSE-IVR-call_graph.pdf); [FSE page](https://conf.researchr.org/details/fse-2025/fse-2025-ideas-visions-and-reflections/5/Do-you-have-5-min-Improving-Call-Graph-Analysis-with-Runtime-Information)
- [TITLE-ONLY, 2025] "CGPatch: Streamlining Static Call Graph Validation Using Selective Instrumentation" (Springer LNCS chapter). — [Springer](https://link.springer.com/chapter/10.1007/978-3-032-07612-0_22)
- [TITLE-ONLY, 2025–2026] "A Compiler-Assisted Workflow for Efficiency-Guided Selective Tracing" (Springer). — [Springer](https://link.springer.com/chapter/10.1007/978-3-032-35248-4_13)
- [TITLE-ONLY] "GraalNN: Context-Sensitive Static Profiling with Graph Neural Networks" (ACM, DOI 10.1145/3696443.3708958; likely CGO 2025, unverified). It predicts profiles statically. — [ACM DL](https://dl.acm.org/doi/10.1145/3696443.3708958)

### Inferences
- **Sample to AST attribution pipeline:**
  1. **Native code:** resolve the PC to a symbol via ELF `.symtab` or `.dynsym` (PDB on Windows), then to file and line via DWARF `.debug_line` or split DWARF/debuginfod. When debug info is present, also expand inlined frames via `DW_TAG_inlined_subroutine`.
  2. **Function lookup:** look up (file, line) in an **interval tree of Tree-sitter function node byte and line ranges**. The innermost enclosing function node wins, and closures or lambdas get their own nodes.
  3. **Join:** match that node to the SCIP symbol for the call-graph node.
  4. **Interpreted code (Python, Node):** the OTel profiler already returns (file, function name, line), so steps 2 and 3 apply directly.
- **Prologue mis-attribution.** If Architekton does its own frame-pointer unwinding, the about 5% of samples that land in prologues will attach to the *caller's caller*. Mitigations: fall back to `.eh_frame`/SFrame for the leaf frame (as OTel does), or flag leaf frames whose PC offset is under 8 bytes.
- **Runtime-observed entry points, FSE 2025 style.** Stack-sample roots (thread start functions, callbacks invoked from frameworks or event loops) should be added as entry points to Architekton's reachability analysis. Edges seen in samples but missing from the static graph should be drawn as "dynamic-only" edges. This directly fixes unsoundness in the Tree-sitter call graph.
- **Async-task attribution.** Approaches, none verified with a 2025–2026 source in this pass:
  - Tokio task IDs and tracing spans.
  - Go goroutine pprof labels.
  - Node `AsyncLocalStorage`/`async_hooks`.
  - Python asyncio task names.

  A deterministic fallback that needs no runtime cooperation is to attribute await-resumption frames to the `async fn` AST node and to aggregate per coroutine *definition* rather than per OS thread.
- **Windows.** ETW stack walking (via the `StackWalk` events of the Kernel Logger/SampledProfile) serves as the analogue. It was not researched in this pass.

### Gaps
- I found no 2025–2026 peer-reviewed paper that specifically fuses **eBPF stack samples with AST/SCIP call graphs** for program comprehension. The closest are the FSE 2025 IVR paper (Android, instrumentation-based) and CGPatch (title only).
- Parca-specific 2025 symbolization architecture (off-host symbolization via debuginfod) was not fetched; the claim that OTel does off-host symbolization is unverified here.
- No 2025–2026 source on async-task stack attribution in eBPF profilers was found in this pass.
- The ETW / Windows Performance Recorder sampling-to-source mapping was not researched.
