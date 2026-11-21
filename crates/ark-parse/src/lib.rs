//! `ark-parse`: tree-sitter extraction of [`FileFacts`] for TS/TSX/JS, Python, Rust and Go.
//!
//! Extraction is purely syntactic and deterministic. A malformed file yields partial
//! facts plus a diagnostic, never a panic.

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
pub const EXTRACTOR_VERSION: &str = "2";

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
    facts.finish();
    facts
}

#[cfg(test)]
mod tests;
