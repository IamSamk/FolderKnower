//! `ark-metrics`: native complexity metrics and a rule-based Big-O estimator.
//!
//! Every Big-O label carries the rule ids (`basis`) that produced it, so the UI can
//! say *why*. Nested function definitions are not descended into: they get their
//! own metrics.

use ark_scan::Language;
use ark_schema::{BigO, Complexity};
use tree_sitter::Node;

fn is_function_kind(lang: Language, kind: &str) -> bool {
    match lang {
        Language::TypeScript | Language::Tsx | Language::JavaScript => matches!(
            kind,
            "function_declaration"
                | "function_expression"
                | "arrow_function"
                | "method_definition"
                | "generator_function_declaration"
                | "function"
        ),
        Language::Python => matches!(kind, "function_definition" | "lambda"),
        Language::Rust => matches!(kind, "function_item" | "closure_expression"),
        Language::Go => matches!(
            kind,
            "function_declaration" | "method_declaration" | "func_literal"
        ),
        Language::Java | Language::CSharp => matches!(
            kind,
            "method_declaration" | "constructor_declaration" | "lambda_expression"
        ),
    }
}

fn is_loop(lang: Language, kind: &str) -> bool {
    match lang {
        Language::TypeScript | Language::Tsx | Language::JavaScript => matches!(
            kind,
            "for_statement" | "for_in_statement" | "while_statement" | "do_statement"
        ),
        Language::Python => matches!(kind, "for_statement" | "while_statement" | "for_in_clause"),
        Language::Rust => matches!(
            kind,
            "for_expression" | "while_expression" | "loop_expression"
        ),
        Language::Go => kind == "for_statement",
        Language::Java | Language::CSharp => matches!(
            kind,
            "for_statement"
                | "enhanced_for_statement"
                | "for_each_statement"
                | "while_statement"
                | "do_statement"
        ),
    }
}

/// Branch nodes that add a path (cyclomatic) and nest (cognitive).
fn is_branch(lang: Language, kind: &str) -> bool {
    match lang {
        Language::TypeScript | Language::Tsx | Language::JavaScript => matches!(
            kind,
            "if_statement" | "switch_case" | "catch_clause" | "ternary_expression"
        ),
        Language::Python => matches!(
            kind,
            "if_statement"
                | "elif_clause"
                | "except_clause"
                | "conditional_expression"
                | "case_clause"
        ),
        Language::Rust => matches!(kind, "if_expression" | "match_arm"),
        Language::Go => matches!(
            kind,
            "if_statement" | "expression_case" | "type_case" | "communication_case"
        ),
        Language::Java | Language::CSharp => matches!(
            kind,
            "if_statement"
                | "switch_label"
                | "switch_section"
                | "catch_clause"
                | "conditional_expression"
        ),
    }
}

fn is_logical_op(lang: Language, node: Node, src: &[u8]) -> bool {
    match lang {
        Language::Python => node.kind() == "boolean_operator",
        _ => {
            if node.kind() != "binary_expression" {
                return false;
            }
            node.child_by_field_name("operator")
                .and_then(|op| op.utf8_text(src).ok())
                .is_some_and(|t| matches!(t, "&&" | "||" | "??"))
        }
    }
}

fn is_call(lang: Language, kind: &str) -> bool {
    match lang {
        Language::Python => kind == "call",
        _ => kind == "call_expression",
    }
}

/// Last identifier of the callee of a call node, e.g. `items.sort` -> `sort`.
pub fn callee_name<'a>(_lang: Language, call: Node, src: &'a [u8]) -> Option<&'a str> {
    let f = call.child_by_field_name("function")?;
    let leaf = match f.kind() {
        "member_expression" => f.child_by_field_name("property")?,
        "attribute" => f.child_by_field_name("attribute")?,
        "field_expression" => f.child_by_field_name("field")?,
        "selector_expression" => f.child_by_field_name("field")?,
        "scoped_identifier" => f.child_by_field_name("name")?,
        "generic_function" => return callee_name(_lang, f, src),
        _ => f,
    };
    leaf.utf8_text(src).ok()
}

const SORT_NAMES: &[&str] = &[
    "sort",
    "sorted",
    "sort_by",
    "sort_by_key",
    "sort_unstable",
    "sort_unstable_by",
    "Slice",
    "SliceStable",
    "Sort",
    "Stable",
    "toSorted",
];

const GROW_NAMES: &[&str] = &[
    "push",
    "append",
    "extend",
    "insert",
    "add",
    "set",
    "concat",
    "unshift",
    "push_str",
    "push_back",
    "emplace",
];

#[derive(Default)]
struct Walk {
    cyclomatic: u32,
    cognitive: u32,
    max_nesting: u32,
    max_loop_depth: u32,
    self_calls: u32,
    self_calls_halving: bool,
    sort_calls: u32,
    sort_in_loop: bool,
    grow_in_loop: bool,
    concat_in_loop: bool,
    halving_loop: bool,
}

fn text<'a>(n: Node, src: &'a [u8]) -> &'a str {
    n.utf8_text(src).unwrap_or("")
}

fn looks_halving(t: &str) -> bool {
    let t = t.replace(' ', "");
    t.contains("/2")
        || t.contains("//2")
        || t.contains(">>1")
        || t.contains("*=2")
        || t.contains("/=2")
        || t.contains(">>=1")
        || t.contains("mid")
}

fn walk(
    lang: Language,
    node: Node,
    src: &[u8],
    self_name: &str,
    nesting: u32,
    loop_depth: u32,
    w: &mut Walk,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        let kind = child.kind();
        if is_function_kind(lang, kind) {
            continue; // nested functions are measured on their own
        }
        let mut next_nesting = nesting;
        let mut next_loop = loop_depth;
        if is_loop(lang, kind) {
            w.cyclomatic += 1;
            w.cognitive += 1 + nesting;
            next_nesting += 1;
            next_loop += 1;
            w.max_loop_depth = w.max_loop_depth.max(next_loop);
            // A while/for whose header halves or doubles the index is logarithmic.
            let header_end = child
                .child_by_field_name("body")
                .map(|b| b.start_byte())
                .unwrap_or(child.end_byte());
            let header = &src[child.start_byte()..header_end.max(child.start_byte())];
            let body_text = text(child, src);
            if looks_halving(std::str::from_utf8(header).unwrap_or(""))
                || (kind.contains("while") && looks_halving(body_text))
            {
                w.halving_loop = true;
            }
        } else if is_branch(lang, kind) {
            w.cyclomatic += 1;
            // `else if`/`elif` don't add nesting penalties beyond +1.
            let is_chain =
                kind == "elif_clause" || child.parent().is_some_and(|p| p.kind() == "else_clause");
            w.cognitive += if is_chain { 1 } else { 1 + nesting };
            if !matches!(
                kind,
                "switch_case"
                    | "match_arm"
                    | "case_clause"
                    | "expression_case"
                    | "type_case"
                    | "communication_case"
            ) {
                next_nesting += 1;
            }
        } else if is_logical_op(lang, child, src) {
            w.cyclomatic += 1;
            // Only the first operator of a sequence counts for cognitive complexity.
            if !child.parent().is_some_and(|p| is_logical_op(lang, p, src)) {
                w.cognitive += 1;
            }
        } else if is_call(lang, kind) {
            if let Some(name) = callee_name(lang, child, src) {
                if name == self_name && !self_name.is_empty() {
                    w.self_calls += 1;
                    if let Some(args) = child.child_by_field_name("arguments")
                        && looks_halving(text(args, src))
                    {
                        w.self_calls_halving = true;
                    }
                }
                if SORT_NAMES.contains(&name) {
                    w.sort_calls += 1;
                    if loop_depth > 0 {
                        w.sort_in_loop = true;
                    }
                }
                if loop_depth > 0 && GROW_NAMES.contains(&name) {
                    w.grow_in_loop = true;
                }
            }
        } else if loop_depth > 0
            && matches!(
                kind,
                "augmented_assignment_expression"
                    | "augmented_assignment"
                    | "compound_assignment_expr"
            )
        {
            if text(child, src).contains("+=") {
                w.concat_in_loop = true;
            }
        } else if matches!(
            kind,
            "list_comprehension" | "dictionary_comprehension" | "set_comprehension"
        ) {
            w.grow_in_loop = true;
        }
        w.max_nesting = w.max_nesting.max(next_nesting);
        walk(lang, child, src, self_name, next_nesting, next_loop, w);
    }
}

fn rank(b: BigO) -> u8 {
    match b {
        BigO::Constant => 0,
        BigO::Logarithmic => 1,
        BigO::Linear => 2,
        BigO::Linearithmic => 3,
        BigO::Quadratic => 4,
        BigO::Polynomial => 5,
        BigO::Exponential => 6,
        BigO::Unknown => 7,
    }
}

fn max_o(a: BigO, b: BigO) -> BigO {
    if rank(b) > rank(a) { b } else { a }
}

/// Measure one function body. `self_name` is the function's own short name (for recursion).
pub fn analyze(lang: Language, body: Node, src: &[u8], self_name: &str) -> Complexity {
    let mut w = Walk {
        cyclomatic: 1,
        ..Walk::default()
    };
    walk(lang, body, src, self_name, 0, 0, &mut w);

    let mut basis = Vec::new();
    let mut time = match w.max_loop_depth {
        0 => {
            basis.push("bigo.no-loops".to_owned());
            BigO::Constant
        }
        1 if w.halving_loop => {
            basis.push("bigo.loop.halving".to_owned());
            BigO::Logarithmic
        }
        1 => {
            basis.push("bigo.loop.depth-1".to_owned());
            BigO::Linear
        }
        2 if w.halving_loop => {
            basis.push("bigo.loop.depth-2-halving".to_owned());
            BigO::Linearithmic
        }
        2 => {
            basis.push("bigo.loop.depth-2".to_owned());
            BigO::Quadratic
        }
        _ => {
            basis.push("bigo.loop.depth-3+".to_owned());
            BigO::Polynomial
        }
    };
    if w.sort_calls > 0 {
        basis.push("bigo.lib.sort".to_owned());
        time = max_o(
            time,
            if w.sort_in_loop {
                BigO::Quadratic
            } else {
                BigO::Linearithmic
            },
        );
    }
    if w.self_calls >= 2 && !w.self_calls_halving {
        basis.push("bigo.recursion.fanout".to_owned());
        time = max_o(time, BigO::Exponential);
    } else if w.self_calls >= 2 {
        basis.push("bigo.recursion.divide-and-conquer".to_owned());
        time = max_o(time, BigO::Linearithmic);
    } else if w.self_calls == 1 && w.self_calls_halving {
        basis.push("bigo.recursion.halving".to_owned());
        time = max_o(time, BigO::Logarithmic);
    } else if w.self_calls == 1 {
        basis.push("bigo.recursion.linear".to_owned());
        time = max_o(
            time,
            if w.max_loop_depth > 0 {
                BigO::Quadratic
            } else {
                BigO::Linear
            },
        );
    }
    if basis.len() > 1 {
        basis.retain(|b| b != "bigo.no-loops");
    }

    let mut memory_flags = Vec::new();
    let mut space = BigO::Constant;
    if w.grow_in_loop {
        memory_flags.push("alloc-in-loop".to_owned());
        space = BigO::Linear;
        basis.push("space.alloc-in-loop".to_owned());
    }
    if w.self_calls > 0 {
        memory_flags.push("recursion-stack".to_owned());
        space = max_o(
            space,
            if w.self_calls_halving {
                BigO::Logarithmic
            } else {
                BigO::Linear
            },
        );
        basis.push("space.recursion-stack".to_owned());
    }
    if w.concat_in_loop {
        memory_flags.push("concat-in-loop".to_owned());
    }
    if w.sort_calls > 0 && !matches!(lang, Language::Rust | Language::Go) {
        // sorted()/toSorted() copy; in-place sorts still need O(N) scratch in Timsort.
        space = max_o(space, BigO::Linear);
        basis.push("space.lib.sort".to_owned());
    }

    Complexity {
        cyclomatic: w.cyclomatic,
        cognitive: w.cognitive,
        max_nesting: w.max_nesting,
        time_big_o: time,
        space_big_o: space,
        basis,
        memory_flags,
    }
}
