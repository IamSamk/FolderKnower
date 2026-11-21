//! Rust extraction.

use ark_scan::Language;
use ark_schema::Visibility;
use tree_sitter::Node;

use crate::facts::*;
use crate::util::*;

struct X<'a, 'f> {
    src: &'a [u8],
    facts: &'f mut FileFacts,
    fn_stack: Vec<usize>,
    impl_stack: Vec<String>,
}

pub fn extract(root: Node, src: &[u8], facts: &mut FileFacts) {
    let mut x = X {
        src,
        facts,
        fn_stack: Vec::new(),
        impl_stack: Vec::new(),
    };
    x.visit(root);
}

fn attrs_before(node: Node, src: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut prev = node.prev_named_sibling();
    while let Some(p) = prev {
        if p.kind() == "attribute_item" {
            out.push(clip(text(p, src), 160));
            prev = p.prev_named_sibling();
        } else if p.kind() == "line_comment" || p.kind() == "block_comment" {
            prev = p.prev_named_sibling();
        } else {
            break;
        }
    }
    out.reverse();
    out
}

fn is_pub(node: Node) -> bool {
    named_children(node)
        .iter()
        .any(|c| c.kind() == "visibility_modifier")
}

impl X<'_, '_> {
    fn t(&self, n: Node) -> &str {
        text(n, self.src)
    }

    fn visit(&mut self, node: Node) {
        match node.kind() {
            "function_item" => {
                self.function(node);
                return;
            }
            "impl_item" => {
                let ty = node
                    .child_by_field_name("type")
                    .map(|t| clip(self.t(t), 60))
                    .unwrap_or_default();
                self.impl_stack.push(ty);
                for c in named_children(node) {
                    self.visit(c);
                }
                self.impl_stack.pop();
                return;
            }
            "struct_item" => self.strukt(node),
            "use_declaration" => {
                if let Some(arg) = node.child_by_field_name("argument") {
                    let t = self.t(arg).replace(char::is_whitespace, "");
                    let names = t
                        .rsplit("::")
                        .next()
                        .unwrap_or(&t)
                        .trim_matches(['{', '}'])
                        .split(',')
                        .map(|s| s.rsplit(" as ").next().unwrap_or(s).to_owned())
                        .filter(|s| !s.is_empty() && s != "*")
                        .collect();
                    self.facts.imports.push(ImportFact {
                        source: t,
                        names,
                        line: line(node),
                    });
                }
            }
            "mod_item" if node.child_by_field_name("body").is_none() => {
                if let Some(n) = node.child_by_field_name("name") {
                    self.facts.imports.push(ImportFact {
                        source: format!("mod {}", self.t(n)),
                        names: vec![self.t(n).to_owned()],
                        line: line(node),
                    });
                }
            }
            "call_expression" => self.call(node),
            _ => {}
        }
        for c in named_children(node) {
            self.visit(c);
        }
    }

    fn function(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.t(n).to_owned())
            .unwrap_or_default();
        let qualname = match self.impl_stack.last() {
            Some(t) => format!("{t}::{name}"),
            None => name.clone(),
        };
        let visibility = if is_pub(node) {
            Visibility::Public
        } else {
            Visibility::Private
        };
        if visibility == Visibility::Public && self.impl_stack.is_empty() {
            self.facts.exports.push(name.clone());
        }
        let mut params = Vec::new();
        if let Some(p) = node.child_by_field_name("parameters") {
            for c in named_children(p) {
                if c.kind() == "parameter" {
                    params.push(ParamFact {
                        name: c
                            .child_by_field_name("pattern")
                            .map(|n| clip(self.t(n), 40))
                            .unwrap_or_default(),
                        ty: c.child_by_field_name("type").map(|t| clip(self.t(t), 80)),
                        default: None,
                    });
                }
            }
        }
        let return_type = node
            .child_by_field_name("return_type")
            .map(|r| clip(self.t(r), 80));
        let body = node.child_by_field_name("body").unwrap_or(node);
        let complexity = ark_metrics::analyze(Language::Rust, body, self.src, &name);
        let decorators = attrs_before(node, self.src);
        // actix/rocket: #[get("/x")]
        for d in &decorators {
            let inner = d.trim_start_matches("#[").trim_end_matches(']');
            let (m, rest) = inner.split_once('(').unwrap_or((inner, ""));
            if is_http_method(m)
                && let Some(path) = unquote(rest.split([',', ')']).next().unwrap_or(""))
            {
                self.facts.routes.push(RouteFact {
                    method: m.to_ascii_uppercase(),
                    path,
                    framework: "actix".to_owned(),
                    line: line(node),
                    handler: Some(qualname.clone()),
                });
            }
        }
        let is_async = named_children(node)
            .iter()
            .any(|c| c.kind() == "function_modifiers" && self.t(*c).contains("async"));
        self.facts.functions.push(FnFact {
            name,
            qualname,
            visibility,
            params,
            return_type,
            line_start: line(node),
            line_end: node.end_position().row as u32 + 1,
            byte_start: node.start_byte() as u32,
            byte_end: node.end_byte() as u32,
            calls: Vec::new(),
            decorators,
            complexity,
            is_component: false,
            is_async,
        });
        self.fn_stack.push(self.facts.functions.len() - 1);
        if let Some(b) = node.child_by_field_name("body") {
            self.visit(b);
        }
        self.fn_stack.pop();
    }

    fn strukt(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.t(n).to_owned())
            .unwrap_or_default();
        let mut fields = Vec::new();
        if let Some(body) = node.child_by_field_name("body") {
            for f in named_children(body) {
                if f.kind() == "field_declaration" {
                    fields.push(FieldFact {
                        name: f
                            .child_by_field_name("name")
                            .map(|n| self.t(n).to_owned())
                            .unwrap_or_default(),
                        ty: f.child_by_field_name("type").map(|t| clip(self.t(t), 80)),
                        value: attrs_before(f, self.src).first().cloned(),
                    });
                }
            }
        }
        if is_pub(node) {
            self.facts.exports.push(name.clone());
        }
        self.facts.classes.push(ClassFact {
            name,
            bases: Vec::new(),
            decorators: attrs_before(node, self.src),
            line: line(node),
            fields,
        });
    }

    fn call(&mut self, node: Node) {
        let Some(func) = node.child_by_field_name("function") else {
            return;
        };
        let Some(callee) =
            ark_metrics::callee_name(Language::Rust, node, self.src).map(str::to_owned)
        else {
            return;
        };
        let receiver = match func.kind() {
            "field_expression" => func
                .child_by_field_name("value")
                .map(|v| clip(self.t(v), 48)),
            "scoped_identifier" => func
                .child_by_field_name("path")
                .map(|v| clip(self.t(v), 48)),
            _ => None,
        };
        let args = node
            .child_by_field_name("arguments")
            .map(named_children)
            .unwrap_or_default();
        let first_string = args
            .first()
            .filter(|a| a.kind() == "string_literal")
            .and_then(|a| unquote(self.t(*a)));
        let ident_args: Vec<String> = args
            .iter()
            .filter(|a| matches!(a.kind(), "identifier" | "scoped_identifier"))
            .map(|a| clip(self.t(*a), 48))
            .collect();

        // axum: .route("/x", get(handler).post(other))
        if callee == "route"
            && let Some(path) = &first_string
            && path.starts_with('/')
            && let Some(m) = args.get(1)
        {
            let mut stack = vec![*m];
            while let Some(n) = stack.pop() {
                if n.kind() == "call_expression"
                    && let Some(mname) = ark_metrics::callee_name(Language::Rust, n, self.src)
                    && is_http_method(mname)
                {
                    let handler = n
                        .child_by_field_name("arguments")
                        .and_then(|a| named_children(a).into_iter().next())
                        .map(|h| {
                            let t = self.t(h);
                            t.rsplit("::").next().unwrap_or(t).to_owned()
                        });
                    self.facts.routes.push(RouteFact {
                        method: mname.to_ascii_uppercase(),
                        path: path.clone(),
                        framework: "axum".to_owned(),
                        line: line(node),
                        handler,
                    });
                }
                stack.extend(named_children(n));
            }
        }

        // reqwest: client.get(url) / reqwest::get(url)
        if let (Some(r), Some(url)) = (&receiver, &first_string)
            && is_http_method(&callee)
            && (r.contains("client") || r.contains("reqwest") || r.contains("http"))
            && (url.starts_with("http") || url.starts_with('/'))
        {
            self.facts.http_calls.push(HttpCallFact {
                method: callee.to_ascii_uppercase(),
                url: url.clone(),
                client: "reqwest".to_owned(),
                line: line(node),
                in_fn: self
                    .fn_stack
                    .last()
                    .map(|&i| self.facts.functions[i].qualname.clone()),
            });
        }

        let fact = CallFact {
            callee,
            receiver,
            line: line(node),
            first_string,
            ident_args,
        };
        match self.fn_stack.last() {
            Some(&i) => self.facts.functions[i].calls.push(fact),
            None => self.facts.top_calls.push(fact),
        }
    }
}
