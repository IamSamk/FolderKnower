//! Language-generic enrichment that runs after the per-language extractors:
//!
//! - call arity and keyword / option-object arguments (`chunk_size=1000`, `{ k: 4 }`),
//! - the distinct identifiers each function body touches,
//! - a control-flow outline per function (branches, loops, try, early exits) for the
//!   ideal-path view.
//!
//! Everything here is syntactic and deterministic.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ark_scan::Language;
use ark_schema::{FlowKind, FlowNode};
use tree_sitter::Node;

use crate::facts::*;
use crate::util::*;

const IDENT_KINDS: &[&str] = &[
    "identifier",
    "type_identifier",
    "field_identifier",
    "property_identifier",
    "shorthand_property_identifier",
    "package_identifier",
];
const MAX_IDENTS: usize = 160;
const MAX_FLOW_NODES: usize = 120;
const MAX_FLOW_DEPTH: usize = 6;

type ArgInfo = (u32, Vec<(String, String)>);

pub fn enrich(lang: Language, root: Node, src: &[u8], facts: &mut FileFacts) {
    let mut idents: Vec<(u32, &str)> = Vec::new();
    let mut calls: BTreeMap<(u32, String), VecDeque<ArgInfo>> = BTreeMap::new();
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        let k = n.kind();
        if IDENT_KINDS.contains(&k) {
            idents.push((n.start_byte() as u32, text(n, src)));
        } else if k == "string_content" || k == "string_fragment" {
            // Short identifier-like strings: dict keys, model ids ("cache_control", "gpt-4o").
            let t = text(n, src);
            if (2..=48).contains(&t.len())
                && t.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-./:".contains(&b))
            {
                idents.push((n.start_byte() as u32, t));
            }
        }
        if matches!(k, "call" | "call_expression")
            && let Some(callee) = ark_metrics::callee_name(lang, n, src)
        {
            calls
                .entry((line(n), callee.to_owned()))
                .or_default()
                .push_back(call_args(lang, n, src));
        }
        let kids = named_children(n);
        stack.extend(kids.into_iter().rev());
    }
    idents.sort_unstable();

    for f in &mut facts.functions {
        let lo = idents.partition_point(|(b, _)| *b < f.byte_start);
        let hi = idents.partition_point(|(b, _)| *b < f.byte_end);
        let set: BTreeSet<&str> = idents[lo..hi]
            .iter()
            .map(|(_, s)| *s)
            .filter(|s| s.len() <= 48)
            .collect();
        f.idents = set
            .into_iter()
            .take(MAX_IDENTS)
            .map(str::to_owned)
            .collect();

        if let Some(node) =
            root.descendant_for_byte_range(f.byte_start as usize, f.byte_end as usize)
        {
            let body = node.child_by_field_name("body").unwrap_or(node);
            let mut b = FlowBuilder {
                lang,
                src,
                budget: MAX_FLOW_NODES,
            };
            let mut out = Vec::new();
            b.block(body, &mut out, 0);
            f.flow = out;
        }
    }

    let mut assign = |c: &mut CallFact| {
        if let Some(q) = calls.get_mut(&(c.line, c.callee.clone()))
            && let Some((n, kw)) = q.pop_front()
        {
            c.n_args = n;
            c.kwargs = kw;
        }
    };
    for f in &mut facts.functions {
        for c in &mut f.calls {
            assign(c);
        }
    }
    for c in &mut facts.top_calls {
        assign(c);
    }
}

fn call_args(lang: Language, call: Node, src: &[u8]) -> ArgInfo {
    let Some(args) = call.child_by_field_name("arguments") else {
        return (0, Vec::new());
    };
    let items: Vec<Node> = named_children(args)
        .into_iter()
        .filter(|a| !a.kind().contains("comment"))
        .collect();
    let mut kw = Vec::new();
    for a in &items {
        match (lang, a.kind()) {
            (Language::Python, "keyword_argument") => {
                if let (Some(k), Some(v)) = (
                    a.child_by_field_name("name"),
                    a.child_by_field_name("value"),
                ) {
                    kw.push((text(k, src).to_owned(), clip(text(v, src), 64)));
                }
            }
            (Language::JavaScript | Language::TypeScript | Language::Tsx, "object") => {
                for p in named_children(*a) {
                    match p.kind() {
                        "pair" => {
                            if let (Some(k), Some(v)) =
                                (p.child_by_field_name("key"), p.child_by_field_name("value"))
                            {
                                let key = unquote(text(k, src))
                                    .unwrap_or_else(|| text(k, src).to_owned());
                                kw.push((key, clip(text(v, src), 64)));
                            }
                        }
                        "shorthand_property_identifier" => {
                            let t = text(p, src).to_owned();
                            kw.push((t.clone(), t));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    (items.len() as u32, kw)
}

struct FlowBuilder<'a> {
    lang: Language,
    src: &'a [u8],
    budget: usize,
}

fn first_line(s: &str, max: usize) -> String {
    clip(
        s.lines()
            .next()
            .unwrap_or("")
            .trim_end_matches([':', '{'])
            .trim(),
        max,
    )
}

fn is_block(kind: &str) -> bool {
    matches!(
        kind,
        "block"
            | "statement_block"
            | "statement_list"
            | "compound_statement"
            | "switch_body"
            | "match_block"
    )
}

fn is_fn_like(kind: &str) -> bool {
    matches!(
        kind,
        "function_definition"
            | "function_declaration"
            | "function_item"
            | "arrow_function"
            | "function_expression"
            | "method_definition"
            | "func_literal"
            | "closure_expression"
            | "lambda"
            | "class_definition"
            | "class_declaration"
    )
}

fn is_case(kind: &str) -> bool {
    matches!(
        kind,
        "case_clause"
            | "switch_case"
            | "switch_default"
            | "match_arm"
            | "expression_case"
            | "default_case"
            | "type_case"
    )
}

impl FlowBuilder<'_> {
    fn t(&self, n: Node) -> &str {
        text(n, self.src)
    }

    fn push(&mut self, out: &mut Vec<FlowNode>, node: FlowNode) -> bool {
        if self.budget == 0 {
            return false;
        }
        self.budget -= 1;
        out.push(node);
        true
    }

    fn node(&self, kind: FlowKind, n: Node, label: String) -> FlowNode {
        FlowNode {
            kind,
            line: line(n),
            label,
            error: false,
            children: Vec::new(),
        }
    }

    fn block(&mut self, n: Node, out: &mut Vec<FlowNode>, depth: usize) {
        if depth > MAX_FLOW_DEPTH || self.budget == 0 {
            return;
        }
        if is_block(n.kind()) || n.kind() == "else_clause" {
            for c in named_children(n) {
                self.stmt(c, out, depth);
            }
        } else {
            self.stmt(n, out, depth);
        }
    }

    fn children_of(&mut self, n: Option<Node>, depth: usize) -> Vec<FlowNode> {
        let mut v = Vec::new();
        if let Some(n) = n {
            self.block(n, &mut v, depth + 1);
        }
        v
    }

    fn stmt(&mut self, n: Node, out: &mut Vec<FlowNode>, depth: usize) {
        if self.budget == 0 {
            return;
        }
        let k = n.kind();
        if is_fn_like(k) || k.contains("comment") {
            return;
        }
        match k {
            "expression_statement" => {
                // Rust wraps if/match/loops/returns in expression statements.
                if let Some(inner) = named_children(n).into_iter().next()
                    && self.is_control(inner.kind())
                {
                    return self.stmt(inner, out, depth);
                }
                self.simple(n, out);
            }
            "if_statement" | "if_expression" => self.branch(n, out, depth),
            "for_statement" | "for_in_statement" | "while_statement" | "do_statement"
            | "for_expression" | "while_expression" | "loop_expression" => {
                let mut node = self.node(FlowKind::Loop, n, first_line(self.t(n), 60));
                node.children = self.children_of(n.child_by_field_name("body"), depth);
                self.push(out, node);
            }
            "try_statement" => {
                let mut node = self.node(FlowKind::Try, n, "try".to_owned());
                node.children = self.children_of(n.child_by_field_name("body"), depth);
                self.push(out, node);
                for c in named_children(n) {
                    let (kind, label) = match c.kind() {
                        "except_clause" | "catch_clause" | "except_group_clause" => {
                            (FlowKind::Catch, first_line(self.t(c), 60))
                        }
                        "finally_clause" => (FlowKind::Finally, "finally".to_owned()),
                        _ => continue,
                    };
                    let body = c.child_by_field_name("body").or_else(|| {
                        named_children(c)
                            .into_iter()
                            .rev()
                            .find(|x| is_block(x.kind()))
                    });
                    let mut node = self.node(kind, c, label);
                    node.children = self.children_of(body, depth);
                    if !self.push(out, node) {
                        return;
                    }
                }
            }
            "switch_statement"
            | "match_statement"
            | "match_expression"
            | "expression_switch_statement"
            | "type_switch_statement" => {
                let mut node = self.node(FlowKind::Switch, n, first_line(self.t(n), 60));
                let mut cases = Vec::new();
                self.collect_cases(n, &mut cases, depth, 0);
                node.children = cases;
                self.push(out, node);
            }
            "return_statement" | "return_expression" => {
                let t = self.t(n);
                let mut node = self.node(FlowKind::Return, n, first_line(t, 60));
                node.error = is_error_return(t);
                self.push(out, node);
            }
            "raise_statement" | "throw_statement" | "throw_expression" => {
                let mut node = self.node(FlowKind::Raise, n, first_line(self.t(n), 60));
                node.error = true;
                self.push(out, node);
            }
            "with_statement" => {
                // Context managers run their body; show the body inline.
                self.simple(n, out);
                if let Some(b) = n.child_by_field_name("body") {
                    self.block(b, out, depth + 1);
                }
            }
            _ if is_block(k) => self.block(n, out, depth + 1),
            _ => self.simple(n, out),
        }
    }

    fn is_control(&self, kind: &str) -> bool {
        matches!(
            kind,
            "if_expression"
                | "match_expression"
                | "for_expression"
                | "while_expression"
                | "loop_expression"
                | "return_expression"
        )
    }

    fn collect_cases(&mut self, n: Node, out: &mut Vec<FlowNode>, depth: usize, guard: u8) {
        if guard > 2 {
            return;
        }
        for c in named_children(n) {
            if is_case(c.kind()) {
                let mut node = self.node(FlowKind::Case, c, first_line(self.t(c), 48));
                let kids = named_children(c);
                let mut v = Vec::new();
                for (i, s) in kids.iter().enumerate() {
                    // Skip the pattern / value, keep the body statements.
                    let is_pattern = i == 0
                        && !is_block(s.kind())
                        && !s.kind().ends_with("_statement")
                        && kids.len() > 1;
                    if !is_pattern {
                        self.block(*s, &mut v, depth + 1);
                    }
                }
                node.children = v;
                if !self.push(out, node) {
                    return;
                }
            } else if is_block(c.kind()) {
                self.collect_cases(c, out, depth, guard + 1);
            }
        }
    }

    fn branch(&mut self, n: Node, out: &mut Vec<FlowNode>, depth: usize) {
        let cond = n
            .child_by_field_name("condition")
            .map(|c| clip(self.t(c).trim_matches(['(', ')']), 60))
            .unwrap_or_default();
        let mut node = self.node(FlowKind::Branch, n, format!("if {cond}"));
        node.children = self.children_of(n.child_by_field_name("consequence"), depth);
        if !self.push(out, node) {
            return;
        }
        let mut c = n.walk();
        let alts: Vec<Node> = n.children_by_field_name("alternative", &mut c).collect();
        for alt in alts {
            self.alternative(alt, out, depth);
        }
    }

    fn alternative(&mut self, alt: Node, out: &mut Vec<FlowNode>, depth: usize) {
        match alt.kind() {
            "elif_clause" => {
                let cond = alt
                    .child_by_field_name("condition")
                    .map(|c| clip(self.t(c), 60))
                    .unwrap_or_default();
                let mut node = self.node(FlowKind::Else, alt, format!("elif {cond}"));
                node.children = self.children_of(alt.child_by_field_name("consequence"), depth);
                self.push(out, node);
            }
            "if_statement" | "if_expression" => {
                // Go: `else if` is a nested if under `alternative`.
                let before = out.len();
                self.branch(alt, out, depth);
                if let Some(first) = out.get_mut(before) {
                    first.kind = FlowKind::Else;
                    first.label = format!("else {}", first.label);
                }
            }
            "else_clause" => {
                let inner = alt
                    .child_by_field_name("body")
                    .or_else(|| named_children(alt).into_iter().next());
                match inner {
                    Some(i) if matches!(i.kind(), "if_statement" | "if_expression") => {
                        self.alternative(i, out, depth)
                    }
                    _ => {
                        let mut node = self.node(FlowKind::Else, alt, "else".to_owned());
                        node.children = self.children_of(inner, depth);
                        self.push(out, node);
                    }
                }
            }
            _ => {
                let mut node = self.node(FlowKind::Else, alt, "else".to_owned());
                node.children = self.children_of(Some(alt), depth);
                self.push(out, node);
            }
        }
    }

    /// A plain statement: keep it only when it calls something (that is what a student
    /// follows in a flowchart). Rust `?` marks an early error exit.
    fn simple(&mut self, n: Node, out: &mut Vec<FlowNode>) {
        let t = self.t(n);
        if !contains_call(n, 4) {
            return;
        }
        let head = t.trim_start();
        if [
            "panic!",
            "unreachable!",
            "bail!",
            "todo!",
            "unimplemented!",
            "log.Fatal",
            "os.Exit",
            "process.exit",
            "sys.exit",
        ]
        .iter()
        .any(|p| head.starts_with(p))
        {
            let mut node = self.node(FlowKind::Raise, n, first_line(t, 60));
            node.error = true;
            self.push(out, node);
            return;
        }
        let mut node = self.node(FlowKind::Call, n, first_line(t, 60));
        if self.lang == Language::Rust && contains_kind(n, "try_expression", 5) {
            node.label.push_str("  (may return early with ?)");
        }
        self.push(out, node);
    }
}

fn contains_call(n: Node, depth: u8) -> bool {
    if matches!(
        n.kind(),
        "call" | "call_expression" | "macro_invocation" | "new_expression"
    ) {
        return true;
    }
    if depth == 0 || is_fn_like(n.kind()) {
        return false;
    }
    named_children(n)
        .into_iter()
        .any(|c| contains_call(c, depth - 1))
}

fn contains_kind(n: Node, kind: &str, depth: u8) -> bool {
    if n.kind() == kind {
        return true;
    }
    depth > 0
        && named_children(n)
            .into_iter()
            .any(|c| contains_kind(c, kind, depth - 1))
}

fn is_error_return(t: &str) -> bool {
    let t = t.trim();
    t.starts_with("return Err")
        || t.contains("return nil, err")
        || t.contains(", err\n")
        || t.ends_with(", err")
        || t.contains("errors.New")
        || t.contains("fmt.Errorf")
        || t.contains("Promise.reject")
        || t.contains("HTTPException")
        || t.contains("status(4")
        || t.contains("status(5")
}
