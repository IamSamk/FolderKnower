//! Import-path resolution per language, external package naming, file-based routes
//! (Next.js) and URL normalisation.

use std::collections::BTreeMap;

use ark_parse::{FileFacts, RouteFact};
use ark_scan::Language;

use crate::{LinkInputs, ParsedFile};

pub struct Resolver<'a> {
    by_path: BTreeMap<&'a str, usize>,
    /// Python dotted module (every suffix) -> file indices.
    py_modules: BTreeMap<String, Vec<usize>>,
    /// Go package dir -> file indices.
    go_dirs: BTreeMap<String, Vec<usize>>,
    go_modules: &'a [(String, String)],
    rust_crates: &'a [(String, String)],
}

fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

fn join(dir: &str, rel: &str) -> String {
    let mut parts: Vec<&str> = if dir.is_empty() {
        Vec::new()
    } else {
        dir.split('/').collect()
    };
    for seg in rel.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

impl<'a> Resolver<'a> {
    pub fn new(files: &'a [ParsedFile], inputs: &'a LinkInputs) -> Self {
        let mut by_path = BTreeMap::new();
        let mut py_modules: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut go_dirs: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, f) in files.iter().enumerate() {
            let p = f.scan.path.as_str();
            by_path.insert(p, i);
            match f.scan.language {
                Some(Language::Python) => {
                    let stem = p.trim_end_matches(".pyi").trim_end_matches(".py");
                    let stem = stem.strip_suffix("/__init__").unwrap_or(stem);
                    let parts: Vec<&str> = stem.split('/').collect();
                    for k in 0..parts.len() {
                        py_modules.entry(parts[k..].join(".")).or_default().push(i);
                    }
                }
                Some(Language::Go) if !p.ends_with("_test.go") => {
                    go_dirs.entry(dir_of(p).to_owned()).or_default().push(i);
                }
                _ => {}
            }
        }
        Self {
            by_path,
            py_modules,
            go_dirs,
            go_modules: &inputs.go_modules,
            rust_crates: &inputs.rust_crates,
        }
    }

    fn file(&self, p: &str) -> Option<usize> {
        self.by_path.get(p).copied()
    }

    pub fn resolve_import(
        &self,
        from: &str,
        lang: Language,
        source: &str,
        names: &[String],
    ) -> Vec<usize> {
        match lang {
            Language::TypeScript | Language::Tsx | Language::JavaScript => {
                self.resolve_js(from, source).into_iter().collect()
            }
            Language::Python => self.resolve_py(from, source, names),
            Language::Rust => self.resolve_rust(from, source).into_iter().collect(),
            Language::Go => self.resolve_go(source),
        }
    }

    fn resolve_js(&self, from: &str, source: &str) -> Option<usize> {
        let bases: Vec<String> = if source.starts_with('.') {
            vec![join(dir_of(from), source)]
        } else if let Some(rest) = source
            .strip_prefix("@/")
            .or_else(|| source.strip_prefix("~/"))
        {
            // tsconfig-style alias: try src/, the app root of `from`, and the repo root.
            let mut v = vec![format!("src/{rest}"), rest.to_owned()];
            if let Some(i) = from.find("/src/") {
                v.insert(0, format!("{}/src/{rest}", &from[..i]));
            }
            v
        } else if source.starts_with('/') {
            vec![source.trim_start_matches('/').to_owned()]
        } else {
            return None;
        };
        const EXTS: &[&str] = &[
            "",
            ".ts",
            ".tsx",
            ".js",
            ".jsx",
            ".mjs",
            ".cjs",
            "/index.ts",
            "/index.tsx",
            "/index.js",
            "/index.jsx",
        ];
        for b in &bases {
            // `./x.js` in TS ESM really means `./x.ts`.
            let stripped = b
                .strip_suffix(".js")
                .or_else(|| b.strip_suffix(".jsx"))
                .unwrap_or(b);
            for base in [b.as_str(), stripped] {
                for e in EXTS {
                    if let Some(i) = self.file(&format!("{base}{e}")) {
                        return Some(i);
                    }
                }
            }
        }
        None
    }

    fn resolve_py(&self, from: &str, source: &str, names: &[String]) -> Vec<usize> {
        let dots = source.chars().take_while(|c| *c == '.').count();
        let module = &source[dots..];
        let mut candidates = Vec::new();
        if dots > 0 {
            let mut dir = dir_of(from).to_owned();
            for _ in 1..dots {
                dir = dir_of(&dir).to_owned();
            }
            let base = if module.is_empty() {
                dir.clone()
            } else {
                join(&dir, &module.replace('.', "/"))
            };
            for p in [format!("{base}.py"), format!("{base}/__init__.py")] {
                if let Some(i) = self.file(&p) {
                    candidates.push(i);
                }
            }
            // `from . import routes` / `from .api import users` -> submodules
            for n in names {
                for p in [format!("{base}/{n}.py"), format!("{base}/{n}/__init__.py")] {
                    if let Some(i) = self.file(&p) {
                        candidates.push(i);
                    }
                }
            }
        } else if !module.is_empty() {
            if let Some(v) = self.py_modules.get(module) {
                // Prefer the shortest path (closest to a source root).
                let mut v = v.clone();
                v.sort();
                if let Some(&i) = v.first() {
                    candidates.push(i);
                }
            }
            for n in names {
                if let Some(v) = self.py_modules.get(&format!("{module}.{n}"))
                    && let Some(&i) = v.first()
                {
                    candidates.push(i);
                }
            }
        }
        candidates.sort();
        candidates.dedup();
        candidates
    }

    fn resolve_rust(&self, from: &str, source: &str) -> Option<usize> {
        let src_root = match from.find("src/") {
            Some(i) => &from[..i + 4],
            None => "",
        };
        if let Some(m) = source.strip_prefix("mod ") {
            let d = dir_of(from);
            let file = from.rsplit('/').next().unwrap_or(from);
            let base = if matches!(file, "mod.rs" | "lib.rs" | "main.rs") {
                d.to_owned()
            } else {
                join(d, file.trim_end_matches(".rs"))
            };
            return self
                .file(&format!("{base}/{m}.rs"))
                .or_else(|| self.file(&format!("{base}/{m}/mod.rs")));
        }
        let segs: Vec<&str> = source.split("::").collect();
        let (base, rest): (String, &[&str]) = match segs.first() {
            Some(&"crate") => (src_root.trim_end_matches('/').to_owned(), &segs[1..]),
            Some(&"super") => (dir_of(dir_of(from)).to_owned(), &segs[1..]),
            Some(&"self") => (dir_of(from).to_owned(), &segs[1..]),
            Some(first) => {
                // Another crate in this workspace: `use ark_core::graph` -> crates/ark-core/src.
                let (_, dir) = self.rust_crates.iter().find(|(n, _)| n == first)?;
                let src = if dir.is_empty() {
                    "src".to_owned()
                } else {
                    format!("{dir}/src")
                };
                let rest = &segs[1..];
                for take in (1..=rest.len()).rev() {
                    let rel = rest[..take].join("/");
                    let rel = rel.trim_matches(['{', '}', '*']);
                    if let Some(i) = self
                        .file(&format!("{src}/{rel}.rs"))
                        .or_else(|| self.file(&format!("{src}/{rel}/mod.rs")))
                    {
                        return Some(i);
                    }
                }
                return self.file(&format!("{src}/lib.rs"));
            }
            None => return None,
        };
        for take in (1..=rest.len()).rev() {
            let rel = rest[..take].join("/");
            let rel = rel.trim_matches(['{', '}', '*']);
            let p = if base.is_empty() {
                rel.to_owned()
            } else {
                format!("{base}/{rel}")
            };
            if let Some(i) = self
                .file(&format!("{p}.rs"))
                .or_else(|| self.file(&format!("{p}/mod.rs")))
            {
                return Some(i);
            }
        }
        None
    }

    fn resolve_go(&self, source: &str) -> Vec<usize> {
        for (dir, module) in self.go_modules {
            if let Some(rest) = source.strip_prefix(module.as_str()) {
                let rel = rest.trim_start_matches('/');
                let d = match (dir.is_empty(), rel.is_empty()) {
                    (true, _) => rel.to_owned(),
                    (false, true) => dir.clone(),
                    (false, false) => format!("{dir}/{rel}"),
                };
                if let Some(v) = self.go_dirs.get(&d) {
                    return v.clone();
                }
            }
        }
        Vec::new()
    }
}

/// Normalised external package name, or None for relative/internal specifiers.
pub fn external_package(lang: Language, source: &str) -> Option<String> {
    let s = source.trim();
    if s.is_empty() || s.starts_with('.') || s.starts_with('/') {
        return None;
    }
    Some(match lang {
        Language::TypeScript | Language::Tsx | Language::JavaScript => {
            let s = s.trim_start_matches("node:");
            if s.starts_with("@/") || s.starts_with("~/") {
                return None;
            }
            let mut parts = s.split('/');
            let first = parts.next()?;
            if first.starts_with('@') {
                format!("{first}/{}", parts.next().unwrap_or(""))
            } else {
                first.to_owned()
            }
        }
        Language::Python => s.split('.').next()?.to_owned(),
        Language::Rust => {
            let s = s.strip_prefix("mod ").map(|_| "").unwrap_or(s);
            let first = s.split("::").next()?;
            if first.is_empty() || matches!(first, "crate" | "self" | "super") {
                return None;
            }
            first.replace('_', "-")
        }
        Language::Go => {
            let parts: Vec<&str> = s.split('/').collect();
            if parts[0].contains('.') {
                parts[..parts.len().min(3)].join("/")
            } else {
                s.to_owned()
            }
        }
    })
}

/// Next.js `app/**/route.ts` and `pages/api/**` routes.
pub fn file_based_routes(path: &str, facts: &FileFacts) -> Vec<RouteFact> {
    let mut out = Vec::new();
    let seg_norm = |s: &str| -> Option<String> {
        if s.starts_with('(') && s.ends_with(')') {
            None // route group
        } else if s.starts_with('[') {
            Some(":param".to_owned())
        } else {
            Some(s.to_owned())
        }
    };
    let file = path.rsplit('/').next().unwrap_or(path);
    let is_js = [".ts", ".tsx", ".js", ".jsx"]
        .iter()
        .any(|e| file.ends_with(e));
    if !is_js {
        return out;
    }
    if file.starts_with("route.")
        && let Some(i) = path.find("app/")
        && (i == 0 || path[..i].ends_with('/'))
    {
        let dir = dir_of(&path[i + 4..]);
        let segs: Vec<String> = dir
            .split('/')
            .filter(|s| !s.is_empty())
            .filter_map(seg_norm)
            .collect();
        let route = format!("/{}", segs.join("/"));
        for f in &facts.functions {
            let m = f.name.as_str();
            if matches!(
                m,
                "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
            ) {
                out.push(RouteFact {
                    method: m.to_owned(),
                    path: route.clone(),
                    framework: "nextjs".to_owned(),
                    line: f.line_start,
                    handler: Some(f.qualname.clone()),
                });
            }
        }
    } else if let Some(i) = path.find("pages/api/")
        && (i == 0 || path[..i].ends_with('/'))
    {
        let rel = &path[i + 6..];
        let rel = rel.rsplit_once('.').map(|(a, _)| a).unwrap_or(rel);
        let rel = rel.strip_suffix("/index").unwrap_or(rel);
        let segs: Vec<String> = rel.split('/').filter_map(seg_norm).collect();
        let handler = facts
            .functions
            .iter()
            .find(|f| f.name == "default" || f.name == "handler")
            .map(|f| f.qualname.clone());
        // `if (req.method !== "POST")` and `switch (req.method)` name the verbs it serves.
        let method = if facts.verbs_checked.is_empty() {
            "ANY".to_owned()
        } else {
            facts.verbs_checked.join("|")
        };
        out.push(RouteFact {
            method,
            path: format!("/{}", segs.join("/")),
            framework: "nextjs".to_owned(),
            line: 1,
            handler,
        });
    }
    out
}

/// `/api/todos/:param?x=1` -> `["api", "todos", "*"]`. Scheme, host and a leading
/// base-URL placeholder are dropped.
pub fn normalize_url_segments(url: &str) -> Vec<String> {
    let mut u = url.trim();
    if let Some(i) = u.find("://") {
        u = &u[i + 3..];
        u = u.find('/').map(|j| &u[j..]).unwrap_or("");
    }
    let u = u.split(['?', '#']).next().unwrap_or("");
    let mut segs: Vec<String> = u
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s.starts_with(':')
                || s.starts_with('{')
                || s.starts_with('<')
                || s.starts_with('[')
                || s.contains(":param")
            {
                "*".to_owned()
            } else {
                s.to_ascii_lowercase()
            }
        })
        .collect();
    while segs.first().is_some_and(|s| s == "*") {
        segs.remove(0);
    }
    segs
}
