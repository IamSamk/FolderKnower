//! Copy-ready Markdown digest of a graph: headline, size, tiers, Mermaid diagrams of
//! the tiers, containers and AI pipeline, routes, data models and health. Everything
//! comes from the graph; nothing is guessed.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use ark_schema::SystemArchitectureGraph;

/// Rough tokens per source line, for the size estimate only.
const TOKENS_PER_LINE: u64 = 10;
const MAX_ROWS: usize = 40;

fn name_of<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Mermaid label text: quotes and brackets would end the label early.
fn label(s: &str) -> String {
    s.replace('"', "#quot;")
        .replace(['[', ']', '{', '}', '|'], " ")
        .trim()
        .to_owned()
}

/// Path part of a node id: `file:a/b.py`, `fn:a/b.py#f`, `route:GET /x#a/b.py`.
fn path_of(id: &str) -> &str {
    let rest = id.split_once(':').map_or(id, |(_, r)| r);
    if id.starts_with("route:") {
        return rest.rsplit_once('#').map_or(rest, |(_, p)| p);
    }
    rest.split_once('#').map_or(rest, |(p, _)| p)
}

/// The digest as Markdown.
pub fn markdown(g: &SystemArchitectureGraph) -> String {
    let mut md = String::new();
    let _ = writeln!(md, "# {}\n", g.repo_name);
    let _ = writeln!(md, "{}\n", g.summary.headline);

    // Size.
    let files: Vec<_> = g
        .tiers
        .iter()
        .flat_map(|t| t.clusters.iter().flat_map(|c| &c.files))
        .collect();
    let lines: u64 = files.iter().map(|f| u64::from(f.lines)).sum();
    let functions: usize = files.iter().map(|f| f.functions.len()).sum();
    let _ = writeln!(md, "## Size\n");
    let _ = writeln!(md, "| Source files | Lines | Functions | Tokens (rough) |");
    let _ = writeln!(md, "| ---: | ---: | ---: | ---: |");
    let _ = writeln!(
        md,
        "| {} | {} | {} | ~{} |\n",
        files.len(),
        lines,
        functions,
        lines * TOKENS_PER_LINE
    );

    // Tiers.
    let mut tier_of: BTreeMap<&str, String> = BTreeMap::new();
    let _ = writeln!(md, "## Tiers\n");
    let _ = writeln!(md, "| Tier | Files | Lines | Parts |");
    let _ = writeln!(md, "| --- | ---: | ---: | --- |");
    for t in &g.tiers {
        let name = name_of(&t.label);
        let fs: Vec<_> = t.clusters.iter().flat_map(|c| &c.files).collect();
        if fs.is_empty() {
            continue;
        }
        for f in &fs {
            tier_of.insert(f.path.as_str(), name.clone());
        }
        let parts: Vec<&str> = t.clusters.iter().map(|c| c.name.as_str()).collect();
        let _ = writeln!(
            md,
            "| {name} | {} | {} | {} |",
            fs.len(),
            fs.iter().map(|f| u64::from(f.lines)).sum::<u64>(),
            parts.join(", ")
        );
    }
    md.push('\n');

    // Tier diagram from cross-tier edges.
    let mut links: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for e in &g.cross_tier_edges {
        let (Some(a), Some(b)) = (
            tier_of.get(path_of(&e.source)),
            tier_of.get(path_of(&e.target)),
        ) else {
            continue;
        };
        if a != b {
            *links
                .entry((a.clone(), b.clone(), name_of(&e.protocol)))
                .or_default() += 1;
        }
    }
    if !links.is_empty() {
        let _ = writeln!(md, "```mermaid\nflowchart LR");
        for name in tier_of.values().collect::<std::collections::BTreeSet<_>>() {
            let _ = writeln!(md, "  {name}[\"{name}\"]");
        }
        for ((a, b, p), n) in &links {
            let _ = writeln!(md, "  {a} -->|\"{} x{n}\"| {b}", label(p));
        }
        let _ = writeln!(md, "```\n");
    }

    // Containers.
    if let Some(d) = g.insights.as_ref().and_then(|i| i.deployment.as_ref())
        && !d.services.is_empty()
    {
        let _ = writeln!(md, "## Containers\n");
        let _ = writeln!(md, "```mermaid\nflowchart LR");
        let ids: BTreeMap<&str, String> = d
            .services
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.as_str(), format!("s{i}")))
            .collect();
        for s in &d.services {
            let _ = writeln!(
                md,
                "  {}[\"{} ({})\"]",
                ids[s.id.as_str()],
                label(&s.name),
                name_of(&s.kind)
            );
        }
        for l in &d.links {
            if let (Some(a), Some(b)) = (ids.get(l.from.as_str()), ids.get(l.to.as_str())) {
                let _ = writeln!(md, "  {a} -->|\"{}\"| {b}", label(&name_of(&l.via)));
            }
        }
        let _ = writeln!(md, "```\n");
    }

    // AI pipeline.
    if let Some(ai) = g.insights.as_ref().and_then(|i| i.ai.as_ref())
        && !ai.stages.is_empty()
    {
        let _ = writeln!(md, "## AI pipeline\n");
        let _ = writeln!(md, "{}\n", ai.headline);
        let _ = writeln!(md, "```mermaid\nflowchart TD");
        let ids: BTreeMap<&str, String> = ai
            .stages
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id.as_str(), format!("a{i}")))
            .collect();
        for s in &ai.stages {
            let _ = writeln!(
                md,
                "  {}[\"{}: {}\"]",
                ids[s.id.as_str()],
                name_of(&s.role),
                label(&s.name)
            );
        }
        for e in &ai.edges {
            if let (Some(a), Some(b)) = (ids.get(e.from.as_str()), ids.get(e.to.as_str())) {
                let arrow = if e.loop_back { "-.->" } else { "-->" };
                if e.label.is_empty() {
                    let _ = writeln!(md, "  {a} {arrow} {b}");
                } else {
                    let _ = writeln!(md, "  {a} {arrow}|\"{}\"| {b}", label(&e.label));
                }
            }
        }
        let _ = writeln!(md, "```\n");
        let _ = writeln!(md, "| Step | Role | Where | Settings |");
        let _ = writeln!(md, "| --- | --- | --- | --- |");
        for s in &ai.stages {
            let params: Vec<String> = s.params.iter().map(|(k, v)| format!("{k}={v}")).collect();
            let _ = writeln!(
                md,
                "| {} | {} | `{}:{}` | {} |",
                s.name,
                name_of(&s.role),
                path_of(&s.file_id),
                s.line,
                params.join(", ")
            );
        }
        md.push('\n');
    }

    // Routes.
    if !g.routes.is_empty() {
        let _ = writeln!(md, "## Routes ({})\n", g.routes.len());
        let _ = writeln!(md, "| Method | Path | Where |");
        let _ = writeln!(md, "| --- | --- | --- |");
        for r in g.routes.iter().take(MAX_ROWS) {
            let _ = writeln!(
                md,
                "| {} | `{}` | `{}:{}` |",
                r.method,
                r.path,
                path_of(&r.file_id),
                r.line
            );
        }
        if g.routes.len() > MAX_ROWS {
            let _ = writeln!(md, "\n...and {} more.", g.routes.len() - MAX_ROWS);
        }
        md.push('\n');
    }

    // Data models.
    if !g.data_models.is_empty() {
        let _ = writeln!(md, "## Data models ({})\n", g.data_models.len());
        let _ = writeln!(md, "| Model | Fields | Where |");
        let _ = writeln!(md, "| --- | --- | --- |");
        for m in g.data_models.iter().take(MAX_ROWS) {
            let fields: Vec<&str> = m.fields.iter().map(|f| f.name.as_str()).collect();
            let _ = writeln!(
                md,
                "| {} | {} | `{}:{}` |",
                m.table.as_deref().unwrap_or(&m.name),
                fields.join(", "),
                path_of(&m.file_id),
                m.line
            );
        }
        md.push('\n');
    }

    // Health.
    let _ = writeln!(md, "## Health\n");
    match &g.health {
        Some(h) if !h.issues.is_empty() => {
            for i in h.issues.iter().take(MAX_ROWS) {
                let _ = writeln!(
                    md,
                    "- **{}** `{}:{}`: {}",
                    name_of(&i.severity),
                    path_of(&i.file_id),
                    i.line,
                    i.message
                );
            }
            if h.issues.len() > MAX_ROWS {
                let _ = writeln!(md, "- ...and {} more.", h.issues.len() - MAX_ROWS);
            }
        }
        _ => {
            let _ = writeln!(md, "No problems found.");
        }
    }
    md
}
