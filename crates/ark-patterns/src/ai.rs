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
use crate::deployment::InfraFile;

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
    analyze_with(&CATALOG, files, okf_evidence, &[])
}

pub fn analyze_texts(files: &[ParsedFile], okf_evidence: &[String], texts: &[InfraFile]) -> AiOut {
    analyze_with(&CATALOG, files, okf_evidence, texts)
}

pub fn analyze_with(
    cat: &Catalog,
    files: &[ParsedFile],
    okf_evidence: &[String],
    texts: &[InfraFile],
) -> AiOut {
    let mut stages: BTreeMap<(AiRole, String), AiStage> = BTreeMap::new();
    let mut uses: BTreeMap<(AiRole, String), u32> = BTreeMap::new();
    let mut frameworks: BTreeSet<String> = BTreeSet::new();
    let mut memory: BTreeMap<String, MemoryTechnique> = BTreeMap::new();
    let mut graph_nodes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for pf in files {
        let path = pf.scan.path.as_str();
        if is_experiment(path) {
            continue;
        }
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
            // A URL string (`http://localhost:11434/api/chat`) is matched as `url:<text>`.
            let url_key = s
                .call
                .first_string
                .as_deref()
                .filter(|u| u.contains("://") || u.starts_with('/') || u.contains("localhost"))
                // A browser calling a relative path (`/api/chat`, `/v1/chat/completions`) is
                // calling its own backend, not a model server.
                .filter(|u| !(u.starts_with('/') && is_web_source(path)))
                .map(|u| format!("url:{u}"));
            if let Some(comp) = cat.components.iter().find(|c| {
                c.patterns
                    .iter()
                    .any(|p| p.is_match(&k) || url_key.as_deref().is_some_and(|u| p.is_match(u)))
                    && requires_ok(&c.requires, s.imports)
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
                    || REGISTERED_TOOL.is_match(d)
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

    add_mentions(&mut stages, &mut frameworks, texts, files);

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

    // Retrieval as two lanes that meet: dense (embed -> vector search) and lexical
    // (keyword index -> graph expansion). Every stage of a role is wired, not only the
    // first, so a pipeline with two retrievers or two checks shows both.
    let ids = |keep: &dyn Fn(&AiStage) -> bool| -> Vec<String> {
        stages
            .iter()
            .filter(|s| keep(s))
            .map(|s| s.id.clone())
            .take(4)
            .collect()
    };
    fn lane_of(s: &AiStage) -> Option<&str> {
        s.params.get("lane").map(String::as_str)
    }
    let is_check = |s: &AiStage| s.role == AiRole::Router && s.params.contains_key("check");
    let role_is = |r: AiRole| move |s: &AiStage| s.role == r;
    let label_of = |r: AiRole| {
        CHAIN
            .iter()
            .find(|(cr, _)| *cr == r)
            .map_or("", |(_, l)| *l)
    };
    let role_of = |id: &str| stages.iter().find(|s| s.id == id).map(|s| s.role);
    let link_levels = |levels: &[Vec<String>], edges: &mut Vec<AiEdge>| -> Vec<String> {
        let mut prev: Vec<String> = Vec::new();
        for level in levels.iter().filter(|l| !l.is_empty()) {
            for p in &prev {
                for t in level {
                    let label = match (role_of(p), role_of(t)) {
                        (Some(AiRole::Splitter), Some(AiRole::KeywordIndex)) => "chunks",
                        (Some(AiRole::Router), _) => "enough evidence",
                        (Some(AiRole::KeywordIndex), Some(AiRole::Retriever)) => "seed hits",
                        (Some(r), _) => label_of(r),
                        _ => "",
                    };
                    add(Some(p.clone()), Some(t.clone()), label, false, edges);
                }
            }
            prev.clone_from(level);
        }
        prev
    };
    // The model that reads the prompt: the one in the prompt's file, else the first one.
    // A second model elsewhere (a served model, a judge) is wired by its own rules below.
    let prompt_files: Vec<&str> = stages
        .iter()
        .filter(|s| s.role == AiRole::Prompt)
        .map(|s| s.file_id.as_str())
        .collect();
    let mut model_ids: Vec<String> =
        ids(&|s| s.role == AiRole::Llm && prompt_files.contains(&s.file_id.as_str()));
    if model_ids.is_empty() {
        model_ids.extend(first(AiRole::Llm));
    }
    let ingest = vec![
        ids(&role_is(AiRole::Loader)),
        ids(&role_is(AiRole::Splitter)),
    ];
    let ingest_end = link_levels(&ingest, &mut edges);
    let dense = vec![
        ingest_end.clone(),
        ids(&role_is(AiRole::Embedder)),
        ids(&role_is(AiRole::VectorStore)),
        ids(&|s| s.role == AiRole::Retriever && !matches!(lane_of(s), Some("lexical" | "graph"))),
    ];
    let lexical = vec![
        ingest_end.clone(),
        ids(&role_is(AiRole::KeywordIndex)),
        ids(&|s| s.role == AiRole::Retriever && lane_of(s) == Some("lexical")),
    ];
    let tail = [
        ids(&role_is(AiRole::Fusion)),
        ids(&role_is(AiRole::Reranker)),
        ids(&is_check),
        ids(&role_is(AiRole::Prompt)),
        model_ids,
    ];
    let dense_end = link_levels(&dense, &mut edges);
    let lexical_end = if lexical[1..].iter().any(|l| !l.is_empty()) {
        link_levels(&lexical, &mut edges)
    } else {
        Vec::new()
    };
    let tail_start: Vec<String> = tail
        .iter()
        .find(|l| !l.is_empty())
        .cloned()
        .unwrap_or_default();
    for end in [&dense_end, &lexical_end] {
        if end == &ingest_end {
            continue;
        }
        for p in end {
            for t in &tail_start {
                let label = match role_of(p) {
                    Some(AiRole::KeywordIndex) => "keyword hits",
                    Some(AiRole::Retriever)
                        if lane_of(stages.iter().find(|s| &s.id == p).expect("stage"))
                            == Some("lexical") =>
                    {
                        "expanded hits"
                    }
                    Some(r) => label_of(r),
                    None => "",
                };
                add(Some(p.clone()), Some(t.clone()), label, false, &mut edges);
            }
        }
    }
    link_levels(&tail, &mut edges);
    // An evidence check that says "not enough" sends the question back to retrieval.
    let starts: Vec<String> = [&dense, &lexical]
        .iter()
        .filter_map(|lane| {
            lane[1..]
                .iter()
                .find(|l| !l.is_empty())
                .and_then(|l| l.first().cloned())
        })
        .collect();
    for c in ids(&is_check) {
        for st in &starts {
            add(
                Some(c.clone()),
                Some(st.clone()),
                "not enough: retrieve more",
                true,
                &mut edges,
            );
        }
    }
    for s in stages.iter().filter(|s| {
        matches!(
            s.role,
            AiRole::VectorStore | AiRole::KeywordIndex | AiRole::Retriever
        )
    }) {
        let label = match s.role {
            AiRole::VectorStore => "nearest-neighbour search",
            AiRole::KeywordIndex => "keyword hits",
            _ => "ranked hits",
        };
        // A search that already leads somewhere (graph expansion, say) keeps that path.
        if !edges.iter().any(|e| e.from == s.id) {
            add(
                Some(s.id.clone()),
                first(AiRole::Fusion),
                label,
                false,
                &mut edges,
            );
        }
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
    let decider = stages
        .iter()
        .find(|s| s.role == AiRole::Router && !s.params.contains_key("check"))
        .map(|s| s.id.clone())
        .or_else(|| first(AiRole::Agent));
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

    let has_check = stages
        .iter()
        .any(|s| s.role == AiRole::Router && s.params.contains_key("check"));
    let has_graph = stages.iter().any(|s| {
        s.role == AiRole::Retriever && s.params.get("lane").is_some_and(|l| l == "lexical")
    });
    let has_agent = present(AiRole::Agent)
        || stages
            .iter()
            .any(|s| s.role == AiRole::Router && !s.params.contains_key("check"))
        || present(AiRole::Tool);
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
    let mut headline = headline;
    if has_graph {
        headline.push_str("; keyword hits are expanded through an entity graph");
    }
    if has_check {
        headline.push_str("; an evidence check sends the question back for more retrieval when the passages are not enough");
    }
    edges.sort_by(|a, b| (&a.from, &a.to).cmp(&(&b.from, &b.to)));
    Some(AiArchitecture {
        kind,
        headline,
        frameworks: frameworks.into_iter().collect(),
        stages,
        edges,
    })
}

static EMBED_MODEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(BAAI/bge-[a-z0-9.\-]+|bge-(?:m3|large|base|small)[a-z0-9.\-]*|text-embedding-[a-z0-9\-]+|nomic-embed-text[a-z0-9.\-]*|all-MiniLM-[a-z0-9\-]+|multilingual-e5-[a-z]+|e5-(?:large|base|small)[a-z0-9\-]*|gte-[a-z0-9\-]+|jina-embeddings-[a-z0-9\-]+|mxbai-embed-[a-z0-9\-]+|snowflake-arctic-embed[a-z0-9\-]*)")
        .expect("embed model regex")
});
static RERANK_MODEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b((?:BAAI/)?bge-reranker-[a-z0-9\-]+|(?:Qwen/)?Qwen[0-9.]*-Reranker-[a-z0-9.\-]+|mxbai-rerank-[a-z0-9\-]+|jina-reranker-[a-z0-9\-]+|ms-marco-[a-z0-9\-]+|rerank-[a-z0-9\-.]+)")
        .expect("rerank model regex")
});
static KEYWORD_SQL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bbm25\b|\bts_rank(?:_cd)?\s*\(|\bto_tsvector\s*\(|\bplainto_tsquery|\bwebsearch_to_tsquery|\btsvector\b")
        .expect("keyword regex")
});

/// A pgvector nearest-neighbour query (`<=>` on a `::vector`) written inside a SQL string.
static VECTOR_QUERY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)<=>[^<>]{0,40}::\s*(?:half)?vec(?:tor)?\b|::\s*(?:half)?vec(?:tor)?\b[^<>]{0,40}<=>|\bembedding\s*<=>")
        .expect("vector query regex")
});
static VECTOR_INDEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bvector_(?:cosine|l2|ip)_ops\b").expect("vector index regex")
});
/// An embeddings endpoint called over HTTP, with no SDK for the parser to recognise.
static GRAPH_QUERY: LazyLock<Regex> = LazyLock::new(|| {
    // Cypher: MATCH (e:Entity) ...
    Regex::new(r"\bMATCH\s*\(\s*\w*\s*:\s*[A-Z]\w*").expect("graph query regex")
});
static AGENT_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:def|fn|func|function)\s+(run_agent\w*|\w*agent_loop\w*|\w*react_loop\w*)")
        .expect("agent loop regex")
});
static FUSION_NAMED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\brrf_k\b|reciprocal[ _-]?rank").expect("fusion setting regex")
});
/// `@register("search_text", "What it does", {...})`: a home-made tool registry.
static REGISTERED_TOOL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^@?register(?:_tool)?\(\s*["']([A-Za-z_]\w*)["']\s*,\s*["']"#)
        .expect("tool registry regex")
});
static EMBED_HTTP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"/v1/embeddings\b|["'`]/embeddings\b"#).expect("embed endpoint regex")
});
/// A hand-written Reciprocal Rank Fusion function.
static FUSION_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:def|fn|func|function)\s+(\w*(?:rrf|reciprocal_?rank)\w*)")
        .expect("fusion regex")
});

/// A chat / instruct model id written in code or config (`Qwen3-27B`, `llama-3.1-8b`).
static LLM_MODEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b((?:Qwen/)?Qwen[0-9.]*-[0-9.]+B[a-z0-9.\-_]*|(?:meta-llama/)?Llama-?3[a-z0-9.\-]*-[0-9]+B[a-z0-9.\-]*|gemma-?[0-9][a-z0-9.\-]*-[0-9]+b[a-z0-9.\-]*|Mistral-[0-9]+B[a-z0-9.\-]*|Phi-?[34][a-z0-9.\-]*|gpt-4[a-z0-9.\-]*|gpt-5[a-z0-9.\-]*|claude-[a-z0-9.\-]+)")
        .expect("llm model regex")
});
/// A chat or completion endpoint called over HTTP (llama.cpp, vLLM, Ollama, OpenAI-style).
static CHAT_HTTP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"\}/chat/completions\b|/v1/chat/completions\b|/v1/completions\b|/api/chat\b|/api/generate\b|["'`/]completion["'`]"#)
        .expect("chat endpoint regex")
});
/// Retrieval that walks an entity graph: a hand-written expansion function, or SQL over
/// entity / relation / mention tables.
static GRAPH_EXPAND: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:def|fn|func|function)\s+(\w*(?:graph|entit|relation|neighbo)\w*(?:expand|walk|hop|travers|neighbo|search|lookup)\w*|\w*(?:expand|travers)\w*(?:graph|entit|relation|neighbo)\w*)|\b(?:FROM|JOIN)\s+(?:\w+\.)?(?:entity_)?(?:relations|mentions|entity_aliases|aliases)\b")
        .expect("graph expand regex")
});
/// A check that decides whether the retrieved evidence is enough, or retrieval must run again.
static EVIDENCE_CHECK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:def|fn|func|function)\s+(\w*(?:sufficien|enough|need_more|needs_more|retrieve_more|should_retry|answerable|is_complete)\w*)")
        .expect("evidence check regex")
});
/// Document loading by hand: PDF readers and OCR.
static DOC_LOADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:from|import)\s+(pypdf|PyPDF2|pdfplumber|fitz|pymupdf|pdfminer|pytesseract|unstructured|docx)\b")
        .expect("loader regex")
});
/// A hand-written chunker and its size constant.
static CHUNK_DEF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:def|fn|func|function)\s+((?:chunk|split_)\w*|\w*_(?:chunk|chunks|chunker|chunking)\b)")
        .expect("chunk regex")
});
static CHUNK_SIZE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:chunk_?(?:size|chars|len(?:gth)?)|max_chars)\s*[:=]\s*(?:int\s*=\s*)?(\d{2,5})\b")
        .expect("chunk size regex")
});

/// Hand-written pipeline steps found in any source file.
#[derive(Default)]
struct Steps<'a> {
    chat_http: Option<(u8, &'a str, u32)>,
    graph: Option<(String, &'a str, u32)>,
    check: Option<(String, &'a str, u32)>,
    loader: Option<(String, &'a str, u32)>,
    chunker: Option<(String, &'a str, u32)>,
    chunk_size: Option<String>,
}

fn scan_steps<'a>(line: &str, path: &'a str, ln: u32, files: &[ParsedFile], st: &mut Steps<'a>) {
    if CHAT_HTTP.is_match(line) && !is_side_code(path) {
        keep_best(&mut st.chat_http, files, path, ln);
    }
    // The function that expands outranks the SQL it runs.
    let named_graph = st
        .graph
        .as_ref()
        .is_some_and(|g| g.0 != "entity graph lookup");
    if !named_graph && let Some(c) = GRAPH_EXPAND.captures(line) {
        match c.get(1) {
            Some(m) => st.graph = Some((m.as_str().to_owned(), path, ln)),
            None if st.graph.is_none() => {
                st.graph = Some(("entity graph lookup".to_owned(), path, ln));
            }
            None => {}
        }
    }
    if st.check.is_none()
        && let Some(c) = EVIDENCE_CHECK.captures(line)
    {
        st.check = Some((c[1].to_owned(), path, ln));
    }
    if st.loader.is_none()
        && let Some(c) = DOC_LOADER.captures(line)
    {
        st.loader = Some((c[1].to_owned(), path, ln));
    }
    if st.chunker.is_none()
        && let Some(c) = CHUNK_DEF.captures(line)
    {
        st.chunker = Some((c[1].to_owned(), path, ln));
    }
    if st.chunk_size.is_none()
        && let Some(c) = CHUNK_SIZE.captures(line)
    {
        st.chunk_size = Some(c[1].to_owned());
    }
}

/// File names that usually hold model ids, connection settings and search SQL.
fn holds_settings(path: &str) -> bool {
    let l = path.to_ascii_lowercase();
    let name = l.rsplit('/').next().unwrap_or(&l);
    name.starts_with(".env")
        || [
            "config", "settings", "store", "search", "embed", "retriev", "vector", "fusion",
            "rank", "rag", "llm", "model", "client",
        ]
        .iter()
        .any(|k| name.contains(k))
        || [".yml", ".yaml", ".toml", ".sql", ".env"]
            .iter()
            .any(|e| name.ends_with(e))
}

/// A line that defines a search pattern (a regex) rather than using what it names.
fn is_pattern_source(line: &str) -> bool {
    ["Regex::new(", "re.compile(", "new RegExp(", "match: ["]
        .iter()
        .any(|k| line.contains(k))
}

fn is_comment(line: &str) -> bool {
    let l = line.trim_start();
    ["#", "//", "--", "/*", "* ", "<!--"]
        .iter()
        .any(|p| l.starts_with(p))
}

/// One search of its own: the function that runs the query, keyed by its id.
type Legs<'a> = BTreeMap<String, (&'a str, u32)>;

/// Records the hit when it sits in a function that searches. Indexing and chat-memory
/// functions run similar queries but are not a way of answering a question.
fn add_leg<'a>(legs: &mut Legs<'a>, files: &[ParsedFile], path: &'a str, line: u32) {
    if is_side_code(path) {
        return;
    }
    let Some(f) = function_at(files, path, line) else {
        return;
    };
    let name = f.rsplit('#').next().unwrap_or_default();
    let name = name.rsplit('.').next().unwrap_or(name).to_ascii_lowercase();
    let searches = ["search", "retriev", "nearest", "similar", "lookup", "find"]
        .iter()
        .any(|w| name.contains(w));
    if searches && !name.contains("memor") {
        legs.entry(f).or_insert((path, line));
    }
}

/// Scripts and browser code: used only when the application itself has no such step.
fn is_side_code(path: &str) -> bool {
    is_web_source(path) || path.split('/').any(|p| p.eq_ignore_ascii_case("scripts"))
}

/// Adds one stage per leg. With several legs each is named after its function.
fn put_legs(
    stages: &mut BTreeMap<(AiRole, String), AiStage>,
    frameworks: &mut BTreeSet<String>,
    role: AiRole,
    label: &str,
    legs: &Legs,
    explain: &str,
) {
    // A search library the parser already recognised describes this role better.
    if stages
        .values()
        .any(|s| s.role == role && s.rule_id != "ai.mention")
    {
        return;
    }
    for (function, (path, line)) in legs {
        let fn_name = function.rsplit(['#', '.']).next().unwrap_or(function);
        let name = if legs.len() == 1 {
            label.to_owned()
        } else {
            format!("{label}: {fn_name}")
        };
        frameworks.insert(label.to_owned());
        stages.insert(
            (role, name.clone()),
            AiStage {
                id: format!("ai:{}:{name}", role_slug(role)),
                role,
                name,
                library: label.to_owned(),
                file_id: file_id(path),
                function_id: Some(function.clone()),
                line: *line,
                params: BTreeMap::new(),
                rule_id: "ai.mention".to_owned(),
                explain: explain.to_owned(),
            },
        );
    }
}

/// Keeps the most telling hit: one inside a function beats a docstring or schema line,
/// and a function that works on the corpus beats a side use such as chat memory.
fn keep_best<'a>(
    slot: &mut Option<(u8, &'a str, u32)>,
    files: &[ParsedFile],
    path: &'a str,
    line: u32,
) {
    let rank = match function_at(files, path, line) {
        None => 0,
        Some(f) => {
            let name = f
                .rsplit('#')
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if name.contains("memory") {
                1
            } else if name
                .rsplit('.')
                .next()
                .is_some_and(|n| n.starts_with("search"))
            {
                4
            } else if ["chunk", "corpus", "doc"].iter().any(|w| name.contains(w)) {
                3
            } else {
                2
            }
        }
    };
    if slot.is_none_or(|(r, _, _)| rank > r) {
        *slot = Some((rank, path, line));
    }
}

fn is_web_source(path: &str) -> bool {
    [".js", ".jsx", ".ts", ".tsx", ".vue", ".svelte"]
        .iter()
        .any(|e| path.ends_with(e))
}

/// Throwaway code: what it loads is not part of the running system.
fn is_experiment(path: &str) -> bool {
    path.split('/').any(|p| {
        ["experiments", "benchmarks", "bench"]
            .iter()
            .any(|d| p.eq_ignore_ascii_case(d))
    })
}

/// The innermost parsed function around a line, so a text hit still links to code.
fn function_at(files: &[ParsedFile], path: &str, line: u32) -> Option<String> {
    let pf = files.iter().find(|p| p.scan.path == path)?;
    pf.facts
        .functions
        .iter()
        .filter(|f| f.line_start <= line && line <= f.line_end)
        .min_by_key(|f| f.line_end - f.line_start)
        .map(|f| fn_id(path, &f.qualname))
}

/// Model ids and SQL full-text calls that sit in config files, env files, compose
/// commands and SQL strings, where no call is made that the parser could see.
fn add_mentions(
    stages: &mut BTreeMap<(AiRole, String), AiStage>,
    frameworks: &mut BTreeSet<String>,
    texts: &[InfraFile],
    files: &[ParsedFile],
) {
    // (path, line) of the code that does each step, when it is written by hand.
    let mut vector_legs = Legs::new();
    let mut word_legs = Legs::new();
    let mut graph_legs = Legs::new();
    let mut agent: Option<(String, &str, u32)> = None;
    let mut fusion_setting: Option<(&str, u32)> = None;
    let mut vector_query: Option<(u8, &str, u32)> = None;
    let mut vector_index: Option<(&str, u32)> = None;
    let mut embed_http: Option<(u8, &str, u32)> = None;
    let mut fusion: Option<(String, &str, u32)> = None;
    let mut embed: BTreeMap<String, (u32, &str, u32)> = BTreeMap::new();
    let mut rerank: BTreeMap<String, (u32, &str, u32)> = BTreeMap::new();
    let mut keyword: Option<(bool, &str, u32)> = None;
    let mut keyword_sql: Option<(u8, &str, u32)> = None;
    let mut llm: BTreeMap<String, (u32, &str, u32)> = BTreeMap::new();
    let mut steps = Steps::default();
    for t in texts {
        let web = is_web_source(t.path);
        for (i, line) in t.text.lines().enumerate() {
            let ln = (i + 1) as u32;
            // A model or method named only in a comment is not something the code does.
            if is_comment(line) || is_pattern_source(line) {
                continue;
            }
            // Searches and the agent loop are recognised by the function they sit in, so
            // any source file counts.
            if !web {
                if VECTOR_QUERY.is_match(line) {
                    add_leg(&mut vector_legs, files, t.path, ln);
                }
                if GRAPH_QUERY.is_match(line) {
                    add_leg(&mut graph_legs, files, t.path, ln);
                }
                if KEYWORD_SQL
                    .find(line)
                    .is_some_and(|m| !m.as_str().eq_ignore_ascii_case("bm25"))
                {
                    add_leg(&mut word_legs, files, t.path, ln);
                }
                if agent.is_none()
                    && !is_side_code(t.path)
                    && let Some(c) = AGENT_DEF.captures(line)
                {
                    agent = Some((c[1].to_owned(), t.path, ln));
                }
            }
            // Model ids and SQL count in files whose name says they hold them; the
            // hand-written pipeline steps below count in any source file.
            if !holds_settings(t.path) {
                if !web {
                    scan_steps(line, t.path, ln, files, &mut steps);
                }
                continue;
            }
            if VECTOR_QUERY.is_match(line) {
                keep_best(&mut vector_query, files, t.path, ln);
            }
            if fusion_setting.is_none() && FUSION_NAMED.is_match(line) {
                fusion_setting = Some((t.path, ln));
            }
            if vector_index.is_none() && VECTOR_INDEX.is_match(line) {
                vector_index = Some((t.path, ln));
            }
            if EMBED_HTTP.is_match(line) {
                keep_best(&mut embed_http, files, t.path, ln);
            }
            if fusion.is_none()
                && let Some(c) = FUSION_DEF.captures(line)
            {
                fusion = Some((c[1].to_owned(), t.path, ln));
            }
            if !web {
                scan_steps(line, t.path, ln, files, &mut steps);
            }
            for m in LLM_MODEL.find_iter(line) {
                let l = m.as_str().to_ascii_lowercase();
                if l.contains("rerank") || l.contains("embed") {
                    continue;
                }
                let e = llm.entry(m.as_str().to_owned()).or_insert((0, t.path, ln));
                e.0 += 1;
            }
            for m in RERANK_MODEL.find_iter(line) {
                let e = rerank
                    .entry(m.as_str().to_owned())
                    .or_insert((0, t.path, ln));
                e.0 += 1;
            }
            for m in EMBED_MODEL.find_iter(line) {
                if m.as_str().to_ascii_lowercase().contains("rerank") {
                    continue;
                }
                let e = embed
                    .entry(m.as_str().to_owned())
                    .or_insert((0, t.path, ln));
                e.0 += 1;
            }
            if let Some(m) = KEYWORD_SQL.find(line) {
                let bm = m.as_str().eq_ignore_ascii_case("bm25");
                // A real full-text call outranks the word "BM25" in a docstring.
                if bm {
                    keyword.get_or_insert((true, t.path, ln));
                } else {
                    keep_best(&mut keyword_sql, files, t.path, ln);
                }
            }
        }
    }
    let best = |m: &BTreeMap<String, (u32, &str, u32)>| {
        m.iter()
            .max_by_key(|(_, v)| v.0)
            .map(|(k, v)| (k.clone(), v.1.to_owned(), v.2))
    };
    if let Some((model, path, line)) = best(&embed) {
        // Point at the code that calls the embeddings server rather than a config line.
        let (path, line) = embed_http.map_or((path, line), |(_, p, l)| (p.to_owned(), l));
        let explain = if model.to_ascii_lowercase().contains("bge-m3") {
            "BGE-M3 embedding model. The model can produce dense, sparse and multi-vector outputs; which of them this app uses depends on what its code stores and searches."
        } else {
            role_explain(AiRole::Embedder)
        };
        let at = (path.as_str(), line);
        put_mention(
            stages,
            frameworks,
            (AiRole::Embedder, model, "embedding model"),
            at,
            explain,
        );
    }
    if let Some((model, path, line)) = best(&rerank) {
        let at = (path.as_str(), line);
        let explain = role_explain(AiRole::Reranker);
        put_mention(
            stages,
            frameworks,
            (AiRole::Reranker, model, "reranker model"),
            at,
            explain,
        );
    }
    if embed.is_empty()
        && let Some((_, p, l)) = embed_http
    {
        put_mention(
            stages,
            frameworks,
            (
                AiRole::Embedder,
                "embeddings server".to_owned(),
                "HTTP embeddings",
            ),
            (p, l),
            role_explain(AiRole::Embedder),
        );
    }
    put_legs(
        stages,
        frameworks,
        AiRole::VectorStore,
        "pgvector",
        &vector_legs,
        role_explain(AiRole::VectorStore),
    );
    // pgvector written as plain SQL; an index definition alone is the fallback location
    if let Some(at) = vector_query.map(|(_, p, l)| (p, l)).or(vector_index) {
        put_mention(
            stages,
            frameworks,
            (AiRole::VectorStore, "pgvector".to_owned(), "pgvector"),
            at,
            role_explain(AiRole::VectorStore),
        );
    }
    if !stages.is_empty()
        && let Some((name, path, line)) = fusion
    {
        put_mention(
            stages,
            frameworks,
            (AiRole::Fusion, name, "Reciprocal Rank Fusion"),
            (path, line),
            role_explain(AiRole::Fusion),
        );
    }
    if let Some((_, p, l)) = keyword_sql {
        keyword = Some((false, p, l));
    }
    if !stages.is_empty() {
        put_legs(
            stages,
            frameworks,
            AiRole::KeywordIndex,
            "Postgres full-text",
            &word_legs,
            FULL_TEXT,
        );
        put_legs(
            stages,
            frameworks,
            AiRole::Retriever,
            "Knowledge graph",
            &graph_legs,
            "Graph search. Follows stored links between entities and pages, so it can reach evidence that shares no words with the question.",
        );
        if let Some((name, p, l)) = agent
            && !stages.values().any(|s| s.role == AiRole::Agent)
        {
            put_leg_free(
                stages,
                frameworks,
                AiRole::Agent,
                &name,
                "agent loop",
                (p, l),
            );
        }
        if let Some(at) = fusion_setting
            && !stages.values().any(|s| s.role == AiRole::Fusion)
        {
            put_leg_free(
                stages,
                frameworks,
                AiRole::Fusion,
                "RRF",
                "Reciprocal Rank Fusion",
                at,
            );
        }
    }
    const FULL_TEXT: &str = "Postgres full-text search. Matches the question's words with a text index and orders hits by ts_rank (not BM25). Catches exact names and ids that embeddings blur.";
    // full-text SQL on its own is ordinary search; it only counts next to other AI stages
    if !stages.is_empty()
        && let Some((bm, path, line)) = keyword
    {
        let (name, lib) = if bm {
            ("BM25", "BM25")
        } else {
            ("Postgres full-text", "Postgres full-text")
        };
        let explain = if bm {
            role_explain(AiRole::KeywordIndex)
        } else {
            FULL_TEXT
        };
        put_mention(
            stages,
            frameworks,
            (AiRole::KeywordIndex, name.to_owned(), lib),
            (path, line),
            explain,
        );
    }
    // The rest only count next to other AI stages: on their own they are ordinary code.
    let Steps {
        chat_http,
        graph,
        check,
        loader,
        chunker,
        chunk_size,
    } = steps;
    if !stages.is_empty() {
        if let Some((name, path, line)) = graph {
            put_mention(
                stages,
                frameworks,
                (AiRole::Retriever, name, "entity graph"),
                (path, line),
                "Graph-assisted retrieval. Starts from the keyword hits, follows linked entities and relations in the graph, and adds the passages they point to.",
            );
            if let Some(s) = stages
                .values_mut()
                .find(|s| s.role == AiRole::Retriever && s.library == "entity graph")
            {
                s.params.insert("lane".to_owned(), "lexical".to_owned());
            }
        }
        if let Some((name, path, line)) = check {
            put_mention(
                stages,
                frameworks,
                (AiRole::Router, name, "evidence check"),
                (path, line),
                "Evidence check. Decides whether the retrieved passages are enough to answer. If not, retrieval runs again with a wider search; if yes, the passages go on to the answer.",
            );
            if let Some(s) = stages
                .values_mut()
                .find(|s| s.role == AiRole::Router && s.library == "evidence check")
            {
                s.params.insert("check".to_owned(), "evidence".to_owned());
            }
        }
        if let Some((name, path, line)) = loader {
            put_mention(
                stages,
                frameworks,
                (AiRole::Loader, name, "document loader"),
                (path, line),
                role_explain(AiRole::Loader),
            );
        }
        if let Some((name, path, line)) = chunker {
            put_mention(
                stages,
                frameworks,
                (AiRole::Splitter, name, "hand-written chunker"),
                (path, line),
                role_explain(AiRole::Splitter),
            );
        }
        if let Some(size) = chunk_size
            && let Some(s) = stages.values_mut().find(|s| s.role == AiRole::Splitter)
        {
            s.params.entry("chunk_size".to_owned()).or_insert(size);
        }
        let own: BTreeMap<String, (u32, &str, u32)> = llm
            .iter()
            .filter(|(_, v)| !is_side_code(v.1))
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        let model = best(if own.is_empty() { &llm } else { &own });
        let at = chat_http
            .map(|(_, p, l)| (p.to_owned(), l))
            .or_else(|| model.as_ref().map(|(_, p, l)| (p.clone(), *l)));
        if let Some((path, line)) = at {
            let name = model
                .as_ref()
                .map_or_else(|| "chat model server".to_owned(), |(m, _, _)| m.clone());
            put_mention(
                stages,
                frameworks,
                (AiRole::Llm, name, "language model"),
                (path.as_str(), line),
                role_explain(AiRole::Llm),
            );
        }
    }
    // Model calls from scripts or the browser give way to the application's own.
    let own = |s: &AiStage| !is_side_code(s.file_id.trim_start_matches("file:"));
    if stages.values().any(|s| s.role == AiRole::Llm && own(s)) {
        stages.retain(|_, s| s.role != AiRole::Llm || own(s));
    }
    for s in stages
        .values_mut()
        .filter(|s| s.role == AiRole::Retriever && s.library == "Knowledge graph")
    {
        // Its own lane into fusion: it is not a step after the vector search.
        s.params.insert("lane".to_owned(), "graph".to_owned());
    }
    for s in stages.values_mut().filter(|s| s.rule_id == "ai.mention") {
        let path = s.file_id.strip_prefix("file:").unwrap_or(&s.file_id);
        s.function_id = function_at(files, path, s.line);
    }
}

/// A stage found in text for a role the code has no stage of yet.
fn put_leg_free(
    stages: &mut BTreeMap<(AiRole, String), AiStage>,
    frameworks: &mut BTreeSet<String>,
    role: AiRole,
    name: &str,
    library: &str,
    (path, line): (&str, u32),
) {
    frameworks.insert(library.to_owned());
    stages.insert(
        (role, name.to_owned()),
        AiStage {
            id: format!("ai:{}:{name}", role_slug(role)),
            role,
            name: name.to_owned(),
            library: library.to_owned(),
            file_id: file_id(path),
            function_id: None,
            line,
            params: BTreeMap::new(),
            rule_id: "ai.mention".to_owned(),
            explain: role_explain(role).to_owned(),
        },
    );
}

/// Adds a stage found in text, or only names the model on a stage of that role that the
/// code already produced.
fn put_mention(
    stages: &mut BTreeMap<(AiRole, String), AiStage>,
    frameworks: &mut BTreeSet<String>,
    (role, name, library): (AiRole, String, &str),
    (path, line): (&str, u32),
    explain: &str,
) {
    // A second retriever or check is a separate step, not a duplicate.
    let distinct = matches!(role, AiRole::Retriever | AiRole::Router)
        && !stages
            .values()
            .any(|s| s.role == role && s.library == library);
    if !distinct && let Some(existing) = stages.values_mut().find(|s| s.role == role) {
        let named =
            existing.params.contains_key("model") || existing.params.contains_key("model_name");
        if !named
            && (matches!(role, AiRole::Embedder | AiRole::Reranker)
                || role == AiRole::Llm && name != "chat model server")
        {
            existing.params.insert("model".to_owned(), name);
        }
        return;
    }
    frameworks.insert(library.to_owned());
    let mut params = BTreeMap::new();
    let names_model = matches!(role, AiRole::Embedder | AiRole::Reranker)
        || role == AiRole::Llm && name != "chat model server";
    if names_model {
        params.insert("model".to_owned(), name.clone());
    }
    stages.insert(
        (role, name.clone()),
        AiStage {
            id: format!("ai:{}:{name}", role_slug(role)),
            role,
            name,
            library: library.to_owned(),
            file_id: file_id(path),
            function_id: None,
            line,
            params,
            rule_id: "ai.mention".to_owned(),
            explain: explain.to_owned(),
        },
    );
}
