//! Example runs: follow one UI element through the running system, hop by hop.
//!
//! Every hop comes from something already proven in the graph: a UI handler, a resolved
//! call, an HTTP edge matched to a route, a proxy rule, an import, an AI pipeline edge,
//! or a service link declared in compose. When a step is matched by import or by kind
//! (not a resolved call) its confidence is lowered and the proof says so. Nothing is
//! invented: if the chain breaks, the run stops there.
//!
//! The same output is what an AI narrator gets from the `ark run` tool, so it can only
//! describe hops that exist.

use std::collections::{BTreeMap, BTreeSet};

use ark_schema::{
    AiRole, CrossTierEdge, Deployment, ExampleRun, Finding, FunctionNode, Protocol, RunHop,
    RunHopKind, RunVia, Service, ServiceKind, SourceFileNode, SystemArchitectureGraph, UiElement,
};

const MAX_RUNS: usize = 6;
const MAX_CALL_DEPTH: usize = 6;

struct Ix<'a> {
    files: BTreeMap<&'a str, &'a SourceFileNode>,
    fns: BTreeMap<&'a str, (&'a SourceFileNode, &'a FunctionNode)>,
    cross: BTreeMap<&'a str, Vec<&'a CrossTierEdge>>,
    dep: Option<&'a Deployment>,
}

impl<'a> Ix<'a> {
    fn new(g: &'a SystemArchitectureGraph) -> Self {
        let mut files = BTreeMap::new();
        let mut fns = BTreeMap::new();
        for t in &g.tiers {
            for c in &t.clusters {
                for f in &c.files {
                    files.insert(f.id.as_str(), f);
                    for x in &f.functions {
                        fns.insert(x.id.as_str(), (f, x));
                    }
                }
            }
        }
        let mut cross: BTreeMap<&str, Vec<&CrossTierEdge>> = BTreeMap::new();
        for e in &g.cross_tier_edges {
            cross.entry(e.source.as_str()).or_default().push(e);
        }
        let dep = g.insights.as_ref().and_then(|i| i.deployment.as_ref());
        Self {
            files,
            fns,
            cross,
            dep,
        }
    }

    fn file_path(&self, file_id: &str) -> String {
        self.files
            .get(file_id)
            .map(|f| f.path.clone())
            .unwrap_or_else(|| file_id.trim_start_matches("file:").to_owned())
    }

    fn fn_loc(&self, fn_id: &str) -> Option<String> {
        self.fns
            .get(fn_id)
            .map(|(f, x)| format!("{}:{}", f.path, x.line_range.0))
    }

    fn fn_label(&self, fn_id: &str) -> String {
        self.fns
            .get(fn_id)
            .map(|(_, x)| format!("{}()", x.name))
            .unwrap_or_else(|| fn_id.rsplit('#').next().unwrap_or(fn_id).to_owned())
    }

    fn file_of_fn(&self, fn_id: &str) -> Option<&'a str> {
        self.fns.get(fn_id).map(|(f, _)| f.id.as_str())
    }

    /// The service whose build context holds this file (the deepest one).
    fn service_of_file(&self, file_id: &str) -> Option<&'a Service> {
        let path = self.file_path(file_id);
        self.dep?
            .services
            .iter()
            .filter_map(|s| s.build_context.as_deref().map(|c| (s, c)))
            .filter(|(_, c)| c.is_empty() || path.starts_with(&format!("{c}/")))
            .max_by_key(|(_, c)| c.len())
            .map(|(s, _)| s)
    }

    fn link(&self, from: &str, to: &str) -> Option<&'a ark_schema::ServiceLink> {
        self.dep?
            .links
            .iter()
            .find(|l| l.from == from && l.to == to)
    }

    /// Services of `kinds` that `from` has a declared link to.
    fn linked_of_kind(&self, from: &str, kinds: &[ServiceKind]) -> Vec<&'a Service> {
        let Some(d) = self.dep else { return vec![] };
        d.links
            .iter()
            .filter(|l| l.from == from)
            .filter_map(|l| d.services.iter().find(|s| s.id == l.to))
            .filter(|s| kinds.contains(&s.kind))
            .collect()
    }
}

fn svc_id(s: Option<&Service>) -> Option<String> {
    s.map(|s| s.id.clone())
}

fn backing_kinds(role: AiRole) -> &'static [ServiceKind] {
    match role {
        AiRole::Llm | AiRole::Embedder => &[ServiceKind::ModelServer],
        AiRole::VectorStore | AiRole::Retriever => &[
            ServiceKind::VectorStore,
            ServiceKind::SearchEngine,
            ServiceKind::Database,
        ],
        AiRole::KeywordIndex => &[ServiceKind::SearchEngine, ServiceKind::VectorStore],
        _ => &[],
    }
}

/// Embedders that run inside the calling process, never behind a model server.
fn in_process(s: &ark_schema::AiStage) -> bool {
    let n = format!("{} {}", s.name, s.library).to_ascii_lowercase();
    s.role == AiRole::Embedder
        && [
            "huggingfaceembeddings",
            "sentencetransformer",
            "fastembed",
            "sentence_transformers",
        ]
        .iter()
        .any(|k| n.contains(k))
}

/// Query-time roles; loaders and splitters run when the index is built, not per request.
fn online(role: AiRole) -> bool {
    !matches!(
        role,
        AiRole::Loader | AiRole::Splitter | AiRole::NeuralLayer
    )
}

fn role_rank(role: AiRole) -> u8 {
    match role {
        AiRole::Agent => 0,
        AiRole::Router => 1,
        AiRole::Tool => 2,
        AiRole::Retriever => 3,
        AiRole::VectorStore => 4,
        AiRole::KeywordIndex => 5,
        AiRole::Fusion => 6,
        AiRole::Reranker => 7,
        AiRole::Prompt => 8,
        AiRole::Llm => 9,
        _ => 10,
    }
}

struct Run<'a, 'g> {
    ix: &'a Ix<'g>,
    g: &'g SystemArchitectureGraph,
    hops: Vec<RunHop>,
    seen_services: BTreeSet<String>,
}

impl Run<'_, '_> {
    fn push(&mut self, h: RunHop) {
        self.hops.push(h);
    }

    /// A hop into a backing container (vector DB, model server, cache) over a declared link.
    fn service_hop(&mut self, from: &Service, to: &Service, why: &str) {
        if !self.seen_services.insert(to.id.clone()) {
            return;
        }
        let Some(l) = self.ix.link(&from.id, &to.id) else {
            return;
        };
        self.push(RunHop {
            node: to.id.clone(),
            kind: RunHopKind::Service,
            label: format!("{} container", to.name),
            service: Some(to.id.clone()),
            via: RunVia::Network,
            proof: vec![l.detail.clone(), to.declared_in.clone()],
            confidence: 0.8,
            explain: format!(
                "{why} The {} container is reached over the network ({}).",
                to.name, l.detail
            ),
        });
    }

    fn follow_ui(&mut self, u: &UiElement) -> bool {
        let file = self.ix.file_path(&u.file_id);
        let svc = self.ix.service_of_file(&u.file_id);
        let input = sample_input(self.g, u);
        if let Some(inp) = self
            .g
            .ui_elements
            .iter()
            .find(|x| same_screen(x, u) && x.id != u.id && is_text_input(x))
        {
            self.push(RunHop {
                node: inp.id.clone(),
                kind: RunHopKind::Ui,
                label: format!("type “{}”", input.clone().unwrap_or_default()),
                service: svc_id(svc),
                via: RunVia::Start,
                proof: vec![format!("{}:{}", file, inp.line)],
                confidence: 1.0,
                explain: "The user types a question into the input on this screen.".into(),
            });
        }
        self.push(RunHop {
            node: u.id.clone(),
            kind: RunHopKind::Ui,
            label: format!(
                "<{}>{}",
                u.tag,
                u.label
                    .as_ref()
                    .map(|l| format!(" “{l}”"))
                    .unwrap_or_default()
            ),
            service: svc_id(svc),
            via: if self.hops.is_empty() {
                RunVia::Start
            } else {
                RunVia::Handler
            },
            proof: vec![format!("{}:{}", file, u.line)],
            confidence: 1.0,
            explain: format!(
                "The user {} this {}.",
                match u.event.as_deref() {
                    Some("onSubmit") => "submits",
                    Some("onChange") => "edits",
                    _ => "clicks",
                },
                u.tag
            ),
        });
        let Some(h) = u.handler.as_deref() else {
            return false;
        };
        self.push(RunHop {
            node: h.to_owned(),
            kind: RunHopKind::Function,
            label: self.ix.fn_label(h),
            service: svc_id(svc),
            via: RunVia::Handler,
            proof: self.ix.fn_loc(h).into_iter().collect(),
            confidence: 1.0,
            explain: format!(
                "Its {} handler runs in the browser.",
                u.event.as_deref().unwrap_or("event")
            ),
        });
        // Breadth-first over resolved calls until an HTTP edge leaves the frontend.
        let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
        let mut queue = vec![(h, 0usize)];
        let mut seen = BTreeSet::from([h]);
        let mut found: Option<(&str, &CrossTierEdge)> = None;
        while let Some((id, d)) = (!queue.is_empty()).then(|| queue.remove(0)) {
            if let Some(e) = self.ix.cross.get(id).and_then(|v| v.first()) {
                found = Some((id, e));
                break;
            }
            if d >= MAX_CALL_DEPTH {
                continue;
            }
            if let Some((_, f)) = self.ix.fns.get(id) {
                for c in &f.calls {
                    if c.confidence >= 0.6
                        && self.ix.fns.contains_key(c.target.as_str())
                        && seen.insert(&c.target)
                    {
                        parent.insert(&c.target, id);
                        queue.push((&c.target, d + 1));
                    }
                }
            }
        }
        let Some((caller, edge)) = found else {
            return true;
        };
        // The client-side call chain from the handler to the function that sends HTTP.
        let mut chain = vec![caller];
        let mut cur = caller;
        while let Some(p) = parent.get(cur) {
            chain.push(p);
            cur = p;
        }
        chain.reverse();
        for f in chain.iter().skip(1) {
            self.push(RunHop {
                node: (*f).to_owned(),
                kind: RunHopKind::Function,
                label: self.ix.fn_label(f),
                service: svc_id(svc),
                via: RunVia::Call,
                proof: self.ix.fn_loc(f).into_iter().collect(),
                confidence: 0.85,
                explain: "A resolved call on the way to the network request.".into(),
            });
        }
        let target_file = self.ix.file_of_fn(&edge.target);
        let back = target_file.and_then(|f| self.ix.service_of_file(f));
        let route = self
            .g
            .routes
            .iter()
            .find(|r| r.handler.as_deref() == Some(edge.target.as_str()));
        let http = edge
            .evidence
            .iter()
            .find_map(|e| e.strip_prefix("http."))
            .map(|e| match (e.strip_prefix("ANY "), route) {
                // A wrapper call with no readable verb sends what the route takes.
                (Some(url), Some(r)) => format!("{} {url}", r.method),
                _ => e.to_owned(),
            })
            .unwrap_or_else(|| "HTTP".into());
        // Through a proxy only when both legs are declared: frontend -> proxy -> backend.
        if let (Some(fs), Some(bs), Some(d)) = (svc, back, self.ix.dep) {
            for p in d.services.iter().filter(|s| s.kind == ServiceKind::Proxy) {
                if let (Some(a), Some(b)) =
                    (self.ix.link(&fs.id, &p.id), self.ix.link(&p.id, &bs.id))
                {
                    self.seen_services.insert(p.id.clone());
                    self.push(RunHop {
                        node: p.id.clone(),
                        kind: RunHopKind::Proxy,
                        label: format!("{} (reverse proxy)", p.name),
                        service: Some(p.id.clone()),
                        via: RunVia::Http,
                        proof: vec![a.detail.clone(), b.detail.clone()],
                        confidence: 0.9,
                        explain: format!(
                            "The request reaches the reverse proxy “{}” first ({}), which forwards it to {} ({}).",
                            p.name, a.detail, bs.name, b.detail
                        ),
                    });
                    break;
                }
            }
        }
        let via = if self
            .hops
            .last()
            .is_some_and(|h| h.kind == RunHopKind::Proxy)
        {
            RunVia::Proxy
        } else {
            RunVia::Http
        };
        let mut proof = vec![edge.id.clone()];
        if let Some(r) = route {
            proof.push(format!("{}:{}", self.ix.file_path(&r.file_id), r.line));
        }
        self.push(RunHop {
            node: route
                .map(|r| r.id.clone())
                .unwrap_or_else(|| edge.target.clone()),
            kind: RunHopKind::Route,
            label: route
                .map(|r| format!("{} {}", r.method, r.path))
                .unwrap_or_else(|| self.ix.fn_label(&edge.target)),
            service: svc_id(back),
            via,
            proof,
            confidence: 1.0,
            explain: format!(
                "The browser sends {http}; the backend route {} receives it and runs {}.",
                route
                    .map(|r| format!("{} {}", r.method, r.path))
                    .unwrap_or_default(),
                self.ix.fn_label(&edge.target)
            ),
        });
        self.follow_backend(&edge.target, back);
        {
            self.push(RunHop {
                node: u.id.clone(),
                kind: RunHopKind::Response,
                label: "answer shown on screen".into(),
                service: svc_id(svc),
                via: RunVia::Return,
                proof: vec![edge.id.clone()],
                confidence: 1.0,
                explain: "The response travels back the same way and the screen updates.".into(),
            });
        }
        true
    }

    fn follow_backend(&mut self, handler: &str, svc: Option<&Service>) {
        // Calls made by the handler, grouped by the file they land in.
        let Some((hf, hfn)) = self.ix.fns.get(handler) else {
            return;
        };
        let mut by_file: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for c in &hfn.calls {
            if c.confidence >= 0.6
                && let Some(f) = self.ix.file_of_fn(&c.target)
                && f != hf.id
            {
                by_file.entry(f).or_default().push(&c.target);
            }
        }
        let memory: Vec<_> = self
            .g
            .insights
            .as_ref()
            .map(|i| i.memory_techniques.iter().collect())
            .unwrap_or_default();
        for (f, targets) in &by_file {
            let names: Vec<String> = targets.iter().map(|t| self.ix.fn_label(t)).collect();
            let path = self.ix.file_path(f);
            let mem: Vec<_> = memory.iter().filter(|m| m.file_id == *f).collect();
            self.push(RunHop {
                node: (*f).to_owned(),
                kind: RunHopKind::Function,
                label: format!("{} in {}", names.join(", "), path),
                service: svc_id(svc),
                via: RunVia::Call,
                proof: targets.iter().filter_map(|t| self.ix.fn_loc(t)).collect(),
                confidence: 0.85,
                explain: if mem.is_empty() {
                    format!(
                        "{} calls {} in {}.",
                        self.ix.fn_label(handler),
                        names.join(", "),
                        path
                    )
                } else {
                    format!(
                        "{} calls {} in {} ({}).",
                        self.ix.fn_label(handler),
                        names.join(", "),
                        path,
                        mem.iter()
                            .map(|m| m.name.as_str())
                            .collect::<Vec<_>>()
                            .join("; ")
                    )
                },
            });
            if let Some(s) = svc
                && mem.iter().any(|m| m.engine.contains("Redis"))
            {
                for c in self.ix.linked_of_kind(&s.id, &[ServiceKind::Cache]) {
                    self.service_hop(s, c, &format!("{path} keeps its data in Redis."));
                }
            }
        }
        // Tables the handler (or a function it calls) reads or writes.
        let mut touched: Vec<&str> = vec![handler];
        touched.extend(by_file.values().flatten().copied());
        let mut seen_models: BTreeSet<&str> = BTreeSet::new();
        for f in touched {
            for e in self.ix.cross.get(f).into_iter().flatten() {
                if e.protocol != Protocol::Sql || !seen_models.insert(&e.target) {
                    continue;
                }
                let Some(m) = self.g.data_models.iter().find(|m| m.id == e.target) else {
                    continue;
                };
                let mut proof = vec![format!("{}:{}", self.ix.file_path(&m.file_id), m.line)];
                proof.extend(e.evidence.iter().take(2).cloned());
                self.push(RunHop {
                    node: m.id.clone(),
                    kind: RunHopKind::Data,
                    label: m
                        .table
                        .clone()
                        .map(|t| format!("{} ({t})", m.name))
                        .unwrap_or_else(|| m.name.clone()),
                    service: svc_id(svc),
                    via: RunVia::Sql,
                    proof,
                    confidence: 0.85,
                    explain: format!(
                        "{} reads or writes the {} table through {} ({}).",
                        self.ix.fn_label(f),
                        m.table.as_deref().unwrap_or(&m.name),
                        m.framework,
                        e.evidence.first().map(String::as_str).unwrap_or("SQL")
                    ),
                });
                if let Some(s) = svc {
                    let dbs = self.ix.linked_of_kind(&s.id, &[ServiceKind::Database]);
                    if let [one] = dbs.as_slice() {
                        self.service_hop(s, one, "The rows live in the database.");
                    }
                }
            }
        }
        // Then the AI pipeline, entered through a file the handler's file imports.
        let Some(ai) = self.g.insights.as_ref().and_then(|i| i.ai.as_ref()) else {
            return;
        };
        let stage_files: BTreeSet<&str> = ai
            .stages
            .iter()
            .filter(|s| online(s.role))
            .map(|s| s.file_id.as_str())
            .collect();
        let entry = if stage_files.contains(hf.id.as_str()) {
            Some((hf.id.as_str(), None))
        } else {
            hf.imports
                .iter()
                .map(|e| e.target.as_str())
                .find(|t| stage_files.contains(t))
                .map(|t| {
                    (
                        t,
                        Some(format!("{} imports {}", hf.path, self.ix.file_path(t))),
                    )
                })
        };
        let Some((entry_file, import_proof)) = entry else {
            return;
        };
        if let Some(p) = &import_proof {
            self.push(RunHop {
                node: entry_file.to_owned(),
                kind: RunHopKind::Function,
                label: format!("hand off to {}", self.ix.file_path(entry_file)),
                service: svc_id(svc),
                via: RunVia::Import,
                proof: vec![p.clone()],
                confidence: 0.7,
                explain: format!(
                    "{} uses the pipeline defined in {} (matched by import; the call itself is dynamic).",
                    self.ix.fn_label(handler),
                    self.ix.file_path(entry_file)
                ),
            });
        }
        // Depth-first over pipeline edges, starting at stages of the entry file that
        // nothing upstream feeds.
        let stages: BTreeMap<&str, &ark_schema::AiStage> = ai
            .stages
            .iter()
            .filter(|s| online(s.role))
            .map(|s| (s.id.as_str(), s))
            .collect();
        let mut out: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
        let mut inn: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
        for e in &ai.edges {
            if e.loop_back
                || !stages.contains_key(e.from.as_str())
                || !stages.contains_key(e.to.as_str())
            {
                continue;
            }
            out.entry(&e.from).or_default().push((&e.to, &e.label));
            inn.entry(&e.to).or_default().push((&e.from, &e.label));
        }
        // Every stage connected to the entry file's stages, in data-flow order
        // (topological, ties by role then line), so parallel lanes such as dense
        // and keyword search both show before the stage that merges them.
        let mut part: BTreeSet<&str> = stages
            .values()
            .filter(|s| s.file_id == entry_file)
            .filter(|s| out.contains_key(s.id.as_str()) || inn.contains_key(s.id.as_str()))
            .map(|s| s.id.as_str())
            .collect();
        let mut frontier: Vec<&str> = part.iter().copied().collect();
        while let Some(id) = frontier.pop() {
            let near = out
                .get(id)
                .into_iter()
                .flatten()
                .chain(inn.get(id).into_iter().flatten())
                .map(|(n, _)| *n);
            for n in near {
                if part.insert(n) {
                    frontier.push(n);
                }
            }
        }
        // Stores and embedders are shown just before the first stage that reads
        // them, not when the index was filled, so they never gate the order.
        let lazy = |id: &str| {
            matches!(
                stages[id].role,
                AiRole::VectorStore | AiRole::KeywordIndex | AiRole::Embedder
            )
        };
        let mut indeg: BTreeMap<&str, usize> = part
            .iter()
            .filter(|id| !lazy(id))
            .map(|id| {
                let n = inn
                    .get(id)
                    .map_or(0, |v| v.iter().filter(|(f, _)| !lazy(f)).count());
                (*id, n)
            })
            .collect();
        let key = |id: &str| (role_rank(stages[id].role), stages[id].line, id.to_owned());
        let mut ready: Vec<&str> = indeg
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut order: Vec<(&str, Option<(&str, &str)>)> = Vec::new();
        let mut emitted: BTreeSet<&str> = BTreeSet::new();
        fn pull<'s>(
            id: &'s str,
            inn: &BTreeMap<&'s str, Vec<(&'s str, &'s str)>>,
            lazy: &dyn Fn(&str) -> bool,
            emitted: &mut BTreeSet<&'s str>,
            order: &mut Vec<(&'s str, Option<(&'s str, &'s str)>)>,
        ) {
            for (f, label) in inn.get(id).into_iter().flatten() {
                if lazy(f) && emitted.insert(f) {
                    pull(f, inn, lazy, emitted, order);
                    order.push((f, Some((id, label))));
                }
            }
        }
        loop {
            if ready.is_empty() {
                // A cycle (agent loop): break it at the earliest stage that an
                // emitted stage feeds, else at the earliest stage left.
                let left = indeg.keys().copied().filter(|id| !emitted.contains(id));
                let fed = left
                    .clone()
                    .filter(|id| {
                        inn.get(id)
                            .is_some_and(|v| v.iter().any(|(f, _)| emitted.contains(f)))
                    })
                    .min_by_key(|id| key(id));
                match fed.or_else(|| left.min_by_key(|id| key(id))) {
                    Some(id) => ready.push(id),
                    None => break,
                }
            }
            ready.sort_by_key(|id| std::cmp::Reverse(key(id)));
            let id = ready.pop().expect("non-empty");
            if !emitted.insert(id) {
                continue;
            }
            pull(id, &inn, &lazy, &mut emitted, &mut order);
            let from = inn
                .get(id)
                .into_iter()
                .flatten()
                .filter(|(f, _)| emitted.contains(f) && !lazy(f))
                .max_by_key(|(f, _)| order.iter().position(|(o, _)| o == f))
                .map(|(f, l)| (*f, *l));
            order.push((id, from));
            for (t, _) in out.get(id).into_iter().flatten() {
                if emitted.contains(t) {
                    continue;
                }
                if let Some(d) = indeg.get_mut(t) {
                    *d = d.saturating_sub(1);
                    if *d == 0 && !ready.contains(t) {
                        ready.push(t);
                    }
                }
            }
        }
        // Stores no query stage reads (ingest only) are left out of the run.
        for (id, from) in order {
            let s = stages[id];
            let path = self.ix.file_path(&s.file_id);
            let params: Vec<String> = s.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
            let feeds_back = from.is_some_and(|(f, _)| f == id);
            let label = from.map(|(_, l)| l.to_owned());
            self.push(RunHop {
                node: s.id.clone(),
                kind: RunHopKind::Stage,
                label: s.name.clone(),
                service: svc_id(self.ix.service_of_file(&s.file_id).or(svc)),
                via: RunVia::Pipeline,
                proof: vec![format!("{path}:{}", s.line), s.rule_id.clone()],
                confidence: if feeds_back { 0.6 } else { 0.9 },
                explain: format!(
                    "{}{}{}",
                    label.map(|l| format!("{l}: ")).unwrap_or_default(),
                    s.explain,
                    if params.is_empty() {
                        String::new()
                    } else {
                        format!(" Settings in the code: {}.", params.join(", "))
                    }
                ),
            });
            if let Some(code_svc) = self
                .ix
                .service_of_file(&s.file_id)
                .or(svc)
                .filter(|_| !in_process(s))
            {
                let targets = self.ix.linked_of_kind(&code_svc.id, backing_kinds(s.role));
                // Only when exactly one linked container can be meant.
                if let [one] = targets.as_slice() {
                    self.service_hop(
                        code_svc,
                        one,
                        &format!("The work behind {} happens in its own container.", s.name),
                    );
                }
            }
        }
    }
}

fn same_screen(a: &UiElement, b: &UiElement) -> bool {
    a.file_id == b.file_id && a.component == b.component
}

fn is_text_input(x: &UiElement) -> bool {
    matches!(x.tag.as_str(), "input" | "textarea") && x.label.is_some()
}

fn sample_input(g: &SystemArchitectureGraph, u: &UiElement) -> Option<String> {
    g.ui_elements
        .iter()
        .filter(|x| same_screen(x, u) && is_text_input(x))
        .find_map(|x| x.label.clone())
}

/// Example runs for the UI elements whose handlers reach the backend.
pub fn example_runs(g: &SystemArchitectureGraph) -> Vec<ExampleRun> {
    let ix = Ix::new(g);
    let mut runs: Vec<ExampleRun> = Vec::new();
    for u in g.ui_elements.iter().filter(|u| u.handler.is_some()) {
        let mut r = Run {
            ix: &ix,
            g,
            hops: Vec::new(),
            seen_services: BTreeSet::new(),
        };
        r.follow_ui(u);
        if !r.hops.iter().any(|h| h.kind == RunHopKind::Route) {
            continue;
        }
        let title = format!(
            "{} → {}",
            u.label.clone().unwrap_or_else(|| format!("<{}>", u.tag)),
            r.hops
                .iter()
                .find(|h| h.kind == RunHopKind::Route)
                .map(|h| h.label.clone())
                .unwrap_or_default()
        );
        runs.push(ExampleRun {
            id: format!("run:{}", u.id),
            title,
            start: u.id.clone(),
            sample_input: sample_input(g, u),
            hops: r.hops,
        });
    }
    // Longest (deepest) runs first; stable by id.
    runs.sort_by(|a, b| b.hops.len().cmp(&a.hops.len()).then(a.id.cmp(&b.id)));
    runs.truncate(MAX_RUNS);
    runs
}

/// When the deployment or AI pipeline is known, the headline describes the running
/// system instead of only its tiers.
/// Folders that hold tooling, samples and tests rather than the shipped system.
pub(crate) fn is_support_path(declared_in: &str) -> bool {
    let path = declared_in
        .rsplit_once(':')
        .map(|(p, _)| p)
        .unwrap_or(declared_in);
    let mut parts: Vec<&str> = path.split('/').collect();
    parts.pop();
    parts.iter().any(|seg| {
        let seg_l = seg.to_ascii_lowercase();
        if seg_l.ends_with("-tests")
            || seg_l.ends_with("-test")
            || seg_l.ends_with("-testing")
            || seg_l.ends_with("-examples")
            || seg_l.ends_with("-test-image")
        {
            return true;
        }
        matches!(
            seg.to_ascii_lowercase().as_str(),
            "dev"
                | "devctr"
                | "examples"
                | "example"
                | "samples"
                | "sample"
                | "demo"
                | "demos"
                | "test"
                | "tests"
                | "testing"
                | "e2e"
                | "integration-tests"
                | "ci"
                | "tools"
                | "hack"
                | "docs"
                | "benchmarks"
                | "bench"
                | "scripts"
                | "release"
        )
    })
}

/// Programs the repository builds: Go `cmd/<name>/*.go`, Rust `src/main.rs` and `src/bin/*.rs`.
/// Top-level ones first, biggest first.
fn entrypoints(g: &SystemArchitectureGraph) -> Vec<String> {
    // (depth, name, root folder whose files measure its size)
    let mut found: Vec<(usize, String, String)> = Vec::new();
    let mut paths: Vec<&str> = Vec::new();
    for f in g
        .tiers
        .iter()
        .flat_map(|t| t.clusters.iter())
        .flat_map(|c| c.files.iter())
    {
        let p = f.path.as_str();
        if is_support_path(&format!("{p}:0")) {
            continue;
        }
        paths.push(p);
        let parts: Vec<&str> = p.split('/').collect();
        let n = parts.len();
        if p.ends_with(".go") && n >= 3 && parts[n - 3] == "cmd" && !parts[n - 2].starts_with('_') {
            found.push((n, parts[n - 2].to_owned(), parts[..n - 1].join("/")));
        } else if p.ends_with("/src/main.rs") && n >= 3 {
            found.push((n, parts[n - 3].to_owned(), parts[..n - 2].join("/")));
        } else if p.ends_with(".rs") && n >= 3 && parts[n - 2] == "bin" && parts[n - 3] == "src" {
            found.push((
                n,
                parts[n - 1].trim_end_matches(".rs").to_owned(),
                parts[..n - 1].join("/"),
            ));
        }
    }
    let Some(min_depth) = found.iter().map(|f| f.0).min() else {
        return Vec::new();
    };
    found.retain(|f| f.0 <= min_depth + 1);
    let mut seen = std::collections::BTreeSet::new();
    let mut ranked: Vec<(usize, String)> = Vec::new();
    for (_, name, root) in found {
        if !seen.insert(name.clone()) {
            continue;
        }
        let prefix = format!("{root}/");
        let weight = paths.iter().filter(|p| p.starts_with(&prefix)).count();
        ranked.push((weight, name));
    }
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    ranked.into_iter().map(|(_, n)| n).collect()
}

fn short_list(names: &[String]) -> String {
    if names.len() > 5 {
        format!("{}, and more", names[..4].join(", "))
    } else {
        names.join(", ")
    }
}

pub fn system_headline(g: &mut SystemArchitectureGraph) {
    let Some(ins) = g.insights.as_ref() else {
        return;
    };
    let packages = ins.packages.len();
    let real_services = ins.deployment.as_ref().map_or(0, |d| {
        d.services
            .iter()
            .filter(|s| !is_support_path(&s.declared_in))
            .count()
    });
    let has_real_service = real_services > 0;
    // Many programs and few containers (Kubernetes): the programs are the system.
    let image_only = ins.deployment.as_ref().is_some_and(|d| {
        d.services
            .iter()
            .filter(|s| !is_support_path(&s.declared_in))
            .all(|s| s.code_files < 10)
    });
    let many_programs = image_only && entrypoints(g).len() >= 4;
    if !has_real_service || many_programs {
        // No shipped containers: describe the programs, or the package set, instead.
        let programs = entrypoints(g);
        if programs.len() >= 2 {
            g.summary.headline = format!(
                "{} programs in {} packages: {}.",
                programs.len(),
                packages.max(1),
                short_list(&programs)
            );
            return;
        }
        if packages >= 8 && g.routes.len() < 5 {
            let names: Vec<String> = ins.packages.iter().map(|p| p.name.clone()).collect();
            g.summary.headline =
                format!("Monorepo of {packages} packages: {}.", short_list(&names));
            return;
        }
    }
    let Some(d) = ins.deployment.as_ref().filter(|d| !d.services.is_empty()) else {
        return;
    };
    // Tooling, samples and tests are in the container list but are not the system.
    let only_real: std::collections::BTreeSet<&str> = if has_real_service {
        d.services
            .iter()
            .filter(|s| is_support_path(&s.declared_in))
            .map(|s| s.name.as_str())
            .collect()
    } else {
        Default::default()
    };
    // Test doubles and load generators are in the compose files but are not the system.
    let support = |n: &str| {
        if only_real.contains(n) {
            return true;
        }
        let n = n.to_ascii_lowercase();
        n.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| {
            matches!(
                w,
                "mock"
                    | "test"
                    | "loadtest"
                    | "loadgenerator"
                    | "e2e"
                    | "bench"
                    | "prepull"
                    | "copy"
            )
        })
    };
    let name = |k: ServiceKind| {
        let all: Vec<&str> = d
            .services
            .iter()
            .filter(|s| s.kind == k && !support(&s.name))
            .map(|s| s.name.as_str())
            .collect();
        if all.len() > 5 {
            let mut v: Vec<&str> = all[..4].to_vec();
            v.push("more");
            v
        } else {
            all
        }
    };
    let group = |kinds: &[(ServiceKind, &str)]| -> Vec<String> {
        kinds
            .iter()
            .filter_map(|(k, word)| {
                let n = name(*k);
                (!n.is_empty()).then(|| format!("{word} ({})", n.join(", ")))
            })
            .collect()
    };
    let path = group(&[
        (ServiceKind::Frontend, "frontend"),
        (ServiceKind::Proxy, "proxy"),
        (ServiceKind::Api, "API"),
        (ServiceKind::Agent, "agent"),
        (ServiceKind::Worker, "worker"),
    ]);
    let backing = group(&[
        (ServiceKind::ModelServer, "model server"),
        (ServiceKind::VectorStore, "vector store"),
        (ServiceKind::SearchEngine, "search engine"),
        (ServiceKind::Database, "database"),
        (ServiceKind::Cache, "cache"),
        (ServiceKind::Broker, "queue"),
        (ServiceKind::ObjectStore, "object store"),
    ]);
    let mut parts = path.join(" → ");
    if !backing.is_empty() {
        if !parts.is_empty() {
            parts.push_str(", backed by ");
        }
        parts.push_str(&backing.join(", "));
    }
    let ai = ins.ai.as_ref().map(|a| a.headline.clone());
    let gpu = d.services.iter().filter(|s| s.gpu).count();
    // "AI system" needs real weight: a single assistant endpoint in a shop is a feature.
    let ai_files = ins
        .ai
        .as_ref()
        .map(|a| {
            a.stages
                .iter()
                .map(|s| s.file_id.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        })
        .unwrap_or(0);
    let total_files: usize = g
        .tiers
        .iter()
        .flat_map(|t| t.clusters.iter())
        .map(|c| c.files.len())
        .sum::<usize>()
        .max(1);
    let lead = match &ai {
        Some(_) if ai_files >= 8 || ai_files * 10 >= total_files => "AI system",
        Some(_) => "App with an AI feature",
        None => "Containerized app",
    };
    if parts.is_empty() {
        let mut langs: std::collections::BTreeMap<&str, usize> = Default::default();
        for f in g
            .tiers
            .iter()
            .flat_map(|t| t.clusters.iter())
            .flat_map(|c| c.files.iter())
        {
            *langs.entry(f.language.as_str()).or_default() += 1;
        }
        let mut top: Vec<(&str, usize)> = langs.into_iter().collect();
        top.sort_by_key(|x| std::cmp::Reverse(x.1));
        let total: usize = top.iter().map(|x| x.1).sum();
        let names: Vec<&str> = top.iter().take(3).map(|x| x.0).collect();
        g.summary.headline = format!("Codebase of {total} files in {}.", names.join(", "));
        return;
    }
    let n = d.services.iter().filter(|s| !support(&s.name)).count();
    let noun = if n == 1 { "container" } else { "containers" };
    g.summary.headline = format!("{lead} in {n} {noun}: {parts}.");
    let limited: u64 = d.services.iter().filter_map(|s| s.memory_limit).sum();
    let mut text = format!(
        "{} services from {}; {} links proven by env URLs, depends_on or proxy rules.",
        d.services.len(),
        d.sources.join(", "),
        d.links.len()
    );
    if limited > 0 {
        text.push_str(&format!(
            " Memory limits add up to {:.1} GB",
            limited as f64 / (1u64 << 30) as f64
        ));
        if gpu > 0 {
            text.push_str(&format!("; {gpu} service(s) ask for a GPU."));
        } else {
            text.push('.');
        }
    }
    let mut evidence: Vec<String> = d.services.iter().map(|s| s.id.clone()).collect();
    evidence.truncate(12);
    g.summary.findings.insert(0, Finding { text, evidence });
    if let Some(h) = ai {
        g.summary.findings.insert(
            1,
            Finding {
                text: h,
                evidence: vec!["insights.ai".into()],
            },
        );
    }
}
