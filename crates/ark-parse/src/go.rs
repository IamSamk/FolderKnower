//! Go extraction.

use ark_scan::Language;
use ark_schema::Visibility;
use tree_sitter::Node;

use crate::facts::*;
use crate::util::*;

struct X<'a, 'f> {
    src: &'a [u8],
    facts: &'f mut FileFacts,
    fn_stack: Vec<usize>,
}

pub fn extract(root: Node, src: &[u8], facts: &mut FileFacts) {
    let mut x = X {
        src,
        facts,
        fn_stack: Vec::new(),
    };
    x.visit(root);
}

fn exported(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase())
}

impl X<'_, '_> {
    fn t(&self, n: Node) -> &str {
        text(n, self.src)
    }

    fn visit(&mut self, node: Node) {
        match node.kind() {
            "function_declaration" | "method_declaration" => {
                self.function(node);
                return;
            }
            "import_spec" => {
                if let Some(p) = node.child_by_field_name("path")
                    && let Some(source) = unquote(self.t(p))
                {
                    let local = node
                        .child_by_field_name("name")
                        .map(|n| self.t(n).to_owned())
                        .unwrap_or_else(|| source.rsplit('/').next().unwrap_or(&source).to_owned());
                    self.facts.imports.push(ImportFact {
                        source,
                        names: vec![local],
                        line: line(node),
                    });
                }
            }
            "type_spec" => self.type_spec(node),
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
        let qualname = match node.child_by_field_name("receiver") {
            Some(r) => {
                let ty = named_children(r)
                    .first()
                    .and_then(|p| p.child_by_field_name("type"))
                    .map(|t| self.t(t).trim_start_matches('*').to_owned())
                    .unwrap_or_default();
                format!("{ty}.{name}")
            }
            None => name.clone(),
        };
        let visibility = if exported(&name) {
            Visibility::Public
        } else {
            Visibility::Private
        };
        if visibility == Visibility::Public && !qualname.contains('.') {
            self.facts.exports.push(name.clone());
        }
        let mut params = Vec::new();
        if let Some(p) = node.child_by_field_name("parameters") {
            for c in named_children(p) {
                if !matches!(
                    c.kind(),
                    "parameter_declaration" | "variadic_parameter_declaration"
                ) {
                    continue;
                }
                let ty = c.child_by_field_name("type").map(|t| clip(self.t(t), 80));
                let mut cur = c.walk();
                let names: Vec<String> = c
                    .children_by_field_name("name", &mut cur)
                    .map(|n| self.t(n).to_owned())
                    .collect();
                if names.is_empty() {
                    params.push(ParamFact {
                        name: "_".to_owned(),
                        ty: ty.clone(),
                        default: None,
                    });
                }
                for n in names {
                    params.push(ParamFact {
                        name: n,
                        ty: ty.clone(),
                        default: None,
                    });
                }
            }
        }
        let return_type = node
            .child_by_field_name("result")
            .map(|r| clip(self.t(r), 80));
        let body = node.child_by_field_name("body").unwrap_or(node);
        let complexity = ark_metrics::analyze(Language::Go, body, self.src, &name);
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
            decorators: Vec::new(),
            complexity,
            is_component: false,
            is_async: false,
            idents: Vec::new(),
            flow: Vec::new(),
        });
        self.fn_stack.push(self.facts.functions.len() - 1);
        if let Some(b) = node.child_by_field_name("body") {
            self.visit(b);
        }
        self.fn_stack.pop();
    }

    fn type_spec(&mut self, node: Node) {
        let Some(ty) = node.child_by_field_name("type") else {
            return;
        };
        if ty.kind() != "struct_type" {
            return;
        }
        let name = node
            .child_by_field_name("name")
            .map(|n| self.t(n).to_owned())
            .unwrap_or_default();
        let mut fields = Vec::new();
        for list in named_children(ty) {
            for f in named_children(list) {
                if f.kind() != "field_declaration" {
                    continue;
                }
                let fty = f.child_by_field_name("type").map(|t| clip(self.t(t), 80));
                let tag = f.child_by_field_name("tag").map(|t| clip(self.t(t), 160));
                let mut cur = f.walk();
                let names: Vec<String> = f
                    .children_by_field_name("name", &mut cur)
                    .map(|n| self.t(n).to_owned())
                    .collect();
                if names.is_empty() {
                    // embedded field, e.g. gorm.Model
                    fields.push(FieldFact {
                        name: fty.clone().unwrap_or_default(),
                        ty: fty.clone(),
                        value: tag.clone(),
                    });
                }
                for n in names {
                    fields.push(FieldFact {
                        name: n,
                        ty: fty.clone(),
                        value: tag.clone(),
                    });
                }
            }
        }
        let bases = fields
            .iter()
            .filter(|f| f.ty.as_deref() == Some(f.name.as_str()))
            .map(|f| f.name.clone())
            .collect();
        if exported(&name) {
            self.facts.exports.push(name.clone());
        }
        self.facts.classes.push(ClassFact {
            name,
            bases,
            decorators: Vec::new(),
            line: line(node),
            fields,
        });
    }

    fn call(&mut self, node: Node) {
        let Some(func) = node.child_by_field_name("function") else {
            return;
        };
        let Some(callee) =
            ark_metrics::callee_name(Language::Go, node, self.src).map(str::to_owned)
        else {
            return;
        };
        let receiver = (func.kind() == "selector_expression")
            .then(|| func.child_by_field_name("operand"))
            .flatten()
            .map(|o| clip(self.t(o), 48));
        let args = node
            .child_by_field_name("arguments")
            .map(named_children)
            .unwrap_or_default();
        let first_string = args
            .first()
            .filter(|a| {
                matches!(
                    a.kind(),
                    "interpreted_string_literal" | "raw_string_literal"
                )
            })
            .and_then(|a| unquote(self.t(*a)));
        let ident_args: Vec<String> = args
            .iter()
            .filter(|a| matches!(a.kind(), "identifier" | "selector_expression"))
            .map(|a| clip(self.t(*a), 48))
            .collect();

        // gin r.GET("/x", h), echo e.POST, fiber app.Get, net/http HandleFunc("/x", h)
        if let Some(path) = &first_string
            && path.starts_with('/')
            && receiver.is_some()
        {
            let method = if is_http_method(&callee) {
                Some(callee.to_ascii_uppercase())
            } else if matches!(callee.as_str(), "HandleFunc" | "Handle" | "Any") {
                Some("ANY".to_owned())
            } else {
                None
            };
            if let Some(method) = method {
                self.facts.routes.push(RouteFact {
                    method,
                    path: path.clone(),
                    framework: "go-http".to_owned(),
                    line: line(node),
                    handler: ident_args
                        .last()
                        .map(|h| h.rsplit('.').next().unwrap_or(h).to_owned()),
                });
            }
        }

        // http.Get(url) / client.Get(url)
        if let (Some(r), Some(url)) = (&receiver, &first_string)
            && matches!(callee.as_str(), "Get" | "Post" | "Head" | "PostForm")
            && (r == "http" || r.to_ascii_lowercase().contains("client"))
            && (url.starts_with("http") || url.starts_with('/'))
        {
            self.facts.http_calls.push(HttpCallFact {
                method: callee.to_ascii_uppercase(),
                url: url.clone(),
                client: "net/http".to_owned(),
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
            ..CallFact::default()
        };
        match self.fn_stack.last() {
            Some(&i) => self.facts.functions[i].calls.push(fact),
            None => self.facts.top_calls.push(fact),
        }
    }
}
