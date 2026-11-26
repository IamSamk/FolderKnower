//! `ark-link`: cross-file resolution. Turns per-file facts into a graph:
//! import edges, call edges (with provenance + confidence), routes with resolved
//! handlers, UI elements with resolved handlers, and frontend-to-backend HTTP edges.
//!
//! All edges come from static evidence. Nothing here guesses with a model.

mod resolve;

use std::collections::{BTreeMap, BTreeSet};

use ark_parse::FileFacts;
use ark_scan::ScannedFile;
use ark_schema::{CrossTierEdge, Edge, Protocol, Provenance, RouteNode, UiElement};

pub use resolve::{external_package, normalize_url_segments};

pub struct ParsedFile {
    pub scan: ScannedFile,
    pub facts: FileFacts,
}

pub fn file_id(path: &str) -> String {
    format!("file:{path}")
}

pub fn fn_id(path: &str, qualname: &str) -> String {
    format!("fn:{path}#{qualname}")
}

/// Go module roots found in `go.mod` files: (directory, module path).
#[derive(Debug, Clone, Default)]
pub struct LinkInputs {
    pub go_modules: Vec<(String, String)>,
}

#[derive(Debug, Default)]
pub struct Linked {
    /// path -> internal import edges (target = file id).
    pub file_imports: BTreeMap<String, Vec<Edge>>,
    /// path -> external package names it imports.
    pub externals: BTreeMap<String, BTreeSet<String>>,
    /// fn id -> outgoing call edges.
    pub fn_calls: BTreeMap<String, Vec<Edge>>,
    /// fn id -> callers.
    pub called_by: BTreeMap<String, BTreeSet<String>>,
    pub routes: Vec<RouteNode>,
    pub ui_elements: Vec<UiElement>,
    /// Frontend/client function -> backend route handler.
    pub http_edges: Vec<CrossTierEdge>,
    /// path -> relative imports that resolve to no file: (line, specifier).
    pub unresolved_imports: BTreeMap<String, Vec<(u32, String)>>,
    /// Same-origin HTTP calls (`/api/...`) that match no route declared here:
    /// (path, line, method, url). Only filled when the repository declares routes.
    pub dead_calls: Vec<(String, u32, String, String)>,
}

/// Round confidence to 2 decimals so floating noise never changes output bytes.
pub fn conf(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

const STOP: &[&str] = &[
    "get",
    "set",
    "map",
    "filter",
    "push",
    "append",
    "len",
    "join",
    "split",
    "then",
    "catch",
    "log",
    "error",
    "warn",
    "info",
    "debug",
    "toString",
    "format",
    "keys",
    "values",
    "items",
    "update",
    "add",
    "remove",
    "find",
    "forEach",
    "reduce",
    "slice",
    "includes",
    "replace",
    "trim",
    "parse",
    "json",
    "send",
    "write",
    "read",
    "close",
    "open",
    "next",
    "call",
    "apply",
    "bind",
    "emit",
    "on",
    "use",
    "new",
    "default",
    "init",
    "run",
    "main",
    "test",
    "length",
    "status",
    "render",
    "create",
    "delete",
    "save",
    "load",
    "start",
    "stop",
    "clone",
    "into",
    "from",
    "unwrap",
    "expect",
    "iter",
    "collect",
    "to_string",
    "as_str",
    "Println",
    "Printf",
    "Errorf",
    "Sprintf",
    "print",
    "str",
    "int",
    "list",
    "dict",
    "type",
    "isinstance",
    "super",
    "setState",
    "useState",
    "useEffect",
    "require",
    "sort",
    "sorted",
    "pop",
    "shift",
    "max",
    "min",
    "abs",
    "round",
    "copy",
    "extend",
    "insert",
    "lower",
    "upper",
    "strip",
    "encode",
    "decode",
    "match",
    "test",
    "exec",
    "assert",
    "fetch",
    "has",
    "contains",
    "query",
];

struct Index<'a> {
    files: &'a [ParsedFile],
    /// short name -> [(file idx, qualname)]
    by_name: BTreeMap<&'a str, Vec<(usize, &'a str)>>,
    /// per file: local imported name -> target file indices
    imported_names: Vec<BTreeMap<String, Vec<usize>>>,
}

impl<'a> Index<'a> {
    fn fns_in(&self, fi: usize, name: &str) -> Vec<&'a str> {
        let f = &self.files[fi].facts;
        let mut v: Vec<&str> = f
            .functions
            .iter()
            .filter(|x| x.qualname == name || x.name == name)
            .map(|x| x.qualname.as_str())
            .collect();
        // Prefer exact qualname, then top-level.
        v.sort_by_key(|q| (*q != name, q.contains(['.', ':']), q.len()));
        v.dedup();
        v
    }

    /// Resolve a call from file `fi` to (target fn ids, provenance, confidence, candidates).
    fn resolve_call(
        &self,
        fi: usize,
        receiver: Option<&str>,
        callee: &str,
        enclosing_class: Option<&str>,
    ) -> Option<Edge> {
        let path = self.files[fi].scan.path.as_str();
        let recv_root = receiver.map(|r| r.split(['.', ':', '(', '[']).next().unwrap_or(r));
        let local_recv = matches!(recv_root, None | Some("this") | Some("self") | Some("Self"));

        // 1. Same file (methods on self/this resolve within the enclosing class first).
        if local_recv {
            if let Some(cls) = enclosing_class
                && recv_root.is_some()
            {
                for sep in [".", "::"] {
                    let q = format!("{cls}{sep}{callee}");
                    if !self.fns_in(fi, &q).is_empty() {
                        return Some(edge(fn_id(path, &q), Provenance::TsResolved, 0.95, None));
                    }
                }
            }
            if let Some(q) = self.fns_in(fi, callee).first() {
                return Some(edge(fn_id(path, q), Provenance::TsResolved, 0.9, None));
            }
        }
        // 2. Imported name (`import { createTodo }`) or imported namespace (`api.createTodo`).
        let key = if local_recv { Some(callee) } else { recv_root };
        if let Some(k) = key
            && let Some(targets) = self.imported_names[fi].get(k)
        {
            let mut hits = Vec::new();
            for &t in targets {
                for q in self.fns_in(t, callee) {
                    hits.push(fn_id(&self.files[t].scan.path, q));
                }
                // Class import + method call: `Repo.find` -> `Repo.find`
                if !local_recv && let Some(r) = recv_root {
                    for sep in [".", "::"] {
                        let q = format!("{r}{sep}{callee}");
                        if !self.fns_in(t, &q).is_empty() {
                            hits.push(fn_id(&self.files[t].scan.path, &q));
                        }
                    }
                }
            }
            // `send_digest.delay(...)`: attribute call on an imported function (Celery,
            // functools wrappers) is a call into that function.
            if hits.is_empty()
                && !local_recv
                && let Some(r) = recv_root
            {
                for &t in targets {
                    for q in self.fns_in(t, r) {
                        hits.push(fn_id(&self.files[t].scan.path, q));
                    }
                }
            }
            hits.sort();
            hits.dedup();
            if let Some(first) = hits.first() {
                let cands = (hits.len() > 1).then(|| hits.clone());
                return Some(edge(
                    first.clone(),
                    Provenance::TsResolved,
                    if cands.is_some() { 0.7 } else { 0.85 },
                    cands,
                ));
            }
        }
        // 3. Global name match, ranked by path proximity.
        if STOP.contains(&callee) || callee.len() < 3 {
            return None;
        }
        let cands = self.by_name.get(callee)?;
        let mut ranked: Vec<(usize, String)> = cands
            .iter()
            .filter(|(t, _)| *t != fi)
            .map(|(t, q)| {
                let tp = &self.files[*t].scan.path;
                let common = path
                    .split('/')
                    .zip(tp.split('/'))
                    .take_while(|(a, b)| a == b)
                    .count();
                (usize::MAX - common, fn_id(tp, q))
            })
            .collect();
        if ranked.is_empty() || ranked.len() > 6 {
            return None;
        }
        ranked.sort();
        let ids: Vec<String> = ranked.into_iter().map(|(_, id)| id).collect();
        let (c, cands) = match (ids.len(), local_recv) {
            (1, true) => (0.6, None),
            (1, false) => (0.4, None),
            _ => (0.3, Some(ids.clone())),
        };
        Some(edge(ids[0].clone(), Provenance::TsNameMatch, c, cands))
    }
}

fn edge(
    target: String,
    provenance: Provenance,
    confidence: f64,
    candidates: Option<Vec<String>>,
) -> Edge {
    Edge {
        target,
        provenance,
        confidence: conf(confidence),
        candidates,
    }
}

fn enclosing_class(qualname: &str) -> Option<&str> {
    qualname
        .rsplit_once("::")
        .or_else(|| qualname.rsplit_once('.'))
        .map(|(c, _)| c)
}

/// Link all files. `files` must be sorted by path.
pub fn link(files: &[ParsedFile], inputs: &LinkInputs) -> Linked {
    let mut by_name: BTreeMap<&str, Vec<(usize, &str)>> = BTreeMap::new();
    for (i, f) in files.iter().enumerate() {
        for func in &f.facts.functions {
            by_name
                .entry(func.name.as_str())
                .or_default()
                .push((i, func.qualname.as_str()));
        }
    }
    let resolver = resolve::Resolver::new(files, inputs);

    let mut out = Linked::default();
    let mut imported_names = vec![BTreeMap::<String, Vec<usize>>::new(); files.len()];
    for (i, f) in files.iter().enumerate() {
        let Some(lang) = f.scan.language else {
            continue;
        };
        let mut targets = BTreeSet::new();
        for imp in &f.facts.imports {
            let resolved = resolver.resolve_import(&f.scan.path, lang, &imp.source, &imp.names);
            if resolved.is_empty() {
                if imp.source.starts_with('.') {
                    out.unresolved_imports
                        .entry(f.scan.path.clone())
                        .or_default()
                        .push((imp.line, imp.source.clone()));
                }
                if let Some(pkg) = external_package(lang, &imp.source) {
                    out.externals
                        .entry(f.scan.path.clone())
                        .or_default()
                        .insert(pkg);
                }
                continue;
            }
            for &t in &resolved {
                if t != i {
                    targets.insert(t);
                }
            }
            for n in &imp.names {
                imported_names[i]
                    .entry(n.clone())
                    .or_default()
                    .extend(resolved.iter().copied());
            }
        }
        let edges: Vec<Edge> = targets
            .into_iter()
            .map(|t| {
                edge(
                    file_id(&files[t].scan.path),
                    Provenance::TsResolved,
                    0.95,
                    None,
                )
            })
            .collect();
        out.file_imports.insert(f.scan.path.clone(), edges);
    }
    for m in &mut imported_names {
        for v in m.values_mut() {
            v.sort();
            v.dedup();
        }
    }
    let idx = Index {
        files,
        by_name,
        imported_names,
    };

    // Calls
    for (i, f) in files.iter().enumerate() {
        for func in &f.facts.functions {
            let src_id = fn_id(&f.scan.path, &func.qualname);
            let cls = enclosing_class(&func.qualname);
            let mut edges: BTreeMap<String, Edge> = BTreeMap::new();
            // Functions passed as values (callbacks, handlers, DI providers) are edges too,
            // but only when the reference resolves through scope or imports.
            let mut refs = Vec::new();
            for c in &func.calls {
                for a in &c.ident_args {
                    let (recv, name) = match a.rsplit_once('.') {
                        Some((r, n)) => (Some(r), n),
                        None => (None, a.as_str()),
                    };
                    if let Some(mut e) = idx.resolve_call(i, recv, name, cls)
                        && e.provenance == Provenance::TsResolved
                        && e.target.starts_with("fn:")
                    {
                        e.confidence = conf(e.confidence.min(0.7));
                        refs.push(e);
                    }
                }
            }
            for e in refs {
                if e.target != src_id {
                    edges.entry(e.target.clone()).or_insert(e);
                }
            }
            for c in &func.calls {
                if let Some(e) = idx.resolve_call(i, c.receiver.as_deref(), &c.callee, cls) {
                    if e.target == src_id {
                        continue; // recursion is captured by metrics
                    }
                    edges
                        .entry(e.target.clone())
                        .and_modify(|old| {
                            if e.confidence > old.confidence {
                                *old = e.clone();
                            }
                        })
                        .or_insert(e);
                }
            }
            for t in edges.keys() {
                out.called_by
                    .entry(t.clone())
                    .or_default()
                    .insert(src_id.clone());
            }
            out.fn_calls.insert(src_id, edges.into_values().collect());
        }
    }

    // Routes
    for (i, f) in files.iter().enumerate() {
        let path = f.scan.path.as_str();
        let mut facts_routes = f.facts.routes.clone();
        facts_routes.extend(resolve::file_based_routes(path, &f.facts));
        for r in facts_routes {
            let handler = r.handler.as_deref().and_then(|h| {
                let (recv, name) = match h.rsplit_once('.') {
                    Some((a, b)) if !h.contains("@L") => (Some(a), b),
                    _ => (None, h),
                };
                idx.resolve_call(i, recv, name, None)
                    .filter(|e| e.provenance == Provenance::TsResolved)
                    .map(|e| e.target)
                    .or_else(|| idx.fns_in(i, h).first().map(|q| fn_id(path, q)))
            });
            out.routes.push(RouteNode {
                id: format!("route:{} {}#{}", r.method, r.path, path),
                method: r.method,
                path: r.path,
                framework: r.framework,
                file_id: file_id(path),
                line: r.line,
                handler,
            });
        }
    }
    out.routes.sort_by(|a, b| a.id.cmp(&b.id));
    out.routes.dedup_by(|a, b| a.id == b.id);

    // UI elements
    for (i, f) in files.iter().enumerate() {
        let path = f.scan.path.as_str();
        for u in &f.facts.ui_elements {
            let component = u.in_fn.as_ref().map(|q| fn_id(path, q));
            let handler = u.handler.as_deref().and_then(|h| {
                let (recv, name) = match h.rsplit_once('.') {
                    Some((a, b)) => (Some(a), b),
                    None => (None, h),
                };
                idx.resolve_call(i, recv, name, None).map(|e| e.target)
            });
            out.ui_elements.push(UiElement {
                id: format!("ui:{path}:{}:{}", u.line, u.column),
                tag: u.tag.clone(),
                label: u.label.clone(),
                file_id: file_id(path),
                line: u.line,
                component,
                event: u.event.clone(),
                handler,
            });
        }
    }

    // HTTP client call -> route
    let route_segs: Vec<(Vec<String>, &RouteNode)> = out
        .routes
        .iter()
        .map(|r| (normalize_url_segments(&r.path), r))
        .collect();
    let mut http: BTreeMap<String, CrossTierEdge> = BTreeMap::new();
    for f in files {
        for call in &f.facts.http_calls {
            let segs = normalize_url_segments(&call.url);
            if segs.is_empty() {
                continue;
            }
            let mut best: Vec<(&RouteNode, u8)> = Vec::new();
            for (rs, r) in &route_segs {
                if !method_ok(&call.method, &r.method) {
                    continue;
                }
                let score = match_score(&segs, rs);
                if score == 0 {
                    continue;
                }
                match best.first().map(|b| b.1) {
                    Some(s) if score < s => {}
                    Some(s) if score == s => best.push((r, score)),
                    _ => best = vec![(r, score)],
                }
            }
            if best.is_empty() && !route_segs.is_empty() && call.url.starts_with('/') {
                out.dead_calls.push((
                    f.scan.path.clone(),
                    call.line,
                    call.method.clone(),
                    call.url.clone(),
                ));
            }
            let source = call
                .in_fn
                .as_ref()
                .map(|q| fn_id(&f.scan.path, q))
                .unwrap_or_else(|| file_id(&f.scan.path));
            for (r, score) in best {
                let target = r.handler.clone().unwrap_or_else(|| r.id.clone());
                let id = format!("xedge:{source}->{target}");
                http.entry(id.clone()).or_insert(CrossTierEdge {
                    id,
                    source: source.clone(),
                    target,
                    protocol: Protocol::HttpRest,
                    evidence: vec![
                        format!("http.{} {}", call.method, call.url),
                        format!("route.{}", r.framework),
                        if score == 2 {
                            "match.exact".into()
                        } else {
                            "match.suffix".into()
                        },
                    ],
                });
            }
        }
    }
    out.http_edges = http.into_values().collect();
    out
}

fn method_ok(call: &str, route: &str) -> bool {
    route == "ANY" || route.split('|').any(|m| m.eq_ignore_ascii_case(call))
}

/// 2 = same shape, 1 = one is a mounted suffix of the other, 0 = no match.
fn match_score(url: &[String], route: &[String]) -> u8 {
    let eq = |a: &str, b: &str| a == b || a == "*" || b == "*";
    if url.len() == route.len() && url.iter().zip(route).all(|(a, b)| eq(a, b)) {
        return 2;
    }
    let (long, short) = if url.len() > route.len() {
        (url, route)
    } else {
        (route, url)
    };
    if short.is_empty() || short.iter().all(|s| s == "*") {
        return 0;
    }
    let off = long.len() - short.len();
    if long[off..].iter().zip(short).all(|(a, b)| eq(a, b)) && long[..off].iter().all(|s| s != "*")
    {
        return 1;
    }
    0
}

#[cfg(test)]
mod tests;
