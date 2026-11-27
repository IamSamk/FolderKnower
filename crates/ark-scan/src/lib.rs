//! `ark-scan`: deterministic repository walk, blake3 content hashes and a Merkle root.
//!
//! - honours `.gitignore`/`.ignore` plus a built-in vendor/minified denylist;
//! - never follows symlinks;
//! - skips files over a size cap (default 1 MB);
//! - output is sorted by path, so the Merkle root is independent of walk order.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};

/// Directories that are never interesting for architecture.
pub const DENY_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".turbo",
    ".venv",
    "venv",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    ".git",
    ".hg",
    ".idea",
    ".vscode",
    "vendor",
    "coverage",
    ".folderknower",
    "Pods",
    ".gradle",
    // Test inputs: sample projects and recorded data used by tests, not part of the app.
    "fixtures",
    "__fixtures__",
    "testdata",
    "__snapshots__",
    // Code that lives in the repo but is someone else's.
    "third_party",
    "third-party",
    "thirdparty",
    "external-crates",
];

/// Source languages FolderKnower recognises by extension but cannot parse yet.
const UNPARSED_EXTS: &[(&str, &str)] = &[
    ("java", "Java"),
    ("kt", "Kotlin"),
    ("kts", "Kotlin"),
    ("cs", "C#"),
    ("swift", "Swift"),
    ("rb", "Ruby"),
    ("php", "PHP"),
    ("c", "C"),
    ("h", "C"),
    ("cc", "C++"),
    ("cpp", "C++"),
    ("hpp", "C++"),
    ("scala", "Scala"),
    ("dart", "Dart"),
    ("ex", "Elixir"),
    ("exs", "Elixir"),
    ("lua", "Lua"),
    ("zig", "Zig"),
    ("vue", "Vue"),
    ("svelte", "Svelte"),
];

/// Machine-written code: protobuf stubs and the like. Reading it adds noise, the schema
/// it came from is the source of truth.
fn is_generated(name: &str) -> bool {
    name.ends_with(".pb.go")
        || name.ends_with("_pb2.py")
        || name.ends_with("_pb2_grpc.py")
        || name.ends_with("_pb2.pyi")
        || name.ends_with(".pb.ts")
        || name.ends_with(".pb.js")
        || name.ends_with("_grpc.pb.go")
        || name.ends_with(".generated.ts")
        || name.ends_with(".gen.go")
}

/// Source languages the parser understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    TypeScript,
    Tsx,
    JavaScript,
    Python,
    Rust,
    Go,
}

impl Language {
    pub fn from_path(path: &str) -> Option<Self> {
        let ext = path.rsplit('.').next()?.to_ascii_lowercase();
        if path.ends_with(".d.ts") {
            return None;
        }
        Some(match ext.as_str() {
            "ts" | "mts" | "cts" => Self::TypeScript,
            "tsx" => Self::Tsx,
            "js" | "mjs" | "cjs" | "jsx" => Self::JavaScript,
            "py" | "pyi" => Self::Python,
            "rs" => Self::Rust,
            "go" => Self::Go,
            _ => return None,
        })
    }

    /// Name used in the schema (`language` field).
    pub fn name(self) -> &'static str {
        match self {
            Self::TypeScript | Self::Tsx => "typescript",
            Self::JavaScript => "javascript",
            Self::Python => "python",
            Self::Rust => "rust",
            Self::Go => "go",
        }
    }
}

/// What a non-code file is, if it matters at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OtherKind {
    Config,
    Doc,
    Migration,
    Infra,
    Manifest,
    Schema,
}

impl OtherKind {
    pub fn from_path(path: &str) -> Option<Self> {
        let lower = path.to_ascii_lowercase();
        let name = lower.rsplit('/').next().unwrap_or(&lower);
        if matches!(
            name,
            "package.json"
                | "cargo.toml"
                | "go.mod"
                | "pyproject.toml"
                | "requirements.txt"
                | "pipfile"
                | "setup.py"
                | "setup.cfg"
                | "composer.json"
                | "gemfile"
        ) {
            return Some(Self::Manifest);
        }
        if name.ends_with(".prisma") || name.ends_with(".sql") || name.ends_with(".graphql") {
            return Some(if lower.contains("migration") {
                Self::Migration
            } else {
                Self::Schema
            });
        }
        if name == "dockerfile"
            || name.starts_with("dockerfile.")
            || name.ends_with(".dockerfile")
            || name.starts_with("docker-compose")
            || name.starts_with("compose.")
            || name.ends_with(".tf")
            || lower.contains(".github/workflows/")
            || lower.starts_with("k8s/")
            || lower.contains("/k8s/")
            || ((lower.ends_with(".yaml") || lower.ends_with(".yml"))
                && lower
                    .split('/')
                    .any(|p| matches!(p, "kubernetes" | "kubernetes-manifests" | "kustomize")))
            || name == "vercel.json"
            || name == "netlify.toml"
            || name == "nginx.conf"
            || (name.ends_with(".conf") && lower.contains("nginx"))
            || name == "procfile"
        {
            return Some(Self::Infra);
        }
        if name.ends_with(".md") || name.ends_with(".mdx") || name.ends_with(".rst") {
            return Some(Self::Doc);
        }
        if name.ends_with(".yaml")
            || name.ends_with(".yml")
            || name.ends_with(".toml")
            || name.ends_with(".ini")
            || name.starts_with(".env")
            || (name.ends_with(".json") && !name.ends_with(".lock.json"))
            || name.ends_with(".config.js")
            || name.ends_with(".config.ts")
        {
            return Some(Self::Config);
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedFile {
    /// Forward-slash path relative to the root.
    pub path: String,
    pub size: u64,
    /// blake3 hex of the bytes.
    pub hash: String,
    pub language: Option<Language>,
    pub other: Option<OtherKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub files: Vec<ScannedFile>,
    /// Merkle root over sorted (path, hash) pairs.
    pub root_hash: String,
    /// Files skipped because they exceed the size cap.
    pub skipped_large: Vec<String>,
    /// Source files in languages that are recognised but not parsed yet, by language.
    #[serde(default)]
    pub unparsed: BTreeMap<String, u32>,
}

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub max_file_bytes: u64,
    pub respect_gitignore: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_file_bytes: 1024 * 1024,
            respect_gitignore: true,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("root {0} is not a directory")]
    NotADirectory(PathBuf),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn is_minified(name: &str) -> bool {
    name.contains(".min.") || name.ends_with(".bundle.js") || name.ends_with(".map")
}

fn is_lockfile(name: &str) -> bool {
    matches!(
        name,
        "package-lock.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "cargo.lock"
            | "poetry.lock"
            | "go.sum"
            | "bun.lockb"
            | "uv.lock"
    )
}

/// Walk `root` and hash every relevant file.
pub fn scan(root: &Path, opts: &ScanOptions) -> Result<ScanResult, ScanError> {
    if !root.is_dir() {
        return Err(ScanError::NotADirectory(root.to_path_buf()));
    }
    let walker = WalkBuilder::new(root)
        .follow_links(false)
        .hidden(false)
        .git_ignore(opts.respect_gitignore)
        .git_global(false)
        .git_exclude(opts.respect_gitignore)
        .ignore(opts.respect_gitignore)
        .require_git(false)
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            e.depth() == 0
                || !(e.file_type().is_some_and(|t| t.is_dir())
                    && DENY_DIRS.contains(&name.as_ref()))
        })
        .build();

    let mut files = Vec::new();
    let mut skipped_large = Vec::new();
    let mut unparsed: BTreeMap<String, u32> = BTreeMap::new();
    for entry in walker.flatten() {
        let Some(ft) = entry.file_type() else {
            continue;
        };
        if !ft.is_file() {
            continue; // symlinks and dirs are skipped
        }
        let Ok(rel) = entry.path().strip_prefix(root) else {
            continue;
        };
        let rel = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        let name = rel.rsplit('/').next().unwrap_or(&rel).to_ascii_lowercase();
        if is_minified(&name) || is_lockfile(&name) || is_generated(&name) {
            continue;
        }
        let language = Language::from_path(&rel);
        if language.is_none()
            && let Some(ext) = name.rsplit_once('.').map(|(_, e)| e)
            && let Some((_, lang)) = UNPARSED_EXTS.iter().find(|(e, _)| *e == ext)
            && !rel
                .to_ascii_lowercase()
                .split('/')
                .any(|p| matches!(p, "test" | "tests" | "example" | "examples"))
        {
            *unparsed.entry((*lang).to_owned()).or_insert(0) += 1;
        }
        let other = if language.is_none() {
            OtherKind::from_path(&rel)
        } else {
            None
        };
        if language.is_none() && other.is_none() {
            continue;
        }
        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        if size > opts.max_file_bytes {
            skipped_large.push(rel);
            continue;
        }
        let bytes = fs::read(entry.path())?;
        files.push(ScannedFile {
            path: rel,
            size,
            hash: blake3::hash(&bytes).to_hex().to_string(),
            language,
            other,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    skipped_large.sort();
    let root_hash = merkle_root(files.iter().map(|f| (f.path.as_str(), f.hash.as_str())));
    Ok(ScanResult {
        files,
        root_hash,
        skipped_large,
        unparsed,
    })
}

/// Merkle root over (path, hash) pairs. Pairs are sorted first, so input order is irrelevant.
pub fn merkle_root<'a>(pairs: impl IntoIterator<Item = (&'a str, &'a str)>) -> String {
    let mut leaves: Vec<(&str, &str)> = pairs.into_iter().collect();
    leaves.sort();
    let mut level: Vec<blake3::Hash> = leaves
        .iter()
        .map(|(p, h)| {
            let mut hasher = blake3::Hasher::new();
            hasher.update(b"leaf\0");
            hasher.update(p.as_bytes());
            hasher.update(b"\0");
            hasher.update(h.as_bytes());
            hasher.finalize()
        })
        .collect();
    if level.is_empty() {
        return blake3::hash(b"empty").to_hex().to_string();
    }
    while level.len() > 1 {
        level = level
            .chunks(2)
            .map(|pair| {
                let mut hasher = blake3::Hasher::new();
                hasher.update(b"node\0");
                hasher.update(pair[0].as_bytes());
                if let Some(r) = pair.get(1) {
                    hasher.update(r.as_bytes());
                }
                hasher.finalize()
            })
            .collect();
    }
    level[0].to_hex().to_string()
}

/// Difference between two scans.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeSet {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub removed: Vec<String>,
}

impl ChangeSet {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.removed.is_empty()
    }
}

pub fn diff(old: &ScanResult, new: &ScanResult) -> ChangeSet {
    if old.root_hash == new.root_hash {
        return ChangeSet::default();
    }
    let old_map: BTreeMap<&str, &str> = old
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.hash.as_str()))
        .collect();
    let new_map: BTreeMap<&str, &str> = new
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.hash.as_str()))
        .collect();
    let mut cs = ChangeSet::default();
    for (p, h) in &new_map {
        match old_map.get(p) {
            None => cs.added.push((*p).to_owned()),
            Some(oh) if oh != h => cs.modified.push((*p).to_owned()),
            _ => {}
        }
    }
    for p in old_map.keys() {
        if !new_map.contains_key(p) {
            cs.removed.push((*p).to_owned());
        }
    }
    cs
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn scan_and_diff() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("node_modules/x")).unwrap();
        fs::write(root.join("src/a.ts"), "export const a = 1;").unwrap();
        fs::write(root.join("src/b.py"), "def b(): pass").unwrap();
        fs::write(root.join("node_modules/x/i.js"), "x").unwrap();
        fs::write(root.join("app.min.js"), "x").unwrap();
        fs::write(root.join("README.md"), "# hi").unwrap();
        let s1 = scan(root, &ScanOptions::default()).unwrap();
        let paths: Vec<_> = s1.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, ["README.md", "src/a.ts", "src/b.py"]);

        let s2 = scan(root, &ScanOptions::default()).unwrap();
        assert_eq!(s1.root_hash, s2.root_hash);
        assert!(diff(&s1, &s2).is_empty());

        fs::write(root.join("src/a.ts"), "export const a = 2;").unwrap();
        let s3 = scan(root, &ScanOptions::default()).unwrap();
        let cs = diff(&s1, &s3);
        assert_eq!(cs.modified, ["src/a.ts"]);
        assert!(cs.added.is_empty() && cs.removed.is_empty());
    }

    proptest! {
        #[test]
        fn merkle_root_is_order_independent(mut pairs in prop::collection::vec(("[a-z/]{1,12}", "[0-9a-f]{8}"), 0..40), seed in any::<u64>()) {
            pairs.sort();
            pairs.dedup_by(|a, b| a.0 == b.0);
            let a = merkle_root(pairs.iter().map(|(p, h)| (p.as_str(), h.as_str())));
            let mut shuffled = pairs.clone();
            let n = shuffled.len();
            if n > 1 {
                for i in 0..n {
                    let j = ((seed as usize).wrapping_mul(i + 7)) % n;
                    shuffled.swap(i, j);
                }
            }
            let b = merkle_root(shuffled.iter().map(|(p, h)| (p.as_str(), h.as_str())));
            prop_assert_eq!(a, b);
        }
    }
}
