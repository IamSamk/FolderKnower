//! Features: what the project does for its users, one entry per route family
//! (`/todos/*`, `/auth/*`, `/ask`) with everything that serves it.
//!
//! Seeds are the route handlers. The backend side follows resolved calls from each
//! handler; the frontend side is every client function with an HTTP edge to one of the
//! handlers, and every UI element whose event handler reaches such a function. Tables
//! come from SQL/ORM edges of the feature's functions, AI stages from functions it
//! reaches. A file reached from two features is listed as shared, not owned. Nothing is
//! assigned by name similarity.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ark_link::ParsedFile;
use ark_schema::{
    AiRole, Feature, Protocol, Service, ServiceKind, SystemArchitectureGraph, UiElement,
};

const OPS: &[&str] = &[
    "health", "healthz", "ready", "readyz", "livez", "metrics", "docs", "openapi", "status", "ping",
];

const AUTH: &[&str] = &[
    "auth", "login", "logout", "signin", "signup", "register", "session", "sessions", "token",
    "oauth", "me",
];

/// First meaningful URL segment: `/api/v1/todos/:id` -> `todos`.
fn family(path: &str) -> String {
    for seg in path.split('/') {
        let s = seg.trim();
        if s.is_empty()
            || s == "api"
            || s.starts_with(':')
            || s.starts_with('{')
            || s.starts_with('[')
            || s.starts_with('<')
            || s == "*"
            || (s.len() <= 3 && s.starts_with('v') && s[1..].chars().all(|c| c.is_ascii_digit()))
        {
            continue;
        }
        let s = s.trim_end_matches(".json").to_ascii_lowercase();
        return if OPS.contains(&s.as_str()) {
            "health".into()
        } else if AUTH.contains(&s.as_str()) {
            "auth".into()
        } else {
            s
        };
    }
    "home".into()
}

/// Desktop commands have names, not URLs: `ai_install`, `ai_status`, `check_update`.
/// Commands sharing a first word form one feature ("AI"), then ones sharing a last
/// word ("Update"); the rest stand alone ("Read Source").
fn ipc_families(routes: &[ark_schema::RouteNode]) -> BTreeMap<&str, String> {
    let words = |r: &ark_schema::RouteNode| -> Vec<String> {
        r.path
            .split(['_', '-', ':', '.'])
            .filter(|w| !w.is_empty())
            .map(str::to_ascii_lowercase)
            .collect()
    };
    let ipc: Vec<&ark_schema::RouteNode> = routes.iter().filter(|r| r.method == "IPC").collect();
    let count = |pick: &dyn Fn(&[String]) -> Option<String>| {
        let mut m: BTreeMap<String, usize> = BTreeMap::new();
        for r in &ipc {
            if let Some(w) = pick(&words(r)) {
                *m.entry(w).or_insert(0) += 1;
            }
        }
        m
    };
    // `note` and `notes` are the same thing: compare without a plural `s`.
    let stem = |w: &String| -> String {
        if w.len() > 3 && w.ends_with('s') && !w.ends_with("ss") {
            w[..w.len() - 1].to_owned()
        } else {
            w.clone()
        }
    };
    let first = |w: &[String]| (w.len() > 1).then(|| stem(&w[0]));
    let last = |w: &[String]| (w.len() > 1).then(|| stem(&w[w.len() - 1]));
    let firsts = count(&first);
    let lasts = count(&last);
    ipc.iter()
        .map(|r| {
            let w = words(r);
            let key = match (first(&w), last(&w)) {
                (Some(f), _) if firsts[&f] >= 2 => f,
                (_, Some(l)) if lasts[&l] >= 2 => l,
                _ => w.join("-"),
            };
            (r.id.as_str(), key)
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|(id, key)| {
            // Name the group with the plural when any command uses it ("Notes").
            let plural = format!("{key}s");
            let has_plural = ipc.iter().any(|r| words(r).contains(&plural));
            (id, if has_plural { plural } else { key })
        })
        .collect()
}

fn title(key: &str) -> String {
    match key {
        "auth" => "Sign in".into(),
        "health" => "Health checks".into(),
        "home" => "Home".into(),
        _ => {
            let words: Vec<String> = key
                .split(['-', '_'])
                .filter(|w| !w.is_empty())
                .map(|w| {
                    if w.len() <= 3
                        && !matches!(
                            w,
                            "app"
                                | "ask"
                                | "log"
                                | "map"
                                | "run"
                                | "set"
                                | "get"
                                | "add"
                                | "new"
                                | "top"
                        )
                        || matches!(w, "json" | "html" | "http" | "grpc")
                    {
                        return w.to_ascii_uppercase();
                    }
                    let mut c = w.chars();
                    c.next()
                        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                        .unwrap_or_default()
                })
                .collect();
            words.join(" ")
        }
    }
}

fn file_of(id: &str) -> Option<String> {
    let rest = id.strip_prefix("fn:").or_else(|| {
        id.strip_prefix("route:")
            .and_then(|r| r.split_once('#').map(|x| x.1))
    })?;
    Some(format!("file:{}", rest.split('#').next().unwrap_or(rest)))
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// Files a function enters through an imported object it calls dynamically, e.g.
/// `agent.invoke(...)` where `from .agent import app as agent`. The import must resolve
/// to one of the file's import edges with the same module stem.
fn dynamic_entries(
    fns: &BTreeSet<String>,
    files: &BTreeMap<&str, &ParsedFile>,
    imports: &BTreeMap<String, Vec<String>>,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for f in fns {
        let Some((path, qual)) = f.strip_prefix("fn:").and_then(|r| r.split_once('#')) else {
            continue;
        };
        let Some(pf) = files.get(path) else { continue };
        let Some(func) = pf.facts.functions.iter().find(|x| x.qualname == qual) else {
            continue;
        };
        let targets = imports.get(&format!("file:{path}"));
        for c in &func.calls {
            let root = c
                .receiver
                .as_deref()
                .map(|r| r.split(['.', '(']).next().unwrap_or(r))
                .unwrap_or(c.callee.as_str());
            for imp in &pf.facts.imports {
                if !imp.names.iter().any(|n| n == root) {
                    continue;
                }
                let stem = imp
                    .source
                    .rsplit(['.', '/'])
                    .find(|s| !s.is_empty())
                    .unwrap_or("");
                for t in targets.into_iter().flatten() {
                    let file = t.rsplit('/').next().unwrap_or(t);
                    let fstem = file.split('.').next().unwrap_or(file);
                    let dir_stem = t
                        .trim_end_matches("/__init__.py")
                        .trim_end_matches("/index.ts")
                        .rsplit('/')
                        .next()
                        .unwrap_or("");
                    if !stem.is_empty() && (fstem == stem || dir_stem == stem) {
                        out.insert(t.clone());
                    }
                }
            }
        }
    }
    out
}

fn offline(id: &str, a: &ark_schema::AiArchitecture) -> bool {
    a.stages.iter().any(|s| {
        s.id == id
            && matches!(
                s.role,
                AiRole::Loader
                    | AiRole::Splitter
                    | AiRole::Embedder
                    | AiRole::VectorStore
                    | AiRole::KeywordIndex
            )
    })
}

fn is_builtin(p: &str) -> bool {
    crate::health::is_stdlib(p)
}

pub fn build(
    g: &SystemArchitectureGraph,
    externals: &BTreeMap<String, BTreeSet<String>>,
    parsed: &[ParsedFile],
) -> Vec<Feature> {
    let by_path: BTreeMap<&str, &ParsedFile> =
        parsed.iter().map(|f| (f.scan.path.as_str(), f)).collect();
    let mut file_imports: BTreeMap<String, Vec<String>> = BTreeMap::new();
    // Call graph from the assembled functions.
    let mut calls: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for t in &g.tiers {
        for c in &t.clusters {
            for f in &c.files {
                file_imports.insert(
                    f.id.clone(),
                    f.imports.iter().map(|e| e.target.clone()).collect(),
                );
                for func in &f.functions {
                    calls.insert(
                        func.id.as_str(),
                        func.calls
                            .iter()
                            .map(|e| e.target.as_str())
                            .filter(|t| t.starts_with("fn:"))
                            .collect(),
                    );
                }
            }
        }
    }
    let reach = |seeds: &[&str], depth: u32| -> BTreeSet<String> {
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let mut q: VecDeque<(&str, u32)> = seeds.iter().map(|s| (*s, 0)).collect();
        while let Some((n, d)) = q.pop_front() {
            if !seen.insert(n.to_owned()) || d >= depth {
                continue;
            }
            for t in calls.get(n).into_iter().flatten() {
                q.push_back((t, d + 1));
            }
        }
        seen
    };

    let ipc = ipc_families(&g.routes);
    let mut fams: BTreeMap<String, Vec<&ark_schema::RouteNode>> = BTreeMap::new();
    for r in &g.routes {
        let key = ipc
            .get(r.id.as_str())
            .cloned()
            .unwrap_or_else(|| family(&r.path));
        fams.entry(key).or_default().push(r);
    }
    if fams.len() < 2 {
        return Vec::new();
    }

    struct Draft {
        key: String,
        routes: Vec<String>,
        fns: BTreeSet<String>,
        ui: Vec<String>,
        files: BTreeSet<String>,
        tables: BTreeSet<String>,
        frameworks: BTreeSet<String>,
        stages: Vec<String>,
    }
    let ui_by_handler: Vec<&UiElement> = g
        .ui_elements
        .iter()
        .filter(|u| u.handler.is_some())
        .collect();
    let ai = g.insights.as_ref().and_then(|i| i.ai.as_ref());
    let mut drafts: Vec<Draft> = Vec::new();
    for (key, routes) in &fams {
        let handlers: Vec<&str> = routes.iter().filter_map(|r| r.handler.as_deref()).collect();
        let mut fns = reach(&handlers, 4);
        let mut files: BTreeSet<String> = routes.iter().map(|r| r.file_id.clone()).collect();
        // Client functions that call one of these handlers over HTTP.
        let targets: BTreeSet<&str> = handlers
            .iter()
            .copied()
            .chain(routes.iter().map(|r| r.id.as_str()))
            .collect();
        let clients: BTreeSet<&str> = g
            .cross_tier_edges
            .iter()
            .filter(|e| {
                matches!(e.protocol, Protocol::HttpRest | Protocol::Ipc)
                    && targets.contains(e.target.as_str())
            })
            .map(|e| e.source.as_str())
            .collect();
        let mut ui = Vec::new();
        for u in &ui_by_handler {
            let h = u.handler.as_deref().unwrap_or("");
            let r = reach(&[h], 3);
            if r.iter().any(|x| clients.contains(x.as_str())) {
                ui.push(u.id.clone());
                files.insert(u.file_id.clone());
                // Only the path from the handler to the client belongs here, not every
                // helper the component calls.
                fns.insert(h.to_owned());
            }
        }
        // Typing into an input on the same screen as one of these buttons is part of it.
        let screens: BTreeSet<&str> = g
            .ui_elements
            .iter()
            .filter(|u| ui.contains(&u.id))
            .filter_map(|u| u.component.as_deref())
            .collect();
        for u in &g.ui_elements {
            if u.handler.is_none()
                && u.component.as_deref().is_some_and(|c| screens.contains(c))
                && matches!(u.tag.as_str(), "input" | "textarea" | "select")
                && !ui.contains(&u.id)
            {
                ui.push(u.id.clone());
            }
        }
        fns.extend(clients.iter().map(|c| c.to_string()));
        for f in &fns {
            if let Some(file) = file_of(f) {
                files.insert(file);
            }
        }
        let entered = dynamic_entries(&fns, &by_path, &file_imports);
        let mut stage_set: BTreeSet<String> = ai
            .map(|a| {
                a.stages
                    .iter()
                    .filter(|s| {
                        s.function_id.as_ref().is_some_and(|f| fns.contains(f))
                            || entered.contains(&s.file_id)
                    })
                    .map(|s| s.id.clone())
                    .collect()
            })
            .unwrap_or_default();
        // Follow the pipeline from stages entered through an import (retriever ->
        // reranker -> ...), never back into indexing.
        if let Some(a) = ai {
            let entered_ids: BTreeSet<&str> = a
                .stages
                .iter()
                .filter(|s| entered.contains(&s.file_id))
                .map(|s| s.id.as_str())
                .collect();
            let mut frontier: BTreeSet<String> =
                entered_ids.iter().map(|s| s.to_string()).collect();
            loop {
                let before = frontier.len();
                for e in &a.edges {
                    if !e.loop_back
                        && frontier.contains(&e.from)
                        && !offline(&e.from, a)
                        && !offline(&e.to, a)
                    {
                        frontier.insert(e.to.clone());
                    }
                }
                if frontier.len() == before {
                    break;
                }
            }
            stage_set.extend(frontier);
        }
        let stages: Vec<String> = stage_set.into_iter().collect();
        for st in ai.map(|a| a.stages.as_slice()).unwrap_or(&[]) {
            if stages.contains(&st.id) {
                files.insert(st.file_id.clone());
            }
        }
        files.extend(entered.iter().cloned());
        let tables = g
            .cross_tier_edges
            .iter()
            .filter(|e| e.protocol == Protocol::Sql && fns.contains(&e.source))
            .map(|e| e.target.clone())
            .collect();
        ui.sort();
        drafts.push(Draft {
            key: key.clone(),
            routes: routes.iter().map(|r| r.id.clone()).collect(),
            fns,
            ui,
            files,
            tables,
            frameworks: routes.iter().map(|r| r.framework.clone()).collect(),
            stages,
        });
    }

    let mut owners: BTreeMap<String, u32> = BTreeMap::new();
    for d in &drafts {
        for f in &d.files {
            *owners.entry(f.clone()).or_default() += 1;
        }
    }
    let services: &[Service] = g
        .insights
        .as_ref()
        .and_then(|i| i.deployment.as_ref())
        .map(|d| d.services.as_slice())
        .unwrap_or(&[]);
    let service_of = |file: &str| -> Option<&Service> {
        let path = file.strip_prefix("file:").unwrap_or(file);
        services
            .iter()
            .filter(|s| {
                s.build_context
                    .as_deref()
                    .is_some_and(|c| c.is_empty() || path.starts_with(&format!("{c}/")))
            })
            .max_by_key(|s| s.build_context.as_deref().unwrap_or("").len())
    };
    let only = |k: ServiceKind| {
        let v: Vec<&Service> = services.iter().filter(|s| s.kind == k).collect();
        (v.len() == 1).then(|| v[0].id.clone())
    };
    let mems = g
        .insights
        .as_ref()
        .map(|i| i.memory_techniques.as_slice())
        .unwrap_or(&[]);
    let pretty: BTreeMap<String, String> = g
        .insights
        .as_ref()
        .map(|i| {
            i.libraries
                .iter()
                .flat_map(|l| {
                    l.evidence.iter().map(move |e| {
                        let root = e.split(['.', '/']).next().unwrap_or(e);
                        (root.replace('-', "_"), l.name.clone())
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let mut out = Vec::new();
    for d in drafts {
        let (own, shared): (Vec<String>, Vec<String>) = d
            .files
            .iter()
            .cloned()
            .partition(|f| owners.get(f.as_str()).copied().unwrap_or(0) <= 1);
        let stages: Vec<String> = d.stages.clone();
        let roles: BTreeSet<AiRole> = ai
            .map(|a| {
                a.stages
                    .iter()
                    .filter(|s| stages.contains(&s.id))
                    .map(|s| s.role)
                    .collect()
            })
            .unwrap_or_default();
        let mut svc: BTreeSet<String> = d
            .files
            .iter()
            .filter_map(|f| service_of(f).map(|s| s.id.clone()))
            .collect();
        if !d.tables.is_empty()
            && let Some(s) = only(ServiceKind::Database)
        {
            svc.insert(s);
        }
        if (roles.contains(&AiRole::VectorStore) || roles.contains(&AiRole::Retriever))
            && let Some(s) = only(ServiceKind::VectorStore)
        {
            svc.insert(s);
        }
        if roles.contains(&AiRole::Llm)
            && let Some(s) = only(ServiceKind::ModelServer)
        {
            svc.insert(s);
        }
        let redis_mem = mems.iter().any(|m| {
            m.engine == "Redis"
                && d.fns
                    .iter()
                    .any(|f| file_of(f).as_deref() == Some(m.file_id.as_str()))
        });
        if redis_mem && let Some(s) = only(ServiceKind::Cache) {
            svc.insert(s);
        }
        // Frontend boxes the screens live in, and the proxy in front of the API.
        if !d.ui.is_empty()
            && let Some(s) = only(ServiceKind::Proxy)
        {
            svc.insert(s);
        }

        let mut counts: BTreeMap<String, u32> = BTreeMap::new();
        for f in &d.files {
            let path = f.strip_prefix("file:").unwrap_or(f);
            for p in externals
                .get(path)
                .into_iter()
                .flatten()
                .filter(|p| !is_builtin(p))
            {
                let key = p.replace('-', "_");
                let name = pretty
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| p.replace('_', "-"));
                *counts.entry(name).or_default() += 1;
            }
        }
        for fw in &d.frameworks {
            let name = match fw.as_str() {
                "fastapi" => "FastAPI",
                "express" => "Express",
                "flask" => "Flask",
                "django" => "Django",
                "nestjs" => "NestJS",
                "nextjs" | "next" => "Next.js",
                "gin" => "Gin",
                "axum" => "axum",
                "actix" => "actix-web",
                other => other,
            };
            *counts.entry(name.to_owned()).or_default() += 10;
        }
        let mut stack: Vec<(String, u32)> = counts.into_iter().collect();
        stack.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let stack: Vec<String> = stack.into_iter().take(10).map(|x| x.0).collect();

        let screens: BTreeSet<&str> = g
            .ui_elements
            .iter()
            .filter(|u| d.ui.contains(&u.id))
            .map(|u| u.file_id.as_str())
            .collect();
        let table_names: Vec<String> = d
            .tables
            .iter()
            .map(|t| t.rsplit('#').next().unwrap_or(t).to_owned())
            .collect();
        let ipc_only = d.routes.iter().all(|r| r.starts_with("route:IPC "));
        let mut parts = vec![if ipc_only {
            plural(d.routes.len(), "native command", "native commands")
        } else {
            plural(d.routes.len(), "route", "routes")
        }];
        if !screens.is_empty() {
            parts.push(plural(screens.len(), "screen", "screens"));
        }
        if !table_names.is_empty() {
            parts.push(format!(
                "{} ({})",
                plural(table_names.len(), "table", "tables"),
                table_names.join(", ")
            ));
        }
        if !stages.is_empty() {
            parts.push(plural(stages.len(), "AI step", "AI steps"));
        }
        let prefix = if ipc_only {
            let names: Vec<&str> = d
                .routes
                .iter()
                .filter_map(|r| r.strip_prefix("route:IPC ")?.split('#').next())
                .collect();
            let shown = names.iter().take(3).copied().collect::<Vec<_>>().join(", ");
            let more = names.len().saturating_sub(3);
            if more > 0 {
                format!("; the window calls {shown} and {more} more")
            } else {
                format!("; the window calls {shown}")
            }
        } else if d.key == "home" {
            "at the root URL".to_owned()
        } else if d.key == "health" {
            "for health and docs".to_owned()
        } else {
            format!("under /{}", d.key)
        };
        out.push(Feature {
            id: format!("feature:{}", d.key),
            name: title(&d.key),
            explain: if ipc_only {
                format!("{}{prefix}.", parts.join(", "))
            } else {
                format!("{} {prefix}.", parts.join(", "))
            },
            basis: "routes".into(),
            routes: d.routes,
            ui: d.ui,
            files: own,
            shared_files: shared,
            tables: d.tables.into_iter().collect(),
            services: svc.into_iter().collect(),
            ai_stages: stages,
            stack,
        });
    }
    // Biggest first; ties by name for stable output.
    out.sort_by(|a, b| {
        (b.routes.len() + b.files.len())
            .cmp(&(a.routes.len() + a.files.len()))
            .then_with(|| a.name.cmp(&b.name))
    });
    out
}
