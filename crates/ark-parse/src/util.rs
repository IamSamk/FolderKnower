use tree_sitter::Node;

pub fn text<'a>(n: Node, src: &'a [u8]) -> &'a str {
    n.utf8_text(src).unwrap_or("")
}

pub fn line(n: Node) -> u32 {
    n.start_position().row as u32 + 1
}

/// Collapse whitespace and cap length so stored snippets stay small.
pub fn clip(s: &str, max: usize) -> String {
    let collapsed: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= max {
        collapsed
    } else {
        let mut out: String = collapsed.chars().take(max).collect();
        out.push('…');
        out
    }
}

/// Strip quotes from a string literal; template `${expr}` / `{x}` become `:param`.
pub fn unquote(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let raw = raw
        .trim_start_matches(['r', 'b', 'f', 'u', 'R', 'B', 'F', 'U'])
        .trim();
    let first = raw.chars().next()?;
    if !matches!(first, '"' | '\'' | '`') {
        return None;
    }
    let inner = raw
        .trim_start_matches("\"\"\"")
        .trim_end_matches("\"\"\"")
        .trim_matches(first);
    let mut out = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if (c == '$' && chars.peek() == Some(&'{')) || c == '{' {
            if c == '$' {
                chars.next();
            }
            for d in chars.by_ref() {
                if d == '}' {
                    break;
                }
            }
            out.push_str(":param");
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// Iterate a node's named children.
pub fn named_children(n: Node) -> Vec<Node> {
    let mut c = n.walk();
    n.named_children(&mut c).collect()
}

pub const HTTP_METHODS: &[&str] = &["get", "post", "put", "patch", "delete", "head", "options"];

pub fn is_http_method(s: &str) -> bool {
    HTTP_METHODS.contains(&s.to_ascii_lowercase().as_str())
}
