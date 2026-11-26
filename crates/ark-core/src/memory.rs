//! Memory map: where each kind of runtime data lives (GPU memory, RAM, disk, the
//! browser, a hosted provider), who holds it, how long it lives and what bounds it.
//!
//! Built only from facts already in the graph (AI stages, memory techniques, compose
//! services, volumes, limits and commands) plus browser-storage calls in the parsed
//! code. Every placement lists the lines it rests on. When a fact is missing the entry
//! is left out rather than guessed: an embedder without a device setting is placed in
//! RAM only when its service has no GPU.

use std::collections::BTreeMap;

use ark_link::{ParsedFile, file_id};
use ark_schema::{
    AiRole, AiStage, MemoryLifetime as L, MemoryPlace as P, MemoryPlacement, MemoryTechnique,
    Service, ServiceKind, SystemArchitectureGraph,
};

struct Holder {
    id: String,
    name: String,
    gpu: bool,
}

struct Ctx<'a> {
    g: &'a SystemArchitectureGraph,
    services: &'a [Service],
    tier_of: BTreeMap<String, String>,
    out: Vec<MemoryPlacement>,
}

fn path_of(id: &str) -> &str {
    id.strip_prefix("file:").unwrap_or(id)
}

fn fmt_bytes(b: u64) -> String {
    const G: u64 = 1 << 30;
    const M: u64 = 1 << 20;
    if b >= G && b.is_multiple_of(G / 4) {
        format!("{} GiB", (b as f64 / G as f64 * 100.0).round() / 100.0)
    } else {
        format!("{} MiB", b / M)
    }
}

impl<'a> Ctx<'a> {
    /// Deepest build context holding the file, or its code tier when there is no compose.
    fn holder_of(&self, file: &str) -> Holder {
        let path = path_of(file);
        let mut best: Option<&Service> = None;
        for s in self.services {
            let Some(c) = &s.build_context else { continue };
            if (c.is_empty() || path.starts_with(&format!("{c}/")))
                && best.is_none_or(|b| b.build_context.as_deref().unwrap_or("").len() < c.len())
            {
                best = Some(s);
            }
        }
        if let Some(s) = best {
            return Holder {
                id: s.id.clone(),
                name: s.name.clone(),
                gpu: s.gpu,
            };
        }
        let tier = self
            .tier_of
            .get(path)
            .cloned()
            .unwrap_or_else(|| "backend".into());
        Holder {
            id: format!("tier:{tier}"),
            name: format!("{tier} process"),
            gpu: false,
        }
    }

    fn svc(&self, s: &Service) -> Holder {
        Holder {
            id: s.id.clone(),
            name: s.name.clone(),
            gpu: s.gpu,
        }
    }

    /// The only service of a kind, or the one whose image names the engine.
    fn service(&self, kind: ServiceKind, engine: &str) -> Option<&'a Service> {
        let e = engine.to_ascii_lowercase();
        let by_image = self.services.iter().find(|s| {
            !e.is_empty()
                && s.image
                    .as_deref()
                    .is_some_and(|i| i.to_ascii_lowercase().contains(&e))
        });
        if by_image.is_some() {
            return by_image;
        }
        let of_kind: Vec<_> = self.services.iter().filter(|s| s.kind == kind).collect();
        (of_kind.len() == 1).then(|| of_kind[0])
    }

    #[allow(clippy::too_many_arguments)]
    fn add(
        &mut self,
        source: &str,
        what: impl Into<String>,
        place: P,
        h: &Holder,
        lifetime: L,
        bound: Option<String>,
        proof: Vec<String>,
        explain: impl Into<String>,
    ) {
        let what = what.into();
        // Same thing in the same holder: merge the evidence instead of listing it twice.
        // GPU memory wins over RAM (a KV cache setting does not move it off the GPU), and
        // "Model weights (qwen3:4b)" absorbs a bare "Model weights".
        let same = |a: &str, b: &str| {
            a == b
                || (a == "Model weights" && b.starts_with("Model weights"))
                || (b == "Model weights" && a.starts_with("Model weights"))
        };
        let gpu_ram =
            |a: P, b: P| a == b || matches!((a, b), (P::Vram, P::Ram) | (P::Ram, P::Vram));
        if let Some(p) = self
            .out
            .iter_mut()
            .find(|p| same(&p.what, &what) && p.holder == h.id && gpu_ram(p.place, place))
        {
            if place == P::Vram {
                p.place = P::Vram;
            }
            if what.len() > p.what.len() {
                p.what = what;
            }
            for x in proof {
                if !p.proof.contains(&x) {
                    p.proof.push(x);
                }
            }
            if let Some(b) = bound {
                p.bound = Some(match p.bound.take() {
                    Some(old) if !old.contains(&b) => format!("{old}; {b}"),
                    Some(old) => old,
                    None => b,
                });
            }
            return;
        }
        self.out.push(MemoryPlacement {
            id: String::new(),
            what,
            place,
            holder: h.id.clone(),
            holder_name: h.name.clone(),
            lifetime,
            bound,
            proof,
            explain: explain.into(),
            source: source.to_owned(),
        });
    }
}

fn at(file: &str, line: u32) -> String {
    format!("{}:{line}", path_of(file))
}

fn param(st: &AiStage, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| st.params.get(*k))
        .map(|v| v.trim_matches(['"', '\'']).to_owned())
}

fn on_gpu(st: &AiStage, h: &Holder) -> bool {
    let dev = param(st, &["device", "model_kwargs"]).unwrap_or_default();
    dev.contains("cuda") || dev.contains("gpu") || (h.gpu && !dev.contains("cpu"))
}

fn remote() -> Holder {
    Holder {
        id: "remote".into(),
        name: "hosted provider".into(),
        gpu: false,
    }
}

fn redis_bound(s: &Service) -> Option<String> {
    let cmd = s.command.as_deref().unwrap_or("");
    let toks: Vec<&str> = cmd.split_whitespace().collect();
    let flag = |f: &str| {
        toks.iter()
            .position(|t| *t == f)
            .and_then(|i| toks.get(i + 1))
            .map(|v| v.to_string())
    };
    let mut parts = Vec::new();
    if let Some(m) = flag("--maxmemory") {
        parts.push(format!("maxmemory {m}"));
        parts.push(flag("--maxmemory-policy").unwrap_or_else(|| "noeviction".into()));
    }
    if let Some(l) = s.memory_limit {
        parts.push(format!("container limit {}", fmt_bytes(l)));
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn limit(s: &Service) -> Option<String> {
    s.memory_limit
        .map(|l| format!("container limit {}", fmt_bytes(l)))
}

const HOSTED_STORES: &[&str] = &["Pinecone", "SupabaseVectorStore", "MongoDBAtlas"];
const REMOTE_MODELS: &[&str] = &[
    "ChatOpenAI",
    "OpenAI",
    "AzureChatOpenAI",
    "ChatAnthropic",
    "Anthropic",
    "ChatGoogleGenerativeAI",
    "ChatVertexAI",
    "ChatMistralAI",
    "ChatGroq",
    "ChatCohere",
    "ChatBedrock",
    "OpenAIEmbeddings",
    "CohereEmbeddings",
    "VoyageAIEmbeddings",
    "GoogleGenerativeAIEmbeddings",
    "AzureOpenAIEmbeddings",
    "CohereRerank",
];

fn stage(c: &mut Ctx, st: &AiStage) {
    let src = st.function_id.clone().unwrap_or_else(|| st.file_id.clone());
    let proof = vec![at(&st.file_id, st.line)];
    let h = c.holder_of(&src);
    let base = st.name.split('.').next().unwrap_or(&st.name);
    if REMOTE_MODELS.contains(&base) {
        let what = match st.role {
            AiRole::Embedder => "Embedding model (at the provider)",
            AiRole::Reranker => "Reranker model (at the provider)",
            _ => "Model weights and KV cache (at the provider)",
        };
        c.add(
            &st.id,
            what,
            P::Remote,
            &remote(),
            L::Request,
            None,
            proof,
            format!(
                "{} sends the text over HTTPS; the model runs on the provider's GPUs and nothing stays in this project's memory after the reply.",
                st.name
            ),
        );
        return;
    }
    match st.role {
        AiRole::VectorStore => {
            if HOSTED_STORES.iter().any(|s| st.name.contains(s)) {
                c.add(
                    &st.id,
                    "Vectors and chunks (hosted index)",
                    P::Remote,
                    &remote(),
                    L::Persistent,
                    None,
                    proof,
                    "A hosted vector database keeps the index on the provider's machines; this code only sends queries.",
                );
                return;
            }
            let engine = base
                .trim_end_matches("VectorStore")
                .trim_end_matches("VectorSearch");
            if let Some(dir) = param(st, &["persist_directory", "path", "folder_path"]) {
                c.add(
                    &st.id,
                    format!("Vector index files (persist_directory={dir})"),
                    P::Disk,
                    &h,
                    L::Persistent,
                    None,
                    proof.clone(),
                    format!("{} writes the collection to {dir} inside {}, so it survives restarts (if that folder is on a volume).", st.name, h.name),
                );
            }
            if let Some(s) = c.service(ServiceKind::VectorStore, engine) {
                let sh = c.svc(s);
                let mut pr = proof;
                pr.push(s.declared_in.clone());
                c.add(
                    &s.id,
                    "Vector index (HNSW graph + vectors)",
                    P::Ram,
                    &sh,
                    L::Process,
                    limit(s),
                    pr,
                    format!("{} keeps the search graph and the vectors it walks in RAM so a query touches a few hundred vectors in milliseconds.", s.name),
                );
            } else {
                c.add(
                    &st.id,
                    "Vector index (in this process)",
                    P::Ram,
                    &h,
                    L::Process,
                    None,
                    proof,
                    format!("{} builds the index inside {}'s own memory; it is gone when the process stops unless it is saved.", st.name, h.name),
                );
            }
        }
        AiRole::KeywordIndex if st.library.to_ascii_lowercase().contains("bm25") => c.add(
            &st.id,
            "BM25 keyword index (word -> chunks)",
            P::Ram,
            &h,
            L::Process,
            param(st, &["k"]).map(|k| format!("returns top {k}")),
            proof,
            format!("{} holds the word counts for every chunk in {}'s RAM; it is rebuilt when the process restarts.", st.name, h.name),
        ),
        AiRole::Embedder | AiRole::Reranker => {
            let model = param(st, &["model_name", "model"]).unwrap_or_default();
            if base.starts_with("Ollama") {
                if let Some(s) = c.service(ServiceKind::ModelServer, "ollama") {
                    let sh = c.svc(s);
                    let place = if s.gpu { P::Vram } else { P::Ram };
                    let mut pr = proof;
                    pr.push(s.declared_in.clone());
                    c.add(&s.id, "Model weights", place, &sh, L::Process, limit(s), pr,
                        "Ollama loads each model it serves into memory once and shares it between requests.");
                }
                return;
            }
            let place = if on_gpu(st, &h) { P::Vram } else { P::Ram };
            let kind = if st.role == AiRole::Embedder {
                "Embedding model weights"
            } else {
                "Reranker model weights"
            };
            let what = if model.is_empty() {
                kind.to_owned()
            } else {
                format!("{kind} ({model})")
            };
            c.add(
                &st.id,
                what,
                place,
                &h,
                L::Process,
                None,
                proof,
                format!(
                    "{} loads the model into {} when it is created and keeps it there for the life of the process.",
                    st.name,
                    if place == P::Vram { "GPU memory" } else { "RAM" }
                ),
            );
        }
        AiRole::Llm => {
            if base.starts_with("ChatOllama") || base == "Ollama" || base == "OllamaLLM" {
                if let Some(s) = c.service(ServiceKind::ModelServer, "ollama") {
                    let sh = c.svc(s);
                    let place = if s.gpu { P::Vram } else { P::Ram };
                    let mut pr = proof;
                    pr.push(s.declared_in.clone());
                    let model = param(st, &["model"]).unwrap_or_default();
                    c.add(
                        &s.id,
                        if model.is_empty() { "Model weights".to_owned() } else { format!("Model weights ({model})") },
                        place,
                        &sh,
                        L::Process,
                        limit(s),
                        pr.clone(),
                        format!("The chat model is loaded into {} by Ollama, not into the API process; the API only sends HTTP requests.", if s.gpu { "GPU memory" } else { "RAM" }),
                    );
                    c.add(
                        &s.id,
                        "KV cache (attention keys and values)",
                        place,
                        &sh,
                        L::Request,
                        param(st, &["num_ctx"]).map(|n| format!("num_ctx {n} tokens")),
                        pr,
                        "While a reply is generated the model keeps a key and a value per token per layer; the context size decides how big this gets.",
                    );
                }
                return;
            }
            let local = matches!(
                base,
                "LLM" | "Llama" | "AutoModelForCausalLM" | "AutoModelForSeq2SeqLM" | "pipeline"
            );
            if local {
                let vllm = base == "LLM";
                let place = if vllm || on_gpu(st, &h) { P::Vram } else { P::Ram };
                let model = param(st, &["model", "model_path", "pretrained_model_name_or_path"])
                    .unwrap_or_default();
                let bound = param(st, &["gpu_memory_utilization"])
                    .map(|u| format!("gpu_memory_utilization {u}"));
                c.add(
                    &st.id,
                    if model.is_empty() { "Model weights".to_owned() } else { format!("Model weights ({model})") },
                    place,
                    &h,
                    L::Process,
                    bound,
                    proof,
                    format!("{} loads the whole model into {} of {} and keeps it there.", st.name, if place == P::Vram { "GPU memory" } else { "RAM" }, h.name),
                );
            }
        }
        AiRole::Memory | AiRole::Cache => {
            let n = st.name.as_str();
            let what = if st.role == AiRole::Memory {
                "Conversation memory"
            } else {
                "Cached model answers"
            };
            if n.contains("Redis") {
                if let Some(s) = c.service(ServiceKind::Cache, "redis") {
                    let sh = c.svc(s);
                    let mut pr = proof;
                    pr.push(s.declared_in.clone());
                    c.add(&st.id, what, P::Ram, &sh, L::Session, redis_bound(s), pr,
                        format!("{n} stores it in Redis, so every API worker sees the same copy."));
                }
            } else if n.contains("Postgres") {
                if let Some(s) = c.service(ServiceKind::Database, "postgres") {
                    let sh = c.svc(s);
                    c.add(&st.id, what, P::Disk, &sh, L::Persistent, None, proof,
                        format!("{n} writes each step to Postgres, so it survives restarts."));
                }
            } else if n.contains("Sqlite") || n.contains("SQLite") {
                c.add(&st.id, what, P::Disk, &h, L::Persistent, None, proof,
                    format!("{n} writes to a SQLite file next to {}.", h.name));
            } else if n.contains("Mem0") || n.contains("MemoryClient") {
                c.add(&st.id, what, P::Remote, &remote(), L::Persistent, None, proof,
                    "A hosted memory service keeps the facts between sessions.");
            } else {
                c.add(&st.id, what, P::Ram, &h, L::Session, None, proof,
                    format!("{n} keeps it in {}'s RAM: fast, but lost on restart and not shared between workers.", h.name));
            }
        }
        _ => {}
    }
}

/// The memory-related arguments of a setting: `LLM(model=..., max_model_len=8192,
/// gpu_memory_utilization=0.85)` -> `max_model_len=8192, gpu_memory_utilization=0.85`.
fn short(setting: &str) -> String {
    const KEYS: &[&str] = &[
        "gpu_memory_utilization",
        "max_model_len",
        "kv_cache_dtype",
        "n_ctx",
        "n_gpu_layers",
        "num_ctx",
        "keep_alive",
        "maxsize",
        "enable_prefix_caching",
        "cache_implementation",
        "load_in_4bit",
        "load_in_8bit",
        "k",
    ];
    let Some((_, args)) = setting.split_once('(') else {
        return setting.chars().take(80).collect();
    };
    let args = args.trim_end_matches(')');
    let kept: Vec<&str> = args
        .split(", ")
        .map(str::trim)
        .filter(|a| {
            a.split_once('=')
                .is_some_and(|(k, _)| KEYS.contains(&k.trim()))
        })
        .collect();
    if kept.is_empty() {
        setting.chars().take(80).collect()
    } else {
        kept.join(", ")
    }
}

fn technique(c: &mut Ctx, m: &MemoryTechnique) {
    let rule =
        m.id.strip_prefix("mem:")
            .and_then(|r| r.split(':').next())
            .unwrap_or("");
    let h = c.holder_of(&m.file_id);
    let proof = vec![at(&m.file_id, m.line)];
    let gpu = if h.gpu { P::Vram } else { P::Ram };
    let set = Some(short(&m.setting));
    match rule {
        "mem.vllm.paged" | "mem.kv.quant" | "mem.hf.cache" | "mem.hf.sinkcache" => c.add(
            &m.id,
            "KV cache (attention keys and values)",
            if rule.starts_with("mem.vllm") { P::Vram } else { gpu },
            &h,
            L::Request,
            set,
            proof,
            "Every token being generated keeps a key and a value per layer; this cache grows with the context and is freed when the request ends.",
        ),
        "mem.vllm.prefix" => c.add(
            &m.id,
            "Shared prompt prefixes (reused KV blocks)",
            P::Vram,
            &h,
            L::Process,
            set,
            proof,
            "KV blocks for a prefix many prompts share stay in GPU memory and are reused until space is needed.",
        ),
        "mem.llamacpp.ctx" => c.add(
            &m.id,
            "Model layers on the GPU (the rest in RAM)",
            P::Vram,
            &h,
            L::Process,
            set,
            proof,
            "n_gpu_layers moves that many layers into GPU memory; the other layers stay in RAM, memory-mapped from the model file.",
        ),
        "mem.hf.quant-weights" => c.add(
            &m.id,
            "Model weights (quantized)",
            gpu,
            &h,
            L::Process,
            set,
            proof,
            "4-bit or 8-bit weights take a quarter or half of the memory of 16-bit ones.",
        ),
        "mem.torch.ckpt" | "mem.torch.amp" | "mem.attn.impl" | "mem.attn.sdpa" => c.add(
            &m.id,
            "Activations during forward/backward",
            gpu,
            &h,
            L::Request,
            set,
            proof,
            m.explain.clone(),
        ),
        "mem.anthropic.prompt-cache" | "mem.openai.prompt-cache" => c.add(
            &m.id,
            "Cached prompt prefix (at the provider)",
            P::Remote,
            &remote(),
            L::Session,
            Some("kept a few minutes after last use".into()),
            proof,
            m.explain.clone(),
        ),
        "mem.chat.window" => c.add(
            &m.id,
            "Conversation window (last turns)",
            P::Ram,
            &h,
            L::Session,
            set,
            proof,
            m.explain.clone(),
        ),
        "mem.semantic-cache" => c.add(
            &m.id,
            "Cached answers by meaning",
            P::Ram,
            &h,
            L::Process,
            set,
            proof,
            m.explain.clone(),
        ),
        "mem.ollama.keepalive" => {
            if let Some(s) = c.service(ServiceKind::ModelServer, "ollama") {
                let sh = c.svc(s);
                let place = if s.gpu { P::Vram } else { P::Ram };
                c.add(
                    &s.id,
                    "Model weights",
                    place,
                    &sh,
                    L::Process,
                    set,
                    proof,
                    "Ollama keeps a model loaded for keep_alive after its last request, then unloads it.",
                );
            }
        }
        "mem.redis.capped-list" | "mem.redis.ttl-cache" => {
            let what = if rule.ends_with("capped-list") {
                "Chat history (last N messages per conversation)"
            } else {
                "Cached responses (expire after a TTL)"
            };
            let (holder, bound) = match c.service(ServiceKind::Cache, "redis") {
                Some(s) => {
                    let b = redis_bound(s);
                    (c.svc(s), b)
                }
                None => (
                    Holder {
                        id: "remote".into(),
                        name: "Redis server (not in this repo)".into(),
                        gpu: false,
                    },
                    None,
                ),
            };
            let bound = match bound {
                Some(b) => Some(format!("{}; {b}", short(&m.setting))),
                None => set,
            };
            c.add(
                &m.id,
                what,
                P::Ram,
                &holder,
                L::Session,
                bound,
                proof,
                m.explain.clone(),
            );
        }
        "mem.py.lru" => c.add(
            &m.id,
            "Memoized results (functools cache)",
            P::Ram,
            &h,
            L::Process,
            set,
            proof,
            m.explain.clone(),
        ),
        _ => {}
    }
}

fn service(c: &mut Ctx, s: &Service) {
    let h = c.svc(s);
    let decl = vec![s.declared_in.clone()];
    let vol = s.volumes.first().cloned();
    let mut volproof = decl.clone();
    if let Some(v) = &vol {
        volproof.push(format!("volume {v}"));
    }
    let persist = if vol.is_some() {
        L::Persistent
    } else {
        L::Process
    };
    let no_vol = " No volume is mounted, so it is lost when the container is recreated.";
    match s.kind {
        ServiceKind::Cache => {
            if !c.out.iter().any(|p| p.holder == s.id) {
                c.add(&s.id, "Keys and values", P::Ram, &h, L::Process, redis_bound(s), decl.clone(),
                    format!("{} keeps everything in RAM; reads and writes take well under a millisecond.", s.name));
            }
            let cmd = s.command.as_deref().unwrap_or("");
            if vol.is_some() || cmd.contains("--appendonly yes") || cmd.contains("--save ") {
                c.add(&s.id, "Snapshot of the keys", P::Disk, &h, L::Persistent, None, volproof,
                    "Redis writes its keys to disk (RDB snapshot or append-only file) so they come back after a restart.");
            }
        }
        ServiceKind::Database => {
            let tables: Vec<&str> =
                c.g.data_models
                    .iter()
                    .filter_map(|m| m.table.as_deref())
                    .take(4)
                    .collect();
            let what = if tables.is_empty() {
                "Tables and rows".to_owned()
            } else {
                format!("Tables: {}", tables.join(", "))
            };
            c.add(
                &s.id,
                what,
                P::Disk,
                &h,
                persist,
                limit(s),
                volproof,
                format!(
                    "{} stores rows on disk and keeps the pages it reads often in RAM.{}",
                    s.name,
                    if vol.is_some() { "" } else { no_vol }
                ),
            );
        }
        ServiceKind::VectorStore | ServiceKind::SearchEngine => {
            if !c.out.iter().any(|p| p.holder == s.id && p.place == P::Ram) {
                c.add(
                    &s.id,
                    "Vector index (HNSW graph + vectors)",
                    P::Ram,
                    &h,
                    L::Process,
                    limit(s),
                    decl.clone(),
                    format!(
                        "{} keeps the search index in RAM so lookups are fast.",
                        s.name
                    ),
                );
            }
            c.add(
                &s.id,
                "Stored collections (vectors, chunks, metadata)",
                P::Disk,
                &h,
                persist,
                None,
                volproof,
                format!(
                    "{} saves every collection to its data folder.{}",
                    s.name,
                    if vol.is_some() { "" } else { no_vol }
                ),
            );
        }
        ServiceKind::ModelServer => {
            if !c.out.iter().any(|p| p.holder == s.id) {
                c.add(
                    &s.id,
                    "Model weights",
                    if s.gpu { P::Vram } else { P::Ram },
                    &h,
                    L::Process,
                    limit(s),
                    decl.clone(),
                    format!(
                        "{} loads the models it serves into {}.",
                        s.name,
                        if s.gpu { "GPU memory" } else { "RAM" }
                    ),
                );
            }
            if vol.is_some() {
                c.add(&s.id, "Downloaded model files", P::Disk, &h, L::Persistent, None, volproof,
                    "Model files are kept on a volume so they are not downloaded again after a restart.");
            }
        }
        ServiceKind::ObjectStore => c.add(
            &s.id,
            "Uploaded files (objects)",
            P::Disk,
            &h,
            persist,
            None,
            volproof,
            format!(
                "{} stores files as objects in buckets.{}",
                s.name,
                if vol.is_some() { "" } else { no_vol }
            ),
        ),
        ServiceKind::Broker => c.add(
            &s.id,
            "Queued messages",
            P::Ram,
            &h,
            L::Process,
            limit(s),
            decl,
            format!("{} holds messages until a worker takes them.", s.name),
        ),
        _ => {}
    }
}

/// `localStorage.setItem`, `sessionStorage.setItem`, `indexedDB.open`, zustand `persist`.
fn browser(c: &mut Ctx, files: &[ParsedFile]) {
    let h = Holder {
        id: "browser".into(),
        name: "the user's browser".into(),
        gpu: false,
    };
    for f in files {
        let path = f.scan.path.as_str();
        let calls = f
            .facts
            .functions
            .iter()
            .flat_map(|x| x.calls.iter())
            .chain(f.facts.top_calls.iter());
        let mut seen: BTreeMap<&str, u32> = BTreeMap::new();
        for call in calls {
            let recv = call.receiver.as_deref().unwrap_or("");
            let kind = if recv.ends_with("localStorage") && call.callee == "setItem" {
                "local"
            } else if recv.ends_with("sessionStorage") && call.callee == "setItem" {
                "session"
            } else if recv.ends_with("indexedDB") && call.callee == "open" {
                "idb"
            } else if recv.is_empty() && call.callee == "persist" {
                "persist"
            } else {
                continue;
            };
            let l = seen.entry(kind).or_insert(call.line);
            *l = (*l).min(call.line);
        }
        for (kind, line) in seen {
            let key = call_key(f, line);
            let (what, life, explain) = match kind {
                "local" => (
                    format!("localStorage{key}"),
                    L::Persistent,
                    "Saved in the browser per site, up to about 5 MB; it stays after the tab closes until the user clears site data.",
                ),
                "session" => (
                    format!("sessionStorage{key}"),
                    L::Session,
                    "Saved in the browser for this tab only; it is cleared when the tab closes.",
                ),
                "idb" => (
                    "IndexedDB database".to_owned(),
                    L::Persistent,
                    "A database inside the browser for larger structured data and files; it survives restarts.",
                ),
                _ => (
                    "Persisted app state (zustand persist)".to_owned(),
                    L::Persistent,
                    "The store writes its state to localStorage so it comes back after a reload.",
                ),
            };
            c.add(
                &file_id(path),
                what,
                P::Browser,
                &h,
                life,
                None,
                vec![format!("{path}:{line}")],
                explain,
            );
        }
    }
}

/// ` ("token")` when the setItem call at that line has a literal key.
fn call_key(f: &ParsedFile, line: u32) -> String {
    f.facts
        .functions
        .iter()
        .flat_map(|x| x.calls.iter())
        .chain(f.facts.top_calls.iter())
        .find(|c| c.line == line)
        .and_then(|c| c.first_string.clone())
        .map(|k| format!(" (\"{k}\")"))
        .unwrap_or_default()
}

pub fn build(g: &SystemArchitectureGraph, files: &[ParsedFile]) -> Vec<MemoryPlacement> {
    let services: &[Service] = g
        .insights
        .as_ref()
        .and_then(|i| i.deployment.as_ref())
        .map(|d| d.services.as_slice())
        .unwrap_or(&[]);
    let mut tier_of = BTreeMap::new();
    for t in &g.tiers {
        let name = format!("{:?}", t.label).to_ascii_lowercase();
        for cl in &t.clusters {
            for f in &cl.files {
                tier_of.insert(f.path.clone(), name.clone());
            }
        }
    }
    let mut c = Ctx {
        g,
        services,
        tier_of,
        out: Vec::new(),
    };
    if let Some(ins) = &g.insights {
        if let Some(ai) = &ins.ai {
            for st in &ai.stages {
                stage(&mut c, st);
            }
        }
        for m in &ins.memory_techniques {
            technique(&mut c, m);
        }
    }
    for s in services {
        service(&mut c, s);
    }
    browser(&mut c, files);
    let mut out = c.out;
    for p in &mut out {
        p.proof.sort();
        p.proof.dedup();
    }
    out.sort_by(|a, b| (a.place, &a.holder, &a.what).cmp(&(b.place, &b.holder, &b.what)));
    for (i, p) in out.iter_mut().enumerate() {
        p.id = format!("memmap:{i}");
    }
    out
}
