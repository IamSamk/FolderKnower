//! `ark-structure`: semantic (not folder-based) tier classification, subsystem
//! clustering, data models, project summary and clarifying questions.
//!
//! A file's tier comes from what it *does*: the packages it imports, whether it
//! declares routes, renders components, defines tables or runs jobs. Folder names are
//! weak hints. Files with weak evidence borrow from their import neighbours.

mod louvain;
pub mod models;

use std::collections::{BTreeMap, BTreeSet};

use ark_link::{Linked, ParsedFile, file_id};
use ark_schema::{
    ClusterAlgo, ClusterMethod, CrossTierEdge, DataModel, DataModelKind, Finding, ProjectKind,
    ProjectSummary, Question, Stats, TierLabel,
};
use regex::Regex;
use serde::Deserialize;

pub use louvain::louvain;

pub const TIERS_YAML: &str = include_str!("../../../ontology/tiers.yaml");

#[derive(Debug, Deserialize)]
struct TierRuleFile {
    version: String,
    rules: Vec<TierRuleRaw>,
    frameworks: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct TierRuleRaw {
    id: String,
    tier: TierLabel,
    weight: f64,
    #[serde(default)]
    packages: Vec<String>,
    #[serde(default)]
    fact: Option<String>,
    #[serde(default)]
    path: Option<String>,
}

enum Matcher {
    Packages(Vec<String>),
    Fact(String),
    Path(Regex),
}

struct TierRule {
    id: String,
    tier: TierLabel,
    weight: f64,
    matcher: Matcher,
}

pub struct Ruleset {
    pub version: String,
    rules: Vec<TierRule>,
    pub frameworks: BTreeMap<String, String>,
}

impl Ruleset {
    pub fn load() -> Self {
        let f: TierRuleFile = serde_yaml_ng::from_str(TIERS_YAML).expect("valid tiers.yaml");
        let rules = f
            .rules
            .into_iter()
            .map(|r| TierRule {
                matcher: if let Some(p) = r.path {
                    Matcher::Path(Regex::new(&p).expect("tier path regex"))
                } else if let Some(fact) = r.fact {
                    Matcher::Fact(fact)
                } else {
                    Matcher::Packages(r.packages)
                },
                id: r.id,
                tier: r.tier,
                weight: r.weight,
            })
            .collect();
        Self {
            version: f.version,
            rules,
            frameworks: f.frameworks,
        }
    }
}

const ORDER: [TierLabel; 6] = [
    TierLabel::Backend,
    TierLabel::Frontend,
    TierLabel::Storage,
    TierLabel::Gateway,
    TierLabel::Workers,
    TierLabel::Shared,
];

fn ti(t: TierLabel) -> usize {
    ORDER.iter().position(|x| *x == t).expect("tier in ORDER")
}

fn pkg_match(pattern: &str, pkg: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => pkg.starts_with(prefix),
        None => pkg == pattern || pkg.starts_with(&format!("{pattern}/")),
    }
}

pub fn is_test_path(path: &str) -> bool {
    let p = path.to_ascii_lowercase();
    let name = p.rsplit('/').next().unwrap_or(&p);
    p.starts_with("test/")
        || p.starts_with("tests/")
        || p.contains("/test/")
        || p.contains("/tests/")
        || p.contains("/__tests__/")
        || p.contains("/e2e/")
        || name.starts_with("test_")
        || name.ends_with("_test.go")
        || name.ends_with("_test.py")
        || name.contains(".test.")
        || name.contains(".spec.")
        || name == "conftest.py"
}

#[derive(Debug, Clone)]
pub struct FileTier {
    pub tier: TierLabel,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub role: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Cluster {
    pub id: String,
    pub name: String,
    pub method: ClusterMethod,
    pub files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TierOut {
    pub tier: TierLabel,
    pub evidence: Vec<String>,
    pub clusters: Vec<Cluster>,
}

#[derive(Debug)]
pub struct Structure {
    pub file_tiers: BTreeMap<String, FileTier>,
    pub tiers: Vec<TierOut>,
    pub data_models: Vec<DataModel>,
    pub db_edges: Vec<CrossTierEdge>,
    pub summary: ProjectSummary,
    pub questions: Vec<Question>,
    pub ruleset_version: String,
}

pub struct StructureInput<'a> {
    /// Code files (tests excluded), sorted by path.
    pub files: &'a [ParsedFile],
    pub linked: &'a Linked,
    /// Prisma / SQL / other non-code schema sources: (path, contents).
    pub schema_sources: &'a [(String, String)],
    /// Directory prefix (or file path) -> tier, chosen by the user.
    pub overrides: &'a BTreeMap<String, TierLabel>,
}

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn facts_for(f: &ParsedFile, models: &[DataModel]) -> BTreeSet<&'static str> {
    let mut s = BTreeSet::new();
    let fid = file_id(&f.scan.path);
    let facts = &f.facts;
    if facts.functions.iter().any(|x| x.is_component) {
        s.insert("component");
    }
    if !facts.routes.is_empty() {
        s.insert("routes");
    }
    if !facts.http_calls.is_empty() {
        s.insert("http_client");
    }
    if models
        .iter()
        .any(|m| m.file_id == fid && m.kind != DataModelKind::Schema)
    {
        s.insert("tables");
    }
    if models
        .iter()
        .any(|m| m.file_id == fid && m.kind == DataModelKind::Schema)
    {
        s.insert("schemas");
    }
    let mw_call = facts.top_calls.iter().any(|c| {
        (c.callee == "use" || c.callee == "add_middleware" || c.callee == "Use")
            && c.receiver.as_deref().is_some_and(|r| {
                let r = r.to_ascii_lowercase();
                r == "app" || r.contains("router") || r == "server" || r == "e" || r == "r"
            })
    });
    let mw_fn = facts.functions.iter().any(|x| {
        let n = x.name.to_ascii_lowercase();
        n.contains("middleware")
            || n.contains("authenticate")
            || x.decorators.iter().any(|d| d.contains(".middleware"))
    }) || f.scan.path.ends_with("middleware.ts")
        || f.scan.path.ends_with("middleware.js");
    if mw_call || mw_fn {
        s.insert("middleware");
    }
    let task = facts.functions.iter().any(|x| {
        x.decorators.iter().any(|d| {
            d.contains(".task")
                || d.contains("shared_task")
                || d.contains("actor")
                || d.contains("@job")
                || d.contains("scheduled_job")
                || d.contains("@cron")
        })
    }) || facts
        .top_calls
        .iter()
        .chain(facts.functions.iter().flat_map(|x| x.calls.iter()))
        .any(|c| {
            (c.callee == "Worker" && c.first_string.is_some())
                || c.callee == "schedule" && c.receiver.as_deref() == Some("cron")
        });
    if task {
        s.insert("task");
    }
    s
}

fn role_for(f: &ParsedFile, facts: &BTreeSet<&'static str>, tier: TierLabel) -> Option<String> {
    let p = f.scan.path.to_ascii_lowercase();
    let name = p.rsplit('/').next().unwrap_or(&p);
    let stem = name.split('.').next().unwrap_or(name);
    let entry = matches!(
        stem,
        "main" | "index" | "app" | "server" | "__main__" | "manage" | "wsgi" | "asgi" | "lib"
    );
    let r = if facts.contains("routes") {
        "route-handler"
    } else if entry && !facts.contains("component") && !facts.contains("tables") {
        "entrypoint"
    } else if facts.contains("middleware") {
        "middleware"
    } else if facts.contains("task") {
        "worker"
    } else if facts.contains("tables") {
        "orm-models"
    } else if facts.contains("schemas") {
        "schemas"
    } else if facts.contains("component") {
        if p.contains("/pages/")
            || p.contains("/app/") && (stem == "page" || stem == "layout")
            || p.contains("/screens/")
            || p.contains("/views/")
        {
            "page"
        } else {
            "component"
        }
    } else if stem.starts_with("use") && tier == TierLabel::Frontend {
        "hook"
    } else if facts.contains("http_client") {
        "api-client"
    } else if matches!(
        stem,
        "main" | "index" | "app" | "server" | "__main__" | "manage" | "wsgi" | "asgi" | "lib"
    ) {
        "entrypoint"
    } else if p.contains("repositor") || p.contains("/dao") {
        "repository"
    } else if p.contains("service") {
        "service"
    } else if p.contains("type") || name.ends_with(".d.ts") {
        "types"
    } else if p.contains("config") || p.contains("settings") {
        "config"
    } else if p.contains("util") || p.contains("helper") || p.contains("/lib/") {
        "utility"
    } else {
        return None;
    };
    Some(r.to_owned())
}

pub fn build(input: &StructureInput) -> Structure {
    let ruleset = Ruleset::load();
    let files = input.files;
    let linked = input.linked;

    // ---- data models
    let mut raw = models::code_models(files);
    for (path, src) in input.schema_sources {
        if path.ends_with(".prisma") {
            raw.extend(models::prisma_models(path, src));
        } else if path.to_ascii_lowercase().ends_with(".sql") {
            raw.extend(models::sql_models(path, src));
        }
    }
    let data_models = models::finalize(raw);
    let db_edges = models::db_edges(files, &data_models);

    // ---- per-file scores
    let n = files.len();
    let mut scores = vec![[0.0f64; 6]; n];
    let mut evidence: Vec<BTreeSet<String>> = vec![BTreeSet::new(); n];
    let mut fact_sets = Vec::with_capacity(n);
    let empty = BTreeSet::new();
    for (i, f) in files.iter().enumerate() {
        let facts = facts_for(f, &data_models);
        let pkgs = linked.externals.get(&f.scan.path).unwrap_or(&empty);
        for r in &ruleset.rules {
            let fired = match &r.matcher {
                Matcher::Packages(ps) => ps.iter().any(|p| pkgs.iter().any(|k| pkg_match(p, k))),
                Matcher::Fact(fa) => facts.contains(fa.as_str()),
                Matcher::Path(re) => re.is_match(&f.scan.path),
            };
            if fired {
                scores[i][ti(r.tier)] += r.weight;
                evidence[i].insert(r.id.clone());
            }
        }
        // A file that only imports a frontend framework for types, but declares routes, is backend.
        fact_sets.push(facts);
    }

    // ---- semantic propagation over the import graph (Jacobi, 2 rounds)
    let index: BTreeMap<&str, usize> = files
        .iter()
        .enumerate()
        .map(|(i, f)| (f.scan.path.as_str(), i))
        .collect();
    let mut neighbours: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
    let mut importers: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
    for (i, f) in files.iter().enumerate() {
        for e in linked.file_imports.get(&f.scan.path).into_iter().flatten() {
            if let Some(&j) = e.target.strip_prefix("file:").and_then(|p| index.get(p)) {
                neighbours[i].insert(j);
                neighbours[j].insert(i);
                importers[j].insert(i);
            }
        }
    }
    let strong = |s: &[f64; 6]| s.iter().cloned().fold(0.0, f64::max) >= 2.0;
    for _ in 0..2 {
        let snapshot = scores.clone();
        for i in 0..n {
            if strong(&snapshot[i]) || neighbours[i].is_empty() {
                continue;
            }
            let mut acc = [0.0f64; 6];
            for &j in &neighbours[i] {
                let total: f64 = snapshot[j].iter().sum();
                if total > 0.0 {
                    for t in 0..6 {
                        acc[t] += snapshot[j][t] / total;
                    }
                }
            }
            let k = neighbours[i].len() as f64;
            let mut any = false;
            for t in 0..6 {
                let add = 1.5 * acc[t] / k;
                if add > 0.0 {
                    any = true;
                }
                scores[i][t] = snapshot[i][t] + add;
            }
            if any {
                evidence[i].insert("tier.propagated".to_owned());
            }
        }
    }

    let argmax = |s: &[f64; 6]| -> (usize, f64, f64) {
        let mut best = 5; // Shared when there is no evidence at all
        let mut top = 0.0;
        for t in 0..6 {
            if s[t] > top + 1e-9 {
                top = s[t];
                best = t;
            }
        }
        let second = s
            .iter()
            .enumerate()
            .filter(|(t, _)| *t != best)
            .map(|(_, v)| *v)
            .fold(0.0, f64::max);
        (best, top, second)
    };

    let mut file_tiers: BTreeMap<String, FileTier> = BTreeMap::new();
    let mut assigned = vec![5usize; n];
    for i in 0..n {
        let (best, top, second) = argmax(&scores[i]);
        assigned[i] = best;
        let conf = if top <= 0.0 {
            0.3
        } else {
            let margin = (top - second) / top;
            (0.35 + 0.45 * margin + 0.2 * (top / 4.0).min(1.0)).min(0.99)
        };
        file_tiers.insert(
            files[i].scan.path.clone(),
            FileTier {
                tier: ORDER[best],
                confidence: round2(conf),
                evidence: evidence[i].iter().cloned().collect(),
                role: None,
            },
        );
    }
    // Shared: weakly-classified files imported from 2+ different tiers.
    for i in 0..n {
        let own_top = scores[i].iter().cloned().fold(0.0, f64::max);
        let tiers: BTreeSet<usize> = importers[i].iter().map(|&j| assigned[j]).collect();
        if tiers.len() >= 2
            && own_top < 3.0
            && !fact_sets[i].contains("routes")
            && !fact_sets[i].contains("component")
        {
            let ft = file_tiers.get_mut(&files[i].scan.path).expect("present");
            ft.tier = TierLabel::Shared;
            ft.confidence = round2((0.5 + 0.1 * tiers.len() as f64).min(0.9));
            ft.evidence.push("tier.sh.cross-tier-import".to_owned());
            ft.evidence.sort();
        }
    }
    // User overrides: longest matching prefix wins.
    for (path, ft) in file_tiers.iter_mut() {
        if let Some((_, t)) = input
            .overrides
            .iter()
            .filter(|(p, _)| path == *p || path.starts_with(p.as_str()))
            .max_by_key(|(p, _)| p.len())
        {
            ft.tier = *t;
            ft.confidence = 1.0;
            ft.evidence = vec!["user.override".to_owned()];
        }
    }
    for (i, f) in files.iter().enumerate() {
        let ft = file_tiers.get_mut(&f.scan.path).expect("present");
        ft.role = role_for(f, &fact_sets[i], ft.tier);
    }

    // ---- clusters per tier
    let mut tiers_out = Vec::new();
    for tier in [
        TierLabel::Frontend,
        TierLabel::Gateway,
        TierLabel::Backend,
        TierLabel::Storage,
        TierLabel::Workers,
        TierLabel::Shared,
    ] {
        let members: Vec<usize> = (0..n)
            .filter(|&i| file_tiers[&files[i].scan.path].tier == tier)
            .collect();
        if members.is_empty() {
            continue;
        }
        let mut ev: BTreeSet<String> = BTreeSet::new();
        for &i in &members {
            ev.extend(file_tiers[&files[i].scan.path].evidence.iter().cloned());
        }
        let clusters = cluster_tier(tier, &members, files, linked, &index);
        tiers_out.push(TierOut {
            tier,
            evidence: ev.into_iter().collect(),
            clusters,
        });
    }

    let summary = summarize(
        files,
        linked,
        &file_tiers,
        &data_models,
        &db_edges,
        &ruleset,
    );
    let questions = questions(&file_tiers, input.overrides);

    Structure {
        file_tiers,
        tiers: tiers_out,
        data_models,
        db_edges,
        summary,
        questions,
        ruleset_version: ruleset.version,
    }
}

const GENERIC: &[&str] = &[
    "src",
    "app",
    "apps",
    "lib",
    "libs",
    "internal",
    "pkg",
    "cmd",
    "source",
    "code",
    "main",
    "packages",
    "server",
    "client",
    "web",
    "frontend",
    "backend",
    "api",
    "components",
    "pages",
    "index",
    "mod",
    "__init__",
    "routes",
    "views",
    "utils",
    "services",
    "models",
    "core",
];

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_owned()
}

fn cluster_name(paths: &[&str], sizes: &BTreeMap<&str, usize>) -> String {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for p in paths {
        let segs: Vec<&str> = p.split('/').collect();
        let mut seen = BTreeSet::new();
        for (k, s) in segs.iter().enumerate() {
            let s = if k + 1 == segs.len() {
                s.split('.').next().unwrap_or(s)
            } else {
                s
            };
            if GENERIC.contains(&s.to_ascii_lowercase().as_str())
                || s.starts_with('(')
                || s.starts_with('[')
            {
                continue;
            }
            if seen.insert(s.to_owned()) {
                *counts.entry(s.to_owned()).or_insert(0) += 1;
            }
        }
    }
    // Most shared non-generic segment; prefer directory names (earlier segments) on ties.
    let best = counts.iter().max_by(|a, b| {
        a.1.cmp(b.1)
            .then_with(|| b.0.len().cmp(&a.0.len()))
            .then_with(|| b.0.cmp(a.0))
    });
    match best {
        Some((name, &c)) if c >= 2 || paths.len() == 1 => slug(name),
        // No shared token: name the cluster after its largest file.
        _ if !paths.is_empty() => {
            let lead = paths
                .iter()
                .max_by_key(|p| (sizes.get(**p).copied().unwrap_or(0), std::cmp::Reverse(**p)))
                .expect("non-empty");
            slug(
                lead.rsplit('/')
                    .next()
                    .unwrap_or(lead)
                    .split('.')
                    .next()
                    .unwrap_or(lead),
            )
        }
        Some((name, _)) => slug(name),
        None => slug(
            paths
                .first()
                .and_then(|p| p.rsplit('/').nth(1))
                .unwrap_or("root"),
        ),
    }
}

fn cluster_tier(
    tier: TierLabel,
    members: &[usize],
    files: &[ParsedFile],
    linked: &Linked,
    index: &BTreeMap<&str, usize>,
) -> Vec<Cluster> {
    let local: BTreeMap<usize, usize> = members.iter().enumerate().map(|(k, &i)| (i, k)).collect();
    let mut w: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    let mut add = |a: usize, b: usize, x: f64| {
        if a != b {
            let key = if a < b { (a, b) } else { (b, a) };
            *w.entry(key).or_insert(0.0) += x;
        }
    };
    for &i in members {
        let p = &files[i].scan.path;
        for e in linked.file_imports.get(p).into_iter().flatten() {
            if let Some(j) = e.target.strip_prefix("file:").and_then(|t| index.get(t))
                && let (Some(&a), Some(&b)) = (local.get(&i), local.get(j))
            {
                add(a, b, 1.0);
            }
        }
        for func in &files[i].facts.functions {
            let id = ark_link::fn_id(p, &func.qualname);
            for e in linked.fn_calls.get(&id).into_iter().flatten() {
                let tp = e
                    .target
                    .strip_prefix("fn:")
                    .and_then(|t| t.split_once('#'))
                    .map(|(a, _)| a);
                if let Some(j) = tp.and_then(|t| index.get(t))
                    && let (Some(&a), Some(&b)) = (local.get(&i), local.get(j))
                {
                    add(a, b, 0.25 * e.confidence);
                }
            }
        }
    }
    // Weak same-directory prior: chain neighbours in sorted order.
    let mut by_dir: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (k, &i) in members.iter().enumerate() {
        let p = files[i].scan.path.as_str();
        by_dir
            .entry(p.rsplit_once('/').map(|(d, _)| d).unwrap_or(""))
            .or_default()
            .push(k);
    }
    for v in by_dir.values() {
        for pair in v.windows(2) {
            add(pair[0], pair[1], 0.3);
        }
    }
    let edges: Vec<(usize, usize, f64)> = w.into_iter().map(|((a, b), x)| (a, b, x)).collect();
    let resolution = 1.0;
    let labels = louvain(members.len(), &edges, resolution);
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (k, l) in labels.iter().enumerate() {
        groups.entry(*l).or_default().push(members[k]);
    }
    let tier_slug = slug(&format!("{tier:?}"));
    let mut used: BTreeMap<String, usize> = BTreeMap::new();
    let mut out: Vec<Cluster> = groups
        .into_values()
        .map(|g| {
            let paths: Vec<&str> = g.iter().map(|&i| files[i].scan.path.as_str()).collect();
            let sizes: BTreeMap<&str, usize> = g
                .iter()
                .map(|&i| {
                    (
                        files[i].scan.path.as_str(),
                        files[i].facts.functions.len() * 1000 + files[i].facts.lines as usize,
                    )
                })
                .collect();
            let mut name = cluster_name(&paths, &sizes);
            if name.is_empty() {
                name = "root".to_owned();
            }
            let n = used.entry(name.clone()).or_insert(0);
            *n += 1;
            if *n > 1 {
                name = format!("{name}-{n}");
            }
            let mut file_paths: Vec<String> = paths.iter().map(|s| s.to_string()).collect();
            file_paths.sort();
            let anchors = file_paths
                .iter()
                .filter(|p| {
                    let i = index[p.as_str()];
                    !files[i].facts.routes.is_empty()
                })
                .map(|p| file_id(p))
                .collect();
            Cluster {
                id: format!("cluster:{tier_slug}:{name}"),
                name,
                method: ClusterMethod {
                    algo: ClusterAlgo::Louvain,
                    seed: 0,
                    resolution,
                    anchors,
                },
                files: file_paths,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        b.files
            .len()
            .cmp(&a.files.len())
            .then_with(|| a.id.cmp(&b.id))
    });
    out
}

fn summarize(
    files: &[ParsedFile],
    linked: &Linked,
    tiers: &BTreeMap<String, FileTier>,
    models: &[DataModel],
    db_edges: &[CrossTierEdge],
    ruleset: &Ruleset,
) -> ProjectSummary {
    let mut stats = Stats::default();
    for f in files {
        stats.files += 1;
        stats.functions += f.facts.functions.len() as u32;
        stats.lines += f.facts.lines;
        if let Some(l) = f.scan.language {
            *stats.languages.entry(l.name().to_owned()).or_insert(0) += 1;
        }
    }
    let count = |t: TierLabel| tiers.values().filter(|x| x.tier == t).count();
    let (fe, be, db, wk, gw) = (
        count(TierLabel::Frontend),
        count(TierLabel::Backend),
        count(TierLabel::Storage),
        count(TierLabel::Workers),
        count(TierLabel::Gateway),
    );
    // Frameworks per tier, by file count.
    let mut fw_by_tier: BTreeMap<TierLabel, BTreeMap<String, usize>> = BTreeMap::new();
    for (path, pkgs) in &linked.externals {
        let Some(t) = tiers.get(path) else { continue };
        for p in pkgs {
            if let Some(name) = ruleset.frameworks.get(p) {
                *fw_by_tier
                    .entry(t.tier)
                    .or_default()
                    .entry(name.clone())
                    .or_insert(0) += 1;
            }
        }
    }
    let top_fw = |t: TierLabel, k: usize| -> Vec<String> {
        let mut v: Vec<(String, usize)> = fw_by_tier
            .get(&t)
            .map(|m| m.clone().into_iter().collect())
            .unwrap_or_default();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        v.into_iter().take(k).map(|x| x.0).collect()
    };
    let mut all_fw: BTreeMap<String, usize> = BTreeMap::new();
    for m in fw_by_tier.values() {
        for (k, v) in m {
            *all_fw.entry(k.clone()).or_insert(0) += v;
        }
    }
    let has_cli = linked.externals.values().flatten().any(|p| {
        matches!(
            p.as_str(),
            "clap"
                | "argparse"
                | "click"
                | "typer"
                | "commander"
                | "yargs"
                | "github.com/spf13/cobra"
                | "flag"
        )
    });
    let kind = if fe > 0 && (be > 0 || !linked.routes.is_empty()) {
        ProjectKind::FullStack
    } else if fe > 0 {
        ProjectKind::Frontend
    } else if be > 0 || !linked.routes.is_empty() {
        ProjectKind::Backend
    } else if wk > 0 {
        ProjectKind::Worker
    } else if has_cli {
        ProjectKind::Cli
    } else if stats.files > 0 {
        ProjectKind::Library
    } else {
        ProjectKind::Unknown
    };
    let join = |v: Vec<String>| {
        if v.is_empty() {
            String::new()
        } else {
            format!(" ({})", v.join(", "))
        }
    };
    let tables: Vec<&DataModel> = models
        .iter()
        .filter(|m| m.kind != DataModelKind::Schema)
        .collect();
    let schemas = models.len() - tables.len();
    let headline = match kind {
        ProjectKind::FullStack => format!(
            "Full-stack app: frontend{} talks to a backend{}{}.",
            join(top_fw(TierLabel::Frontend, 2)),
            join(top_fw(TierLabel::Backend, 2)),
            if tables.is_empty() {
                String::new()
            } else {
                format!(
                    " backed by {} table{}{}",
                    tables.len(),
                    if tables.len() == 1 { "" } else { "s" },
                    join(top_fw(TierLabel::Storage, 2))
                )
            }
        ),
        ProjectKind::Frontend => format!(
            "Frontend app{} with {} components.",
            join(top_fw(TierLabel::Frontend, 3)),
            files
                .iter()
                .flat_map(|f| &f.facts.functions)
                .filter(|f| f.is_component)
                .count()
        ),
        ProjectKind::Backend => format!(
            "Backend service{} exposing {} routes.",
            join(top_fw(TierLabel::Backend, 3)),
            linked.routes.len()
        ),
        ProjectKind::Worker => format!("Background worker{}.", join(top_fw(TierLabel::Workers, 2))),
        ProjectKind::Cli => "Command-line tool.".to_owned(),
        ProjectKind::Library => format!("Library with {} modules.", stats.files),
        ProjectKind::Unknown => "No source files recognised.".to_owned(),
    };
    let mut findings = Vec::new();
    let mut say = |text: String, evidence: Vec<String>| findings.push(Finding { text, evidence });
    if fe + be + db + wk + gw > 0 {
        say(
            format!(
                "Sorted {} files by what they do, not where they live: {fe} frontend, {gw} gateway/middleware, {be} backend, {db} data, {wk} worker, {} shared.",
                tiers.len(),
                count(TierLabel::Shared)
            ),
            vec!["tier.*".to_owned()],
        );
    }
    if !linked.routes.is_empty() {
        let mut fws: BTreeSet<&str> = BTreeSet::new();
        for r in &linked.routes {
            fws.insert(&r.framework);
        }
        say(
            format!(
                "Found {} HTTP routes ({}).",
                linked.routes.len(),
                fws.into_iter().collect::<Vec<_>>().join(", ")
            ),
            linked.routes.iter().take(8).map(|r| r.id.clone()).collect(),
        );
    }
    if !linked.http_edges.is_empty() {
        let targets: BTreeSet<&str> = linked
            .http_edges
            .iter()
            .map(|e| e.target.as_str())
            .collect();
        say(
            format!(
                "{} client calls reach {} backend handlers over HTTP.",
                linked.http_edges.len(),
                targets.len()
            ),
            linked
                .http_edges
                .iter()
                .take(8)
                .map(|e| e.id.clone())
                .collect(),
        );
    }
    if !models.is_empty() {
        let mirrors = models
            .iter()
            .filter(|m| m.kind == DataModelKind::Schema && !m.mirrors.is_empty())
            .count();
        say(
            format!(
                "{} tables and {schemas} schemas/DTOs; {mirrors} schemas mirror a table; {} functions touch the database.",
                tables.len(),
                db_edges
                    .iter()
                    .map(|e| &e.source)
                    .collect::<BTreeSet<_>>()
                    .len()
            ),
            models.iter().take(8).map(|m| m.id.clone()).collect(),
        );
    }
    let ui = linked
        .ui_elements
        .iter()
        .filter(|u| u.handler.is_some())
        .count();
    if ui > 0 {
        say(
            format!("{ui} UI elements are wired to handlers you can follow into the code."),
            linked
                .ui_elements
                .iter()
                .filter(|u| u.handler.is_some())
                .take(8)
                .map(|u| u.id.clone())
                .collect(),
        );
    }
    if wk > 0 {
        say(
            format!(
                "{wk} file{} run{} as background workers{}.",
                if wk == 1 { "" } else { "s" },
                if wk == 1 { "s" } else { "" },
                join(top_fw(TierLabel::Workers, 2))
            ),
            vec!["tier.wk.*".to_owned()],
        );
    }
    let mut hot: Vec<(&str, &ark_parse::FnFact)> = files
        .iter()
        .flat_map(|f| {
            f.facts
                .functions
                .iter()
                .map(move |x| (f.scan.path.as_str(), x))
        })
        .collect();
    hot.sort_by(|a, b| {
        b.1.complexity
            .cyclomatic
            .cmp(&a.1.complexity.cyclomatic)
            .then_with(|| a.0.cmp(b.0))
    });
    if let Some((p, f)) = hot.first()
        && f.complexity.cyclomatic >= 10
    {
        say(
            format!(
                "Most complex function: {} (cyclomatic {}, {:?}).",
                f.qualname, f.complexity.cyclomatic, f.complexity.time_big_o
            ),
            vec![ark_link::fn_id(p, &f.qualname)],
        );
    }
    let _ = all_fw;
    ProjectSummary {
        kind,
        headline,
        findings,
        stats,
    }
}

fn questions(
    tiers: &BTreeMap<String, FileTier>,
    overrides: &BTreeMap<String, TierLabel>,
) -> Vec<Question> {
    let mut groups: BTreeMap<String, Vec<&FileTier>> = BTreeMap::new();
    for (p, t) in tiers {
        let segs: Vec<&str> = p.split('/').collect();
        if segs.len() < 2 {
            continue;
        }
        let depth = if segs.len() > 2 && GENERIC.contains(&segs[0]) {
            2
        } else {
            1
        };
        let prefix = format!("{}/", segs[..depth].join("/"));
        groups.entry(prefix).or_default().push(t);
    }
    let mut qs = Vec::new();
    for (prefix, v) in groups {
        if overrides
            .keys()
            .any(|o| prefix.starts_with(o.as_str()) || o.starts_with(&prefix))
        {
            continue;
        }
        let avg: f64 = v.iter().map(|t| t.confidence).sum::<f64>() / v.len() as f64;
        if avg >= 0.55 {
            continue;
        }
        let mut votes: BTreeMap<TierLabel, usize> = BTreeMap::new();
        for t in &v {
            *votes.entry(t.tier).or_insert(0) += 1;
        }
        let mut ranked: Vec<(TierLabel, usize)> = votes.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        let suggested = ranked[0].0;
        let mut options: Vec<TierLabel> = ranked.iter().map(|r| r.0).take(3).collect();
        for extra in [TierLabel::Shared, TierLabel::Workers, TierLabel::Backend] {
            if options.len() >= 3 {
                break;
            }
            if !options.contains(&extra) {
                options.push(extra);
            }
        }
        qs.push(Question {
            id: format!("q:{prefix}"),
            text: format!(
                "I'm only {:.0}% sure `{prefix}` ({} files) belongs to {suggested:?}. What is it?",
                avg * 100.0,
                v.len()
            ),
            subject: prefix,
            options,
            suggested,
            confidence: round2(avg),
        });
    }
    qs.sort_by(|a, b| {
        a.confidence
            .partial_cmp(&b.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    qs.truncate(6);
    qs
}

#[cfg(test)]
mod tests;
