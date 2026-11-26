//! `ark-core`: the indexing pipeline.
//!
//! scan -> (cache | parse in parallel) -> link -> classify/cluster -> ontology -> assemble.
//! Output is a [`SystemArchitectureGraph`] whose serialized bytes depend only on the
//! repository contents and the tool/ruleset versions.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

mod features;
mod health;
mod memory;
mod rules;
mod story;

use ark_link::{LinkInputs, ParsedFile, file_id, fn_id, link};
use ark_ontology::Ontology;
use ark_parse::{EXTRACTOR_VERSION, FileFacts, parse_file};
use ark_scan::{Language, OtherKind, ScanOptions, scan};
use ark_schema::{
    FunctionNode, GeneratorInfo, NonCodeKind, NonCodeNode, Parameter, SchemaVersion, Signature,
    SourceFileNode, SubsystemCluster, SystemArchitectureGraph, TierLabel, TierNode,
};
use ark_store::{Store, artifact_key};
use ark_structure::{StructureInput, is_test_path};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

pub use ark_schema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    Scan,
    Parse,
    Link,
    Classify,
    Assemble,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub stage: Stage,
    pub message: String,
    pub done: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Cache file; `None` uses the per-user cache directory.
    pub cache_path: Option<PathBuf>,
    pub no_cache: bool,
    pub overrides: BTreeMap<String, TierLabel>,
    pub scan: ScanOptions,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunStats {
    pub files: usize,
    pub code_files: usize,
    pub parsed: usize,
    pub cached: usize,
    pub millis: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Scan(#[from] ark_scan::ScanError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Per-user cache location for a repository root.
pub fn default_cache_path(root: &Path) -> PathBuf {
    let canon = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let key = blake3::hash(canon.to_string_lossy().as_bytes()).to_hex();
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("folderknower")
        .join(&key[..16])
        .join("index.sqlite")
}

fn grammar_tag(lang: Language) -> &'static str {
    match lang {
        Language::TypeScript | Language::Tsx => "tree-sitter-typescript@0.23.2",
        Language::JavaScript => "tree-sitter-javascript@0.25.0",
        Language::Python => "tree-sitter-python@0.25.0",
        Language::Rust => "tree-sitter-rust@0.24.2",
        Language::Go => "tree-sitter-go@0.25.0",
    }
}

pub fn analyze(
    root: &Path,
    opts: &Options,
    progress: &(dyn Fn(Progress) + Sync),
) -> Result<(SystemArchitectureGraph, RunStats), CoreError> {
    let t0 = Instant::now();
    let say = |stage, message: String, done, total| {
        progress(Progress {
            stage,
            message,
            done,
            total,
        })
    };
    say(Stage::Scan, "Walking the repository".into(), 0, 0);
    let scanned = scan(root, &opts.scan)?;
    let total = scanned.files.len() as u64;
    let code: Vec<_> = scanned
        .files
        .iter()
        .filter(|f| f.language.is_some())
        .cloned()
        .collect();
    say(
        Stage::Scan,
        format!(
            "Found {} files ({} source)",
            scanned.files.len(),
            code.len()
        ),
        total,
        total,
    );

    // ---- parse with cache
    let mut store = if opts.no_cache {
        None
    } else {
        let p = opts
            .cache_path
            .clone()
            .unwrap_or_else(|| default_cache_path(root));
        Store::open(&p).ok()
    };
    let keys: Vec<String> = code
        .iter()
        .map(|f| {
            artifact_key(
                &f.hash,
                grammar_tag(f.language.expect("code")),
                EXTRACTOR_VERSION,
            )
        })
        .collect();
    let mut facts: Vec<Option<FileFacts>> = keys
        .iter()
        .map(|k| {
            store
                .as_ref()
                .and_then(|s| s.get(k).ok().flatten())
                .and_then(|b| serde_json::from_slice(&b).ok())
        })
        .collect();
    let cached = facts.iter().filter(|f| f.is_some()).count();
    let misses: Vec<usize> = (0..code.len()).filter(|&i| facts[i].is_none()).collect();
    say(
        Stage::Parse,
        format!(
            "Parsing {} files ({} unchanged, from cache)",
            misses.len(),
            cached
        ),
        cached as u64,
        code.len() as u64,
    );
    let done = std::sync::atomic::AtomicU64::new(cached as u64);
    let parsed: Vec<(usize, FileFacts)> = misses
        .par_iter()
        .map(|&i| {
            let f = &code[i];
            let src = fs::read(root.join(&f.path)).unwrap_or_default();
            let src = String::from_utf8_lossy(&src);
            let out = parse_file(f.language.expect("code"), &f.path, &src);
            let d = done.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            if d.is_multiple_of(64) {
                progress(Progress {
                    stage: Stage::Parse,
                    message: format!("Parsed {}", f.path),
                    done: d,
                    total: code.len() as u64,
                });
            }
            (i, out)
        })
        .collect();
    if let Some(s) = store.as_mut() {
        let blobs: Vec<(String, Vec<u8>)> = parsed
            .iter()
            .map(|(i, f)| (keys[*i].clone(), serde_json::to_vec(f).unwrap_or_default()))
            .collect();
        let _ = s.put_many(blobs.iter().map(|(k, v)| (k.as_str(), v.as_slice())));
        let _ = s.set_files(
            scanned
                .files
                .iter()
                .map(|f| (f.path.as_str(), f.hash.as_str())),
        );
        let _ = s.retain(&keys);
    }
    let n_parsed = parsed.len();
    for (i, f) in parsed {
        facts[i] = Some(f);
    }
    let all: Vec<ParsedFile> = code
        .into_iter()
        .zip(facts)
        .map(|(scan, facts)| ParsedFile {
            scan,
            facts: facts.unwrap_or_default(),
        })
        .collect();

    // ---- split tests from application code
    let (tests, app): (Vec<ParsedFile>, Vec<ParsedFile>) =
        all.into_iter().partition(|f| is_test_path(&f.scan.path));

    // ---- link
    say(
        Stage::Link,
        "Resolving imports, calls and API routes".into(),
        0,
        0,
    );
    let mut go_modules = Vec::new();
    let mut rust_crates = Vec::new();
    let mut schema_sources = Vec::new();
    for f in &scanned.files {
        let name = f.path.rsplit('/').next().unwrap_or(&f.path);
        if name == "go.mod"
            && let Ok(s) = fs::read_to_string(root.join(&f.path))
            && let Some(m) = s.lines().find_map(|l| l.trim().strip_prefix("module "))
        {
            let dir = f
                .path
                .rsplit_once('/')
                .map(|(d, _)| d.to_owned())
                .unwrap_or_default();
            go_modules.push((dir, m.trim().to_owned()));
        }
        if name == "Cargo.toml"
            && let Ok(s) = fs::read_to_string(root.join(&f.path))
            && let Some(krate) = cargo_lib_name(&s)
        {
            let dir = f
                .path
                .rsplit_once('/')
                .map(|(d, _)| d.to_owned())
                .unwrap_or_default();
            rust_crates.push((krate, dir));
        }
        if matches!(
            f.other,
            Some(OtherKind::Schema) | Some(OtherKind::Migration)
        ) && (name.ends_with(".prisma") || name.to_ascii_lowercase().ends_with(".sql"))
            && let Ok(s) = fs::read_to_string(root.join(&f.path))
        {
            schema_sources.push((f.path.clone(), s));
        }
    }
    go_modules.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.cmp(b)));
    rust_crates.sort();
    let inputs = LinkInputs {
        go_modules,
        rust_crates,
    };
    let linked = link(&app, &inputs);
    let test_linked = link(
        &tests
            .iter()
            .chain(app.iter())
            .map(|f| ParsedFile {
                scan: f.scan.clone(),
                facts: f.facts.clone(),
            })
            .collect::<Vec<_>>(),
        &inputs,
    );
    say(
        Stage::Link,
        format!(
            "Linked {} routes, {} API calls, {} UI handlers",
            linked.routes.len(),
            linked.http_edges.len(),
            linked
                .ui_elements
                .iter()
                .filter(|u| u.handler.is_some())
                .count()
        ),
        0,
        0,
    );

    // ---- classify
    say(Stage::Classify, "Deciding what each file does".into(), 0, 0);
    let structure = ark_structure::build(&StructureInput {
        files: &app,
        linked: &linked,
        schema_sources: &schema_sources,
        overrides: &opts.overrides,
    });
    say(Stage::Classify, structure.summary.headline.clone(), 0, 0);

    // ---- assemble
    say(Stage::Assemble, "Building the graph".into(), 0, 0);
    let ontology = Ontology::load();
    let by_path: BTreeMap<&str, &ParsedFile> =
        app.iter().map(|f| (f.scan.path.as_str(), f)).collect();
    let build_file = |path: &str| -> SourceFileNode {
        let f = by_path[path];
        let ft = &structure.file_tiers[path];
        let functions = f
            .facts
            .functions
            .iter()
            .map(|x| {
                let id = fn_id(path, &x.qualname);
                FunctionNode {
                    name: x.name.clone(),
                    visibility: x.visibility,
                    line_range: (x.line_start, x.line_end),
                    byte_range: (x.byte_start, x.byte_end),
                    signature: Signature {
                        parameters: x
                            .params
                            .iter()
                            .map(|p| Parameter {
                                name: p.name.clone(),
                                ty: p.ty.clone(),
                                default_value: p.default.clone(),
                            })
                            .collect(),
                        return_type: x.return_type.clone(),
                    },
                    complexity: x.complexity.clone(),
                    ontology_tags: ontology.tag(
                        x.calls
                            .iter()
                            .map(|c| (c.receiver.as_deref(), c.callee.as_str())),
                    ),
                    inferred_role: None,
                    live_telemetry: None,
                    flow: (!x.flow.is_empty()).then(|| x.flow.clone()),
                    calls: linked.fn_calls.get(&id).cloned().unwrap_or_default(),
                    called_by: linked
                        .called_by
                        .get(&id)
                        .map(|s| s.iter().cloned().collect())
                        .unwrap_or_default(),
                    id,
                }
            })
            .collect();
        SourceFileNode {
            id: file_id(path),
            path: path.to_owned(),
            language: f
                .scan
                .language
                .map(|l| l.name())
                .unwrap_or("unknown")
                .to_owned(),
            content_hash: f.scan.hash.clone(),
            lines: f.facts.lines,
            tier_confidence: ft.confidence,
            role: ft.role.clone(),
            imports: linked.file_imports.get(path).cloned().unwrap_or_default(),
            functions,
            packages: linked
                .externals
                .get(path)
                .into_iter()
                .flatten()
                .filter(|p| !health::is_stdlib(p))
                .cloned()
                .collect(),
        }
    };
    let tiers: Vec<TierNode> = structure
        .tiers
        .iter()
        .map(|t| TierNode {
            id: format!("tier:{}", format!("{:?}", t.tier).to_ascii_lowercase()),
            label: t.tier,
            evidence: t.evidence.clone(),
            clusters: t
                .clusters
                .iter()
                .map(|c| SubsystemCluster {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    method: c.method.clone(),
                    files: c.files.iter().map(|p| build_file(p)).collect(),
                })
                .collect(),
        })
        .collect();

    let mut non_code: Vec<NonCodeNode> = Vec::new();
    for t in &tests {
        let relates: BTreeSet<String> = test_linked
            .file_imports
            .get(&t.scan.path)
            .into_iter()
            .flatten()
            .filter(|e| !e.target.strip_prefix("file:").is_some_and(is_test_path))
            .map(|e| e.target.clone())
            .collect();
        non_code.push(NonCodeNode {
            id: format!("noncode:{}", t.scan.path),
            path: t.scan.path.clone(),
            kind: NonCodeKind::Test,
            relates_to: relates.into_iter().collect(),
        });
    }
    for f in &scanned.files {
        let Some(o) = f.other else { continue };
        let kind = match o {
            OtherKind::Config => NonCodeKind::Config,
            OtherKind::Doc => NonCodeKind::Doc,
            OtherKind::Migration => NonCodeKind::Migration,
            OtherKind::Infra => NonCodeKind::Infra,
            OtherKind::Manifest => NonCodeKind::Manifest,
            OtherKind::Schema => NonCodeKind::Schema,
        };
        let relates_to = structure
            .data_models
            .iter()
            .filter(|m| m.file_id == file_id(&f.path))
            .map(|m| m.id.clone())
            .collect();
        non_code.push(NonCodeNode {
            id: format!("noncode:{}", f.path),
            path: f.path.clone(),
            kind,
            relates_to,
        });
    }
    non_code.sort_by(|a, b| a.id.cmp(&b.id));

    let mut cross = linked.http_edges.clone();
    cross.extend(structure.db_edges.iter().cloned());
    cross.sort_by(|a, b| a.id.cmp(&b.id));

    let mut ui = linked.ui_elements.clone();
    ui.sort_by(|a, b| a.id.cmp(&b.id));

    // ---- insights: data structures, AI/ML pipeline, libraries, memory techniques
    say(
        Stage::Classify,
        "Recognizing data structures and AI pipelines".into(),
        0,
        0,
    );
    let okf = detect_okf(root, &scanned.files);
    let infra_text = read_infra(root, &scanned.files);
    let infra: Vec<ark_patterns::InfraFile> = infra_text
        .iter()
        .map(|(path, text)| ark_patterns::InfraFile { path, text })
        .collect();
    let code: Vec<ark_patterns::CodeFile> = structure
        .file_tiers
        .iter()
        .map(|(path, ft)| ark_patterns::CodeFile {
            path,
            tier: ft.tier,
        })
        .collect();
    let model_texts = read_model_texts(root, &scanned.files);
    let mentions: Vec<ark_patterns::InfraFile> = infra_text
        .iter()
        .chain(model_texts.iter())
        .map(|(path, text)| ark_patterns::InfraFile { path, text })
        .collect();
    let insights = ark_patterns::analyze(&app, &okf, &infra, &code, &mentions);

    let repo_name = fs::canonicalize(root)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "repository".to_owned());

    let mut graph = SystemArchitectureGraph {
        schema_version: SchemaVersion::V2,
        repo_name,
        root_path: ".".to_owned(),
        generator: GeneratorInfo {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            ruleset_version: format!(
                "ontology {} / tiers {}",
                ontology.version, structure.ruleset_version
            ),
            grammars: ark_parse::grammar_versions()
                .into_iter()
                .map(|(a, b)| (a.to_owned(), b.to_owned()))
                .collect(),
        },
        content_root: scanned.root_hash.clone(),
        tiers,
        cross_tier_edges: cross,
        non_code_nodes: non_code,
        summary: structure.summary,
        data_models: structure.data_models,
        routes: linked.routes.clone(),
        ui_elements: ui,
        questions: structure.questions,
        insights,
        example_runs: Vec::new(),
        features: Vec::new(),
        memory_map: Vec::new(),
        rules: Vec::new(),
        health: None,
    };
    graph.example_runs = story::example_runs(&graph);
    graph.features = features::build(&graph, &linked.externals, &app);
    graph.memory_map = memory::build(&graph, &app);
    graph.rules = rules::extract(root, &scanned.files);
    let services = graph
        .insights
        .as_ref()
        .and_then(|i| i.deployment.as_ref())
        .map(|d| d.services.clone())
        .unwrap_or_default();
    let checked: Vec<&ParsedFile> = app.iter().chain(tests.iter()).collect();
    graph.health = health::check(
        root,
        &scanned.files,
        &checked,
        &linked,
        &services,
        &graph.memory_map,
    );
    story::system_headline(&mut graph);
    let stats = RunStats {
        files: scanned.files.len(),
        code_files: app.len() + tests.len(),
        parsed: n_parsed,
        cached,
        millis: t0.elapsed().as_millis() as u64,
    };
    say(
        Stage::Done,
        format!(
            "Done in {} ms ({} parsed, {} cached)",
            stats.millis, stats.parsed, stats.cached
        ),
        total,
        total,
    );
    Ok((graph, stats))
}

/// Config-like files (settings modules, env files, yaml/toml/json, compose) searched for
/// model ids and full-text SQL. Capped so a huge repo stays quick.
fn read_model_texts(root: &Path, files: &[ark_scan::ScannedFile]) -> Vec<(String, String)> {
    let hit = |p: &str| {
        let l = p.to_ascii_lowercase();
        let name = l.rsplit('/').next().unwrap_or(&l);
        name.starts_with(".env")
            || name.contains("config")
            || name.contains("settings")
            || name.contains("store")
            || name.contains("search")
            || name.contains("embed")
            || name.contains("retriev")
            || name.contains("rank")
            || name.contains("fuse")
            || name.contains("vector")
            || name.ends_with(".yml")
            || name.ends_with(".yaml")
            || name.ends_with(".toml")
            || name.ends_with(".sql")
    };
    files
        .iter()
        .filter(|f| f.size < 400 * 1024 && hit(&f.path))
        .take(400)
        .filter_map(|f| {
            fs::read_to_string(root.join(&f.path))
                .ok()
                .map(|t| (f.path.clone(), t))
        })
        .collect()
}

/// Texts of compose files, Dockerfiles and nginx configs (each capped at 512 KiB).
fn read_infra(root: &Path, files: &[ark_scan::ScannedFile]) -> Vec<(String, String)> {
    use ark_patterns::deployment::{is_compose, is_dockerfile, is_proxy_conf};
    files
        .iter()
        .filter(|f| f.size < 512 * 1024)
        .filter(|f| is_compose(&f.path) || is_dockerfile(&f.path) || is_proxy_conf(&f.path))
        .filter_map(|f| {
            fs::read_to_string(root.join(&f.path))
                .ok()
                .map(|t| (f.path.clone(), t))
        })
        .collect()
}

/// Open Knowledge Format bundles: markdown files whose YAML frontmatter has a `type:`
/// key, inside a folder that also has the reserved `index.md` and `log.md`.
fn detect_okf(root: &Path, files: &[ark_scan::ScannedFile]) -> Vec<String> {
    let mut dirs: BTreeMap<String, (bool, bool, u32)> = BTreeMap::new();
    for f in files {
        if !f.path.to_ascii_lowercase().ends_with(".md") {
            continue;
        }
        let (dir, name) = f.path.rsplit_once('/').unwrap_or(("", f.path.as_str()));
        let e = dirs.entry(dir.to_owned()).or_default();
        match name {
            "index.md" => e.0 = true,
            "log.md" => e.1 = true,
            _ => {
                if e.2 < 3
                    && f.size < 256 * 1024
                    && let Ok(s) = fs::read_to_string(root.join(&f.path))
                    && let Some(rest) = s.strip_prefix("---")
                    && let Some(end) = rest.find("\n---")
                    && rest[..end]
                        .lines()
                        .any(|l| l.trim_start().starts_with("type:"))
                {
                    e.2 += 1;
                }
            }
        }
    }
    dirs.into_iter()
        .filter(|(_, (i, l, typed))| *typed > 0 && (*i || *l))
        .map(|(d, _)| {
            if d.is_empty() {
                "./".to_owned()
            } else {
                format!("{d}/")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;

/// The name other crates `use`: `[lib] name`, else `[package] name` with `-` as `_`.
fn cargo_lib_name(toml: &str) -> Option<String> {
    let mut section = "";
    let (mut pkg, mut lib) = (None, None);
    for l in toml.lines() {
        let l = l.trim();
        if l.starts_with('[') {
            section = l;
            continue;
        }
        let Some(v) = l.strip_prefix("name") else {
            continue;
        };
        let Some(v) = v.trim_start().strip_prefix('=') else {
            continue;
        };
        let v = v.trim().trim_matches('"').to_owned();
        match section {
            "[package]" => pkg = Some(v),
            "[lib]" => lib = Some(v),
            _ => {}
        }
    }
    lib.or(pkg).map(|n| n.replace('-', "_"))
}
