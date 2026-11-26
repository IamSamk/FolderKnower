//! `ark-parse`: tree-sitter extraction of [`FileFacts`] for TS/TSX/JS, Python, Rust and Go.
//!
//! Extraction is purely syntactic and deterministic. A malformed file yields partial
//! facts plus a diagnostic, never a panic.

mod enrich;
mod facts;
mod go;
mod js;
mod python;
mod rust;
mod util;

pub use facts::*;

use ark_scan::Language;
use tree_sitter::Parser;

/// Bump when extraction output changes, so cached facts are invalidated.
pub const EXTRACTOR_VERSION: &str = "5";

fn ts_language(lang: Language) -> tree_sitter::Language {
    match lang {
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
    }
}

/// Grammar crate versions, recorded in the graph's `generator.grammars`.
pub fn grammar_versions() -> Vec<(&'static str, &'static str)> {
    vec![
        ("tree-sitter-go", "0.25.0"),
        ("tree-sitter-javascript", "0.25.0"),
        ("tree-sitter-python", "0.25.0"),
        ("tree-sitter-rust", "0.24.2"),
        ("tree-sitter-typescript", "0.23.2"),
    ]
}

/// Parse one file's source into facts.
pub fn parse_file(lang: Language, path: &str, src: &str) -> FileFacts {
    let mut facts = FileFacts {
        lines: src.lines().count() as u32,
        ..FileFacts::default()
    };
    let mut parser = Parser::new();
    if parser.set_language(&ts_language(lang)).is_err() {
        facts
            .diagnostics
            .push("grammar version mismatch".to_owned());
        return facts;
    }
    let Some(tree) = parser.parse(src, None) else {
        facts.diagnostics.push("parser returned no tree".to_owned());
        return facts;
    };
    let root = tree.root_node();
    if root.has_error() {
        facts
            .diagnostics
            .push("syntax errors present; facts may be partial".to_owned());
        collect_syntax_errors(root, src.as_bytes(), &mut facts.syntax_errors);
        facts
            .syntax_errors
            .retain(|e| !grammar_gap(lang, src, e.line, &e.text));
    }
    let bytes = src.as_bytes();
    match lang {
        Language::TypeScript | Language::Tsx | Language::JavaScript => {
            js::extract(lang, root, bytes, path, &mut facts)
        }
        Language::Python => python::extract(root, bytes, &mut facts),
        Language::Rust => rust::extract(root, bytes, &mut facts),
        Language::Go => go::extract(root, bytes, &mut facts),
    }
    enrich::enrich(lang, root, bytes, &mut facts);
    facts.finish();
    facts
}

/// Valid code the pinned grammars do not parse yet. Reporting these would tell users
/// their working code is broken.
fn grammar_gap(lang: Language, src: &str, line: u32, text: &str) -> bool {
    if !matches!(
        lang,
        Language::TypeScript | Language::Tsx | Language::JavaScript
    ) {
        return false;
    }
    let l = src
        .lines()
        .nth(line as usize - 1)
        .unwrap_or("")
        .trim_start();
    // TS 5.0 `export type * from "./x"`.
    if l.starts_with("export type *") {
        return true;
    }
    // A bare `&` in JSX text, e.g. `<h2>Releases & Installers</h2>`: React renders it as-is.
    lang != Language::TypeScript
        && text.starts_with("cannot parse `&")
        && !text.starts_with("cannot parse `&&")
}

/// First few ERROR / MISSING nodes, outermost first, one per line.
fn collect_syntax_errors(
    root: tree_sitter::Node,
    src: &[u8],
    out: &mut Vec<facts::SyntaxErrorFact>,
) {
    const MAX: usize = 5;
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        if out.len() >= MAX {
            break;
        }
        if n.is_missing() || n.is_error() {
            let line = n.start_position().row as u32 + 1;
            if !out.iter().any(|e| e.line == line) {
                let text = if n.is_missing() {
                    format!("missing {}", n.kind())
                } else {
                    let t = n.utf8_text(src).unwrap_or("").trim();
                    let first = t.lines().next().unwrap_or("").trim();
                    let clip: String = first.chars().take(60).collect();
                    if clip.is_empty() {
                        "unexpected input".to_owned()
                    } else {
                        format!("cannot parse `{clip}`")
                    }
                };
                out.push(facts::SyntaxErrorFact { line, text });
            }
            if n.is_error() {
                // Look inside an error node only for MISSING tokens, which are more precise.
                continue;
            }
        }
        if !n.has_error() {
            continue;
        }
        let mut c = n.walk();
        let kids: Vec<_> = n.children(&mut c).collect();
        stack.extend(kids.into_iter().rev());
    }
    out.sort_by_key(|e| e.line);
}

#[cfg(test)]
mod tests;
