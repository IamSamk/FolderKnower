//! `ark-chunk`: cAST-style syntax-aware chunking (split-then-merge).
//!
//! A node whose text exceeds `max_chars` non-whitespace characters is split into its
//! children recursively; adjacent small siblings are then merged greedily up to the
//! budget. Chunks never cut through a syntax node unless a single leaf is too big.

use serde::{Deserialize, Serialize};
use tree_sitter::Node;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunk {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: u32,
    pub end_line: u32,
}

fn weight(src: &[u8], a: usize, b: usize) -> usize {
    src[a..b]
        .iter()
        .filter(|c| !c.is_ascii_whitespace())
        .count()
}

fn split(node: Node, src: &[u8], max: usize, out: &mut Vec<(usize, usize, u32, u32)>) {
    let (a, b) = (node.start_byte(), node.end_byte());
    if weight(src, a, b) <= max || node.named_child_count() == 0 {
        out.push((
            a,
            b,
            node.start_position().row as u32 + 1,
            node.end_position().row as u32 + 1,
        ));
        return;
    }
    let mut c = node.walk();
    for child in node.named_children(&mut c) {
        split(child, src, max, out);
    }
}

/// Chunk a syntax tree rooted at `root`.
pub fn chunk(root: Node, src: &[u8], max_chars: usize) -> Vec<Chunk> {
    let mut pieces = Vec::new();
    split(root, src, max_chars, &mut pieces);
    let mut out: Vec<Chunk> = Vec::new();
    for (a, b, la, lb) in pieces {
        if let Some(last) = out.last_mut()
            && weight(src, last.start_byte, b) <= max_chars
        {
            last.end_byte = b;
            last.end_line = lb;
            continue;
        }
        out.push(Chunk {
            start_byte: a,
            end_byte: b,
            start_line: la,
            end_line: lb,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_small_and_splits_large() {
        let src = "fn a() { 1 }\nfn b() { 2 }\nfn c() { let x = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16]; }\n";
        let mut p = tree_sitter::Parser::new();
        p.set_language(&tree_sitter_rust_lang()).unwrap();
        let t = p.parse(src, None).unwrap();
        let big = chunk(t.root_node(), src.as_bytes(), 10_000);
        assert_eq!(big.len(), 1);
        let small = chunk(t.root_node(), src.as_bytes(), 20);
        assert!(small.len() >= 2);
        for w in small.windows(2) {
            assert!(w[0].end_byte <= w[1].start_byte);
        }
    }

    fn tree_sitter_rust_lang() -> tree_sitter::Language {
        tree_sitter_rust::LANGUAGE.into()
    }
}
