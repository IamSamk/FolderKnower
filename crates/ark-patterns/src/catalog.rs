//! The AI/ML catalog (`ontology/ai-catalog.yaml`), compiled once.

use std::sync::LazyLock;

use ark_schema::AiRole;
use regex::Regex;
use serde::Deserialize;

pub const CATALOG_YAML: &str = include_str!("../../../ontology/ai-catalog.yaml");

#[derive(Deserialize)]
struct RawFile {
    version: String,
    components: Vec<RawComponent>,
    memory: Vec<RawMemory>,
    libraries: Vec<RawLibrary>,
}

#[derive(Deserialize)]
struct RawComponent {
    id: String,
    role: AiRole,
    library: String,
    #[serde(rename = "match")]
    patterns: Vec<String>,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    params: Vec<String>,
    #[serde(default)]
    positional: Option<String>,
    #[serde(default)]
    explain: Option<String>,
}

#[derive(Deserialize)]
struct RawMemory {
    id: String,
    name: String,
    engine: String,
    #[serde(rename = "match", default)]
    patterns: Vec<String>,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    kwarg: Option<String>,
    #[serde(default)]
    ident: Option<String>,
    explain: String,
}

#[derive(Deserialize)]
struct RawLibrary {
    name: String,
    category: String,
    imports: Vec<String>,
    explain: String,
}

pub struct Component {
    pub id: String,
    pub role: AiRole,
    pub library: String,
    pub patterns: Vec<Regex>,
    pub requires: Vec<String>,
    pub params: Vec<String>,
    pub positional: Option<String>,
    pub explain: Option<String>,
}

pub struct Memory {
    pub id: String,
    pub name: String,
    pub engine: String,
    pub patterns: Vec<Regex>,
    pub requires: Vec<String>,
    pub kwarg: Option<Regex>,
    pub ident: Option<String>,
    pub explain: String,
}

pub struct Library {
    pub name: String,
    pub category: String,
    pub imports: Vec<String>,
    pub explain: String,
}

pub struct Catalog {
    pub version: String,
    pub components: Vec<Component>,
    pub memory: Vec<Memory>,
    pub libraries: Vec<Library>,
}

fn compile(ps: &[String], id: &str) -> Vec<Regex> {
    ps.iter()
        .map(|p| Regex::new(p).unwrap_or_else(|e| panic!("catalog {id}: bad regex {p}: {e}")))
        .collect()
}

pub fn load(yaml: &str) -> Result<Catalog, String> {
    let raw: RawFile = serde_yaml_ng::from_str(yaml).map_err(|e| e.to_string())?;
    Ok(Catalog {
        version: raw.version,
        components: raw
            .components
            .into_iter()
            .map(|c| Component {
                patterns: compile(&c.patterns, &c.id),
                id: c.id,
                role: c.role,
                library: c.library,
                requires: c.requires,
                params: c.params,
                positional: c.positional,
                explain: c.explain,
            })
            .collect(),
        memory: raw
            .memory
            .into_iter()
            .map(|m| Memory {
                patterns: compile(&m.patterns, &m.id),
                kwarg: m
                    .kwarg
                    .map(|k| Regex::new(&format!("^{k}$")).expect("catalog kwarg regex")),
                id: m.id,
                name: m.name,
                engine: m.engine,
                requires: m.requires,
                ident: m.ident,
                explain: m.explain,
            })
            .collect(),
        libraries: raw
            .libraries
            .into_iter()
            .map(|l| Library {
                name: l.name,
                category: l.category,
                imports: l.imports,
                explain: l.explain,
            })
            .collect(),
    })
}

pub static CATALOG: LazyLock<Catalog> =
    LazyLock::new(|| load(CATALOG_YAML).expect("ontology/ai-catalog.yaml is valid"));

/// `source` is `prefix`, or `prefix` followed by `.`, `/`, `_` or `-`.
pub fn import_matches(source: &str, prefix: &str) -> bool {
    source == prefix
        || source
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with(['.', '/', '_', '-']))
}
