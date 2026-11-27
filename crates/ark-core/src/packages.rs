//! Workspace packages: the folders with their own manifest (`package.json`, `Cargo.toml`,
//! `pyproject.toml`, `go.mod`), which files belong to each, and which package imports which.
//! Monorepos are drawn by package first (`web`, `worker`, `packages/shared`), so the map
//! must be able to say the same.

use std::collections::BTreeMap;
use std::path::Path;

use ark_scan::{OtherKind, ScannedFile};
use ark_schema::{PackageLink, SystemArchitectureGraph, WorkspacePackage};

struct Manifest {
    root: String,
    name: String,
    ecosystem: &'static str,
    /// Dependency names declared in the manifest.
    deps: Vec<String>,
}

fn read_manifest(root: &Path, f: &ScannedFile) -> Option<Manifest> {
    let (dir, file) = f.path.rsplit_once('/').unwrap_or(("", f.path.as_str()));
    let eco = match file {
        "package.json" => "npm",
        "Cargo.toml" => "cargo",
        "pyproject.toml" => "python",
        "go.mod" => "go",
        _ => return None,
    };
    let text = std::fs::read_to_string(root.join(&f.path)).ok()?;
    let mut deps: Vec<String> = Vec::new();
    match eco {
        "npm" => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                for k in ["dependencies", "devDependencies", "peerDependencies"] {
                    if let Some(o) = v.get(k).and_then(|o| o.as_object()) {
                        deps.extend(o.keys().cloned());
                    }
                }
            }
        }
        "cargo" => {
            let mut in_deps = false;
            for l in text.lines() {
                let l = l.trim();
                if let Some(h) = l.strip_prefix('[') {
                    let h = h.trim_end_matches(']');
                    let tail = h.rsplit('.').next().unwrap_or(h);
                    in_deps = h.ends_with("dependencies");
                    if !in_deps && h.contains("dependencies.") {
                        deps.push(tail.to_owned()); // [dependencies.name]
                    }
                } else if in_deps && let Some((k, _)) = l.split_once('=') {
                    deps.push(k.trim().split('.').next().unwrap_or("").to_owned());
                }
            }
        }
        _ => {}
    }
    let name = match eco {
        "npm" => serde_json::from_str::<serde_json::Value>(&text)
            .ok()?
            .get("name")
            .and_then(|n| n.as_str())
            .map(str::to_owned),
        "cargo" => {
            // a workspace-only manifest has no [package] table
            let mut in_pkg = false;
            let mut name = None;
            for l in text.lines() {
                let l = l.trim();
                if l.starts_with('[') {
                    in_pkg = l == "[package]";
                } else if in_pkg && let Some(v) = l.strip_prefix("name") {
                    name = v
                        .trim_start()
                        .strip_prefix('=')
                        .map(|v| v.trim().trim_matches('"').to_owned());
                }
            }
            Some(name?)
        }
        "python" => {
            let mut in_proj = false;
            let mut name = None;
            for l in text.lines() {
                let l = l.trim();
                if l.starts_with('[') {
                    in_proj = l == "[project]" || l == "[tool.poetry]";
                } else if in_proj && let Some(v) = l.strip_prefix("name") {
                    name = v
                        .trim_start()
                        .strip_prefix('=')
                        .map(|v| v.trim().trim_matches('"').to_owned());
                }
            }
            name
        }
        _ => text
            .lines()
            .find_map(|l| l.trim().strip_prefix("module "))
            .map(|m| m.trim().to_owned()),
    };
    let name = name.filter(|n| !n.is_empty()).or_else(|| {
        dir.rsplit('/')
            .next()
            .filter(|d| !d.is_empty())
            .map(str::to_owned)
    })?;
    Some(Manifest {
        root: dir.to_owned(),
        name,
        ecosystem: eco,
        deps,
    })
}

/// Packages with at least one source file, biggest first. Empty unless there are two or more.
pub fn build(
    root: &Path,
    files: &[ScannedFile],
    graph: &SystemArchitectureGraph,
) -> Vec<WorkspacePackage> {
    let mut manifests: Vec<Manifest> = files
        .iter()
        .filter(|f| f.other == Some(OtherKind::Manifest))
        .filter_map(|f| read_manifest(root, f))
        .collect();
    manifests.sort_by(|a, b| (a.root.as_str(), a.ecosystem).cmp(&(b.root.as_str(), b.ecosystem)));
    manifests.dedup_by(|a, b| a.root == b.root && a.ecosystem == b.ecosystem);
    // deepest manifest folder owns a file
    let owner = |path: &str| -> Option<usize> {
        manifests
            .iter()
            .enumerate()
            .filter(|(_, m)| m.root.is_empty() || path.starts_with(&format!("{}/", m.root)))
            .max_by_key(|(_, m)| m.root.len())
            .map(|(i, _)| i)
    };
    let mut pkgs: BTreeMap<usize, WorkspacePackage> = BTreeMap::new();
    let mut file_pkg: BTreeMap<String, usize> = BTreeMap::new();
    for t in &graph.tiers {
        for c in &t.clusters {
            for f in &c.files {
                let Some(i) = owner(&f.path) else { continue };
                file_pkg.insert(f.id.clone(), i);
                let m = &manifests[i];
                let p = pkgs.entry(i).or_insert_with(|| WorkspacePackage {
                    id: format!(
                        "pkg:{}:{}",
                        m.ecosystem,
                        if m.root.is_empty() { "." } else { &m.root }
                    ),
                    name: m.name.clone(),
                    root: m.root.clone(),
                    ecosystem: m.ecosystem.to_owned(),
                    files: 0,
                    lines: 0,
                    tiers: BTreeMap::new(),
                    depends_on: Vec::new(),
                });
                p.files += 1;
                p.lines += f.lines;
                *p.tiers.entry(format!("{:?}", t.label)).or_insert(0) += 1;
            }
        }
    }
    if pkgs.len() < 2 {
        return Vec::new();
    }
    let ids: BTreeMap<usize, String> = pkgs.iter().map(|(i, p)| (*i, p.id.clone())).collect();
    let mut links: BTreeMap<(usize, usize), u32> = BTreeMap::new();
    for t in &graph.tiers {
        for c in &t.clusters {
            for f in &c.files {
                let Some(&from) = file_pkg.get(&f.id) else {
                    continue;
                };
                for e in &f.imports {
                    if let Some(&to) = file_pkg.get(&e.target)
                        && to != from
                    {
                        *links.entry((from, to)).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    // Declared dependencies on other workspace packages count even when no import edge
    // resolved (package-name imports like `@scope/shared` are not file paths).
    let by_name: BTreeMap<(&str, &str), usize> = manifests
        .iter()
        .enumerate()
        .filter(|(i, _)| pkgs.contains_key(i))
        .map(|(i, m)| ((m.ecosystem, m.name.as_str()), i))
        .collect();
    for (i, m) in manifests.iter().enumerate() {
        if !pkgs.contains_key(&i) {
            continue;
        }
        for d in &m.deps {
            if let Some(&to) = by_name.get(&(m.ecosystem, d.as_str()))
                && to != i
            {
                links.entry((i, to)).or_insert(0);
            }
        }
    }
    for ((from, to), n) in links {
        if let (Some(p), Some(target)) = (pkgs.get_mut(&from), ids.get(&to)) {
            p.depends_on.push(PackageLink {
                target: target.clone(),
                imports: n,
            });
        }
    }
    let mut out: Vec<WorkspacePackage> = pkgs.into_values().collect();
    for p in &mut out {
        p.depends_on
            .sort_by(|a, b| b.imports.cmp(&a.imports).then(a.target.cmp(&b.target)));
    }
    out.sort_by(|a, b| b.files.cmp(&a.files).then(a.id.cmp(&b.id)));
    out
}
