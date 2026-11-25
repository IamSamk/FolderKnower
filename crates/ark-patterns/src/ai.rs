//! AI / ML architecture recognition: stages, settings and the data-flow between them.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use ark_link::{ParsedFile, file_id, fn_id};
use ark_parse::CallFact;
use ark_schema::{
    AiArchitecture, AiEdge, AiRole, AiStage, AiSystemKind, KnownLibrary, MemoryTechnique,
};
use regex::Regex;

use crate::catalog::{CATALOG, Catalog, import_matches};

static DICT_PAIR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"["']?([A-Za-z_][A-Za-z0-9_]*)["']?\s*:\s*([^,}\]]+)"#).unwrap());

pub fn role_explain(role: AiRole) -> &'static str {
    match role {
        AiRole::Loader => {
            "Loads raw documents (PDF, web pages, files) and turns them into text plus metadata."
        }
        AiRole::Splitter => {
            "Chunking. Cuts long text into overlapping pieces small enough to embed and to fit in the prompt. chunk_size is the piece length, chunk_overlap repeats the end of one chunk at the start of the next so a sentence is not lost at the cut."
        }
        AiRole::Embedder => {
            "Embedding model. Turns each chunk (and later the question) into a vector of numbers so that similar meaning means nearby vectors."
        }
        AiRole::VectorStore => {
            "Vector store. Keeps every chunk's vector with its text and metadata, indexed (often HNSW) for fast nearest-neighbour search."
        }
        AiRole::KeywordIndex => {
            "Keyword index (BM25). Scores chunks by matching words, which catches exact names, ids and error codes that embeddings blur."
        }
        AiRole::Retriever => {
            "Retriever. Embeds the question and returns the k chunks whose vectors are closest (top-k)."
        }
        AiRole::Fusion => {
            "Fusion. Merges the dense and keyword result lists into one ranking (often Reciprocal Rank Fusion)."
        }
        AiRole::Reranker => {
            "Reranker (cross-encoder). Reads the question and each candidate chunk together and scores how well they match. Slower than vector search but much more precise, so it re-orders the top-k candidates and keeps the best top-n."
        }
        AiRole::Prompt => {
            "Prompt template. Puts the instructions, the retrieved chunks and the question into the text the model will read."
        }
        AiRole::Router => {
            "Decision step (router). Looks at the current state or the model's answer and picks what happens next: call a tool, retrieve again, or finish."
        }
        AiRole::Llm => {
            "Language model call. Reads the prompt and generates the answer token by token; with tools enabled it can answer with a tool call instead."
        }
        AiRole::Tool => {
            "Tool. A function the model may ask to run (search, calculator, database query). Its result is fed back to the model."
        }
        AiRole::Agent => {
            "Agent loop. The model chooses a tool, the tool runs, the result goes back to the model, and this repeats until the model decides it is done."
        }
        AiRole::Memory => {
            "Conversation memory. Keeps earlier turns (all, the last k, or a summary) and adds them to the next prompt."
        }
        AiRole::Cache => {
            "Response cache. Returns a stored answer for a repeated (or, for a semantic cache, similar) question without calling the model."
        }
        AiRole::Guardrail => {
            "Guardrail. Checks the input or output against rules (safety, format, PII) before it goes further."
        }
        AiRole::Evaluator => {
            "Evaluator. Scores answers (faithfulness to the retrieved context, relevance) to measure the pipeline."
        }
        AiRole::ModelServer => {
            "Model server. Loads the weights once and serves many requests, batching them and managing the KV cache."
        }
        AiRole::Tokenizer => {
            "Tokenizer. Splits text into token ids, the units a model reads; chunk and context limits are counted in tokens."
        }
        AiRole::NeuralLayer => "Neural network layer defined in code.",
        AiRole::Trainer => {
            "Training. Runs batches through the model, computes the loss and updates the weights with an optimizer."
        }
    }
}

fn role_slug(r: AiRole) -> String {
    let s = format!("{r:?}");
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_ascii_lowercase().to_string() + c.as_str(),
        None => s,
    }
}

fn key(c: &CallFact) -> String {
    match &c.receiver {
        Some(r) => format!("{r}.{}", c.callee),
        None => c.callee.clone(),
    }
}

/// `Chroma.from_documents` rather than a bare `from_documents`.
fn stage_name(c: &CallFact) -> String {
    let generic = c.callee.starts_with("from_")
        || c.callee.starts_with("from")
            && c.callee
                .chars()
                .nth(4)
                .is_some_and(|ch| ch.is_ascii_uppercase())
        || matches!(
            c.callee.as_str(),
            "load_local" | "as_retriever" | "asRetriever" | "invoke"
        );
    match (&c.receiver, generic) {
        (Some(r), true) if !r.contains('(') => {
            let last = r.rsplit('.').next().unwrap_or(r);
            if last
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase())
            {
                format!("{last}.{}", c.callee)
            } else {
                c.callee.clone()
            }
        }
        _ => c.callee.clone(),
    }
}

fn unquote(v: &str) -> String {
    v.trim().trim_matches(['"', '\'', '`']).to_owned()
}

fn params_of(
    c: &CallFact,
    wanted: &[String],
    positional: Option<&str>,
) -> BTreeMap<String, String> {
    let mut p = BTreeMap::new();
    if let (Some(name), Some(s)) = (positional, &c.first_string) {
        p.insert(name.to_owned(), s.clone());
    }
    for (k, v) in &c.kwargs {
        if wanted.iter().any(|w| w == k) {
            p.insert(k.clone(), unquote(v));
        }
        if v.starts_with('{') || v.starts_with("dict(") {
            for cap in DICT_PAIR.captures_iter(v) {
                let ik = &cap[1];
                if wanted.iter().any(|w| w == ik)
                    || matches!(ik, "k" | "top_k" | "fetch_k" | "score_threshold")
                {
                    p.insert(ik.to_owned(), unquote(&cap[2]));
                }
            }
        }
    }
    p
}

struct Site<'a> {
    path: &'a str,
    fn_qual: Option<&'a str>,
    call: &'a CallFact,
    imports: &'a [String],
}

fn requires_ok(req: &[String], imports: &[String]) -> bool {
    req.is_empty()
        || req
            .iter()
            .any(|r| imports.iter().any(|i| import_matches(i, r)))
}

pub struct AiOut {
    pub ai: Option<AiArchitecture>,
    pub memory: Vec<MemoryTechnique>,
    pub libraries: Vec<KnownLibrary>,
}

pub fn analyze(files: &[ParsedFile], okf_evidence: &[String]) -> AiOut {
    analyze_with(&CATALOG, files, okf_evidence)
}

pub fn analyze_with(cat: &Catalog, files: &[ParsedFile], okf_evidence: &[String]) -> AiOut {
    let mut stages: BTreeMap<(AiRole, String), AiStage> = BTreeMap::new();
    let mut uses: BTreeMap<(AiRole, String), u32> = BTreeMap::new();
    let mut frameworks: BTreeSet<String> = BTreeSet::new();
    let mut memory: BTreeMap<String, MemoryTechnique> = BTreeMap::new();
    let mut graph_nodes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for pf in files {
        let path = pf.scan.path.as_str();
        let imports: Vec<String> = pf.facts.imports.iter().map(|i| i.source.clone()).collect();
        let mut sites: Vec<Site> = Vec::new();
        for f in &pf.facts.functions {
            for c in &f.calls {
                sites.push(Site {
                    path,
                    fn_qual: Some(&f.qualname),
                    call: c,
                    imports: &imports,
                });
            }
        }
        for c in &pf.facts.top_calls {
            sites.push(Site {
                path,
                fn_qual: None,
                call: c,
                imports: &imports,
            });
        }

        for s in &sites {
            let k = key(s.call);
            // LangGraph node names: graph.add_node("retrieve", retrieve)
            if matches!(s.call.callee.as_str(), "add_node" | "addNode")
                && let Some(n) = &s.call.first_string
                && requires_ok(
                    &["langgraph".into(), "@langchain/langgraph".into()],
                    s.imports,
                )
            {
                graph_nodes
                    .entry(path.to_owned())
                    .or_default()
                    .insert(n.clone());
            }
            if let Some(comp) = cat.components.iter().find(|c| {
                c.patterns.iter().any(|p| p.is_match(&k)) && requires_ok(&c.requires, s.imports)
            }) {
                let name = stage_name(s.call);
                let sk = (comp.role, name.clone());
                *uses.entry(sk.clone()).or_default() += 1;
                frameworks.insert(comp.library.clone());
                let params = params_of(s.call, &comp.params, comp.positional.as_deref());
                let entry = stages.entry(sk).or_insert_with(|| AiStage {
                    id: format!("ai:{}:{name}", role_slug(comp.role)),
                    role: comp.role,
                    name: name.clone(),
                    library: comp.library.clone(),
                    file_id: file_id(s.path),
                    function_id: s.fn_qual.map(|q| fn_id(s.path, q)),
                    line: s.call.line,
                    params: BTreeMap::new(),
                    rule_id: comp.id.clone(),
                    explain: comp
                        .explain
                        .clone()
                        .unwrap_or_else(|| role_explain(comp.role).to_owned()),
                });
                for (pk, pv) in params {
                    entry.params.entry(pk).or_insert(pv);
                }
            }
            for m in &cat.memory {
                if m.patterns.is_empty()
                    || !m.patterns.iter().any(|p| p.is_match(&k))
                    || !requires_ok(&m.requires, s.imports)
                {
                    continue;
                }
                let setting = match &m.kwarg {
                    Some(re) => {
                        let hits: Vec<String> = s
                            .call
                            .kwargs
                            .iter()
                            .filter(|(kk, _)| re.is_match(kk))
                            .map(|(kk, v)| format!("{kk}={v}"))
                            .collect();
                        if hits.is_empty() {
                            continue;
                        }
                        hits.join(", ")
                    }
                    None => {
                        let kw: Vec<String> = s
                            .call
                            .kwargs
                            .iter()
                            .take(4)
                            .map(|(kk, v)| format!("{kk}={v}"))
                            .collect();
                        if kw.is_empty() {
                            format!("{k}(...)")
                        } else {
                            format!("{k}({})", kw.join(", "))
                        }
                    }
                };
                memory
                    .entry(format!("{}@{}", m.id, s.path))
                    .or_insert_with(|| MemoryTechnique {
                        id: format!("mem:{}:{}", m.id, s.path),
                        name: m.name.clone(),
                        engine: m.engine.clone(),
                        file_id: file_id(s.path),
                        line: s.call.line,
                        setting,
                        explain: m.explain.clone(),
                    });
            }
        }

        // Identifier-only memory rules (e.g. Anthropic cache_control dict keys).
        for m in cat.memory.iter().filter(|m| m.ident.is_some()) {
            let id = m.ident.as_deref().unwrap_or_default();
            if let Some(f) = pf
                .facts
                .functions
                .iter()
                .find(|f| f.idents.iter().any(|i| i == id))
            {
                memory
                    .entry(format!("{}@{}", m.id, path))
                    .or_insert_with(|| MemoryTechnique {
                        id: format!("mem:{}:{path}", m.id),
                        name: m.name.clone(),
                        engine: m.engine.clone(),
                        file_id: file_id(path),
                        line: f.line_start,
                        setting: format!("{id} in {}", f.qualname),
                        explain: m.explain.clone(),
                    });
            }
        }

        // Python tool decorators: @tool, @mcp.tool(), @function_tool.
        for f in &pf.facts.functions {
            if f.decorators.iter().any(|d| {
                let d = d.trim_start_matches('@');
                d == "tool"
                    || d.starts_with("tool(")
                    || d.ends_with(".tool")
                    || d.ends_with(".tool()")
                    || d.starts_with("function_tool")
            }) {
                let sk = (AiRole::Tool, f.name.clone());
                *uses.entry(sk.clone()).or_default() += 1;
                stages.entry(sk).or_insert_with(|| AiStage {
                    id: format!("ai:tool:{}", f.name),
                    role: AiRole::Tool,
                    name: f.name.clone(),
                    library: "tool function".to_owned(),
                    file_id: file_id(path),
                    function_id: Some(fn_id(path, &f.qualname)),
                    line: f.line_start,
                    params: BTreeMap::new(),
                    rule_id: "ai.tool.decorated-function".to_owned(),
                    explain: role_explain(AiRole::Tool).to_owned(),
                });
            }
        }

        // Custom torch modules: class Attention(nn.Module).
        let torch = requires_ok(&["torch".into()], &imports);
        for c in &pf.facts.classes {
            if torch
                && c.bases
                    .iter()
                    .any(|b| b == "nn.Module" || b == "Module" || b.ends_with("PreTrainedModel"))
            {
                let n = c.name.as_str();
                let lower = n.to_ascii_lowercase();
                let explain = if lower.contains("attention") {
                    "A custom attention module: projects tokens to queries, keys and values and mixes values by softmax(QK^T / sqrt(d)). During generation its keys and values are what the KV cache stores."
                } else if lower.contains("mlp")
                    || lower.contains("feedforward")
                    || lower.contains("ffn")
                {
                    "The feed-forward (MLP) part of a transformer block: two linear layers with a non-linearity, applied to each token on its own."
                } else if lower.contains("block")
                    || lower.contains("layer")
                    || lower.contains("decoder")
                    || lower.contains("encoder")
                {
                    "A transformer block: attention (tokens look at each other) then an MLP (each token is transformed), each wrapped in a residual connection and normalization."
                } else if lower.contains("embed") {
                    "An embedding layer: a lookup table from token id to vector, often plus a position encoding."
                } else if lower.contains("norm") {
                    "A normalization layer (LayerNorm / RMSNorm) that keeps activations at a stable scale."
                } else {
                    "A custom neural network module (nn.Module): its forward() defines how input tensors flow through the layers."
                };
                stages
                    .entry((AiRole::NeuralLayer, n.to_owned()))
                    .or_insert_with(|| AiStage {
                        id: format!("ai:neuralLayer:{n}"),
                        role: AiRole::NeuralLayer,
                        name: n.to_owned(),
                        library: "PyTorch".to_owned(),
                        file_id: file_id(path),
                        function_id: None,
                        line: c.line,
                        params: BTreeMap::new(),
                        rule_id: "ai.nn.custom-module".to_owned(),
                        explain: explain.to_owned(),
                    });
                frameworks.insert("PyTorch".to_owned());
            }
        }
    }

    // Attach LangGraph node names to the graph router.
    if !graph_nodes.is_empty() {
        let names: BTreeSet<&String> = graph_nodes.values().flatten().collect();
        let joined = names.into_iter().cloned().collect::<Vec<_>>().join(", ");
        if let Some(st) = stages
            .values_mut()
            .find(|s| s.role == AiRole::Router && s.rule_id == "ai.router.langgraph")
        {
            st.params.insert("nodes".to_owned(), joined);
        }
    }
    for (k, n) in &uses {
        if *n > 1
            && let Some(s) = stages.get_mut(k)
        {
            s.params.insert("uses".to_owned(), n.to_string());
        }
    }

    let libraries = libraries(cat, files, okf_evidence);
    let mem: Vec<MemoryTechnique> = memory.into_values().collect();
    let stages: Vec<AiStage> = stages.into_values().collect();
    let ai = assemble(stages, frameworks);
    AiOut {
        ai,
        memory: mem,
        libraries,
    }
}

fn libraries(cat: &Catalog, files: &[ParsedFile], okf: &[String]) -> Vec<KnownLibrary> {
    let mut out = Vec::new();
    for lib in &cat.libraries {
        let mut ev: BTreeSet<String> = BTreeSet::new();
        for pf in files {
            for i in &pf.facts.imports {
                if lib.imports.iter().any(|p| import_matches(&i.source, p)) {
                    ev.insert(i.source.clone());
                }
            }
        }
        if lib.name.starts_with("Open Knowledge Format") {
            ev.extend(okf.iter().cloned());
        }
        if !ev.is_empty() {
            out.push(KnownLibrary {
                name: lib.name.clone(),
                category: lib.category.clone(),
                explain: lib.explain.clone(),
                evidence: ev.into_iter().take(6).collect(),
            });
        }
    }
    out
}

/// Data-flow order used to wire stages.
const CHAIN: &[(AiRole, &str)] = &[
    (AiRole::Loader, "documents"),
    (AiRole::Splitter, "chunks"),
    (AiRole::Embedder, "vectors"),
    (AiRole::VectorStore, "nearest-neighbour search"),
    (AiRole::Retriever, "top-k chunks"),
    (AiRole::Fusion, "fused candidates"),
    (AiRole::Reranker, "top-n chunks"),
    (AiRole::Prompt, "prompt"),
    (AiRole::Llm, ""),
];

fn assemble(stages: Vec<AiStage>, frameworks: BTreeSet<String>) -> Option<AiArchitecture> {
    if stages.is_empty() {
        return None;
    }
    let first = |r: AiRole| stages.iter().find(|s| s.role == r).map(|s| s.id.clone());
    let present = |r: AiRole| stages.iter().any(|s| s.role == r);
    let mut edges: Vec<AiEdge> = Vec::new();
    let add = |from: Option<String>,
               to: Option<String>,
               label: &str,
               loop_back: bool,
               edges: &mut Vec<AiEdge>| {
        if let (Some(f), Some(t)) = (from, to)
            && f != t
            && !edges.iter().any(|e| e.from == f && e.to == t)
        {
            edges.push(AiEdge {
                from: f,
                to: t,
                label: label.to_owned(),
                loop_back,
            });
        }
    };

    // Retrieval chain.
    let mut prev: Option<(String, &str)> = None;
    for (role, label) in CHAIN {
        if let Some(id) = first(*role) {
            if let Some((p, l)) = &prev {
                add(Some(p.clone()), Some(id.clone()), l, false, &mut edges);
            }
            prev = Some((id, label));
        }
    }
    // Keyword branch joins at fusion, else at the reranker, else at the prompt / model.
    if present(AiRole::KeywordIndex) {
        let join = first(AiRole::Fusion)
            .or_else(|| first(AiRole::Reranker))
            .or_else(|| first(AiRole::Prompt))
            .or_else(|| first(AiRole::Llm));
        add(
            first(AiRole::Splitter),
            first(AiRole::KeywordIndex),
            "chunks",
            false,
            &mut edges,
        );
        add(
            first(AiRole::KeywordIndex),
            join,
            "keyword hits",
            false,
            &mut edges,
        );
    }
    let into_model = first(AiRole::Prompt).or_else(|| first(AiRole::Llm));
    add(
        first(AiRole::Memory),
        into_model.clone(),
        "chat history",
        false,
        &mut edges,
    );
    add(
        first(AiRole::Cache),
        first(AiRole::Llm),
        "cache lookup",
        false,
        &mut edges,
    );
    add(
        first(AiRole::Guardrail),
        first(AiRole::Llm),
        "checks input / output",
        false,
        &mut edges,
    );
    add(
        first(AiRole::Tokenizer),
        first(AiRole::Llm).or_else(|| first(AiRole::NeuralLayer)),
        "token ids",
        false,
        &mut edges,
    );
    add(
        first(AiRole::ModelServer),
        first(AiRole::Llm),
        "serves",
        false,
        &mut edges,
    );
    add(
        first(AiRole::Llm),
        first(AiRole::Evaluator),
        "answers to score",
        false,
        &mut edges,
    );
    add(
        first(AiRole::Trainer),
        first(AiRole::NeuralLayer),
        "updates weights",
        false,
        &mut edges,
    );

    // Agent loop: model -> decision -> tool -> model.
    let decider = first(AiRole::Router).or_else(|| first(AiRole::Agent));
    if decider.is_some() {
        add(
            first(AiRole::Llm),
            decider.clone(),
            "decides next step",
            false,
            &mut edges,
        );
        let tool_targets: Vec<String> = stages
            .iter()
            .filter(|s| s.role == AiRole::Tool)
            .map(|s| s.id.clone())
            .take(8)
            .collect();
        for t in &tool_targets {
            add(
                decider.clone(),
                Some(t.clone()),
                "call tool",
                false,
                &mut edges,
            );
            add(
                Some(t.clone()),
                first(AiRole::Llm),
                "tool result",
                true,
                &mut edges,
            );
        }
        if present(AiRole::Retriever) {
            add(
                decider.clone(),
                first(AiRole::Retriever),
                "retrieve (as a tool)",
                false,
                &mut edges,
            );
        }
        if tool_targets.is_empty() && !present(AiRole::Retriever) {
            add(
                decider.clone(),
                first(AiRole::Llm),
                "loop",
                true,
                &mut edges,
            );
        }
        if let (Some(a), Some(r)) = (first(AiRole::Agent), first(AiRole::Router)) {
            add(Some(a), Some(r), "runs", false, &mut edges);
        }
    }

    let has_agent = present(AiRole::Agent) || present(AiRole::Router) || present(AiRole::Tool);
    let has_retr =
        present(AiRole::Retriever) || present(AiRole::VectorStore) || present(AiRole::KeywordIndex);
    let hybrid = present(AiRole::Fusion)
        || (present(AiRole::KeywordIndex)
            && (present(AiRole::VectorStore) || present(AiRole::Embedder)));
    let kind = if has_agent && has_retr {
        AiSystemKind::AgenticRag
    } else if has_agent {
        AiSystemKind::Agent
    } else if hybrid {
        AiSystemKind::HybridRag
    } else if has_retr {
        AiSystemKind::Rag
    } else if present(AiRole::ModelServer) {
        AiSystemKind::ModelServing
    } else if present(AiRole::Trainer) || present(AiRole::NeuralLayer) {
        AiSystemKind::ModelTraining
    } else {
        AiSystemKind::Inference
    };
    let headline = match kind {
        AiSystemKind::AgenticRag => {
            "Agentic RAG: a model decides, in a loop, when to retrieve and which tools to call"
        }
        AiSystemKind::Agent => "Agent: a model picks tools in a loop until it decides it is done",
        AiSystemKind::HybridRag => {
            "Hybrid RAG: dense vectors and BM25 keywords are searched together, then fused"
        }
        AiSystemKind::Rag => {
            "RAG: documents are chunked, embedded, retrieved by similarity and handed to a model"
        }
        AiSystemKind::ModelServing => {
            "Model serving: weights are loaded once and requests share a managed KV cache"
        }
        AiSystemKind::ModelTraining => "Neural network code: layers are defined and trained",
        AiSystemKind::Inference => "Model inference: the app sends prompts to a language model",
    }
    .to_owned();
    edges.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
    Some(AiArchitecture {
        kind,
        headline,
        frameworks: frameworks.into_iter().collect(),
        stages,
        edges,
    })
}
