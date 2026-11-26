//! Limits written as named constants (`MAX_CHARS = 2000`). Read line by line from the
//! source, top-level definitions only, so each rule carries the line that proves it.

use std::{fs, path::Path};

use ark_scan::ScannedFile;
use ark_schema::CodeRule;

const WORDS: &[&str] = &[
    "MAX",
    "MIN",
    "LIMIT",
    "THRESHOLD",
    "SIZE",
    "CHARS",
    "TOKENS",
    "TTL",
    "TIMEOUT",
    "BUDGET",
    "RETRIES",
    "RETRY",
    "FOLD",
    "WINDOW",
    "OVERLAP",
    "TOP_K",
    "TOPK",
    "BATCH",
    "CACHE",
    "CAPACITY",
    "DEPTH",
    "TURNS",
    "HISTORY",
    "CONTEXT",
];
const EXT: &[&str] = &["py", "ts", "tsx", "js", "jsx", "mjs", "rs", "go"];

fn parse(line: &str) -> Option<(&str, &str)> {
    let t = line.trim_end();
    // top level only: no indentation
    if t.starts_with([' ', '\t']) {
        return None;
    }
    let t = t.strip_prefix("export ").unwrap_or(t);
    let t = [
        "const ",
        "pub const ",
        "pub static ",
        "static ",
        "final ",
        "let ",
    ]
    .iter()
    .find_map(|p| t.strip_prefix(p))
    .unwrap_or(t);
    let (lhs, rhs) = t.split_once('=')?;
    let name = lhs.split([':', ' ']).next()?.trim();
    if name.len() < 3
        || !name.chars().next()?.is_ascii_uppercase()
        || !name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    {
        return None;
    }
    let v = rhs.trim().trim_end_matches([';', ',']);
    let v = v.split(['#', '/']).next()?.trim();
    let n = v.replace('_', "");
    if n.is_empty() || n.parse::<f64>().is_err() {
        return None;
    }
    Some((name, v))
}

pub fn extract(root: &Path, files: &[ScannedFile]) -> Vec<CodeRule> {
    let mut out = Vec::new();
    for f in files.iter().filter(|f| f.size < 200 * 1024) {
        let ext = f.path.rsplit('.').next().unwrap_or("");
        if !EXT.contains(&ext) || f.path.contains("test") || f.path.contains("node_modules") {
            continue;
        }
        let Ok(text) = fs::read_to_string(root.join(&f.path)) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            if let Some((name, value)) = parse(line)
                && WORDS.iter().any(|w| name.contains(w))
            {
                out.push(CodeRule {
                    name: name.to_owned(),
                    value: value.to_owned(),
                    meaning: name.to_ascii_lowercase().replace('_', " "),
                    proof: format!("{}:{}", f.path, i + 1),
                });
            }
        }
    }
    out.sort_by(|a, b| a.proof.cmp(&b.proof));
    out.truncate(60);
    out
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn reads_top_level_numeric_constants_only() {
        assert_eq!(
            parse("MAX_SUMMARY_CHARS = 2000  # fold"),
            Some(("MAX_SUMMARY_CHARS", "2000"))
        );
        assert_eq!(parse("export const TOP_K = 5;"), Some(("TOP_K", "5")));
        assert_eq!(
            parse("pub const CHUNK_SIZE: usize = 1_024;"),
            Some(("CHUNK_SIZE", "1_024"))
        );
        assert_eq!(parse("    MAX_X = 3"), None);
        assert_eq!(parse("NAME = \"x\""), None);
        assert_eq!(parse("max_x = 3"), None);
    }
}
