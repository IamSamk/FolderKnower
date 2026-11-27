//! Java and C# extraction: classes, methods, imports, calls and web routes
//! (Spring `@GetMapping`, ASP.NET `[HttpGet]`).

use ark_scan::Language;
use ark_schema::Visibility;
use tree_sitter::Node;

use crate::facts::*;
use crate::util::*;

struct X<'a, 'f> {
    lang: Language,
    src: &'a [u8],
    facts: &'f mut FileFacts,
    fn_stack: Vec<usize>,
    class_stack: Vec<String>,
    /// Class-level route prefixes, parallel to `class_stack`.
    prefixes: Vec<String>,
}

pub fn extract(lang: Language, root: Node, src: &[u8], facts: &mut FileFacts) {
    let mut x = X {
        lang,
        src,
        facts,
        fn_stack: Vec::new(),
        class_stack: Vec::new(),
        prefixes: Vec::new(),
    };
    x.visit(root);
}

fn is_type_decl(kind: &str) -> bool {
    matches!(
        kind,
        "class_declaration"
            | "interface_declaration"
            | "enum_declaration"
            | "record_declaration"
            | "struct_declaration"
            | "class_definition"
            | "object_definition"
            | "trait_definition"
    )
}

impl X<'_, '_> {
    fn t(&self, n: Node) -> &str {
        text(n, self.src)
    }

    fn visit(&mut self, node: Node) {
        match node.kind() {
            k if is_type_decl(k) => {
                self.type_decl(node);
                return;
            }
            "import_declaration" if self.lang == Language::Scala => self.scala_import(node),
            "import_declaration" => self.java_import(node),
            "using_directive" => self.cs_using(node),
            "method_declaration" | "constructor_declaration" | "function_definition" => {
                self.method(node);
                return;
            }
            "method_invocation" | "invocation_expression" | "call_expression" => self.call(node),
            _ => {}
        }
        for c in named_children(node) {
            self.visit(c);
        }
    }

    fn java_import(&mut self, node: Node) {
        let raw = self.t(node);
        let s = raw
            .trim_start_matches("import")
            .trim()
            .trim_start_matches("static")
            .trim()
            .trim_end_matches(';')
            .trim();
        if s.is_empty() {
            return;
        }
        let last = s.rsplit('.').next().unwrap_or(s).to_owned();
        self.facts.imports.push(ImportFact {
            source: s.to_owned(),
            names: vec![last],
            line: line(node),
        });
    }

    /// `import org.apache.spark.sql.{DataFrame, SparkSession}` -> source up to the braces.
    fn scala_import(&mut self, node: Node) {
        let raw = self.t(node).trim_start_matches("import").trim().to_owned();
        let (path, names) = match raw.split_once('{') {
            Some((p, rest)) => (
                p.trim_end_matches('.').to_owned(),
                rest.trim_end_matches('}')
                    .split(',')
                    .map(|n| n.split("=>").next().unwrap_or("").trim().to_owned())
                    .filter(|n| !n.is_empty())
                    .collect::<Vec<_>>(),
            ),
            None => {
                let last = raw.rsplit('.').next().unwrap_or(&raw).to_owned();
                (raw.clone(), vec![last])
            }
        };
        if path.is_empty() {
            return;
        }
        // Each selector is its own class file: `pkg.Name`.
        if raw.contains('{') {
            for n in &names {
                self.facts.imports.push(ImportFact {
                    source: format!("{path}.{n}"),
                    names: vec![n.clone()],
                    line: line(node),
                });
            }
        } else {
            self.facts.imports.push(ImportFact {
                source: path,
                names,
                line: line(node),
            });
        }
    }

    fn cs_using(&mut self, node: Node) {
        let raw = self.t(node);
        let s = raw
            .trim_start_matches("global")
            .trim()
            .trim_start_matches("using")
            .trim()
            .trim_start_matches("static")
            .trim()
            .trim_end_matches(';')
            .trim();
        if s.is_empty() || s.contains('=') {
            return;
        }
        let last = s.rsplit('.').next().unwrap_or(s).to_owned();
        self.facts.imports.push(ImportFact {
            source: s.to_owned(),
            names: vec![last],
            line: line(node),
        });
    }

    /// Attributes/annotations as `(name, first string argument)`.
    fn annotations(&self, node: Node) -> Vec<(String, Option<String>)> {
        let mut out = Vec::new();
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            for c in named_children(n) {
                match c.kind() {
                    // Java: modifiers > annotation | marker_annotation
                    "modifiers" | "attribute_list" => stack.push(c),
                    "annotation" | "marker_annotation" | "attribute" => {
                        let name = c
                            .child_by_field_name("name")
                            .map(|n| self.t(n).rsplit('.').next().unwrap_or("").to_owned())
                            .unwrap_or_default();
                        let arg = self.first_string(c);
                        out.push((name, arg));
                    }
                    _ => {}
                }
            }
        }
        out.reverse();
        out
    }

    fn first_string(&self, node: Node) -> Option<String> {
        let mut stack = vec![node];
        while let Some(n) = stack.pop() {
            if matches!(
                n.kind(),
                "string_literal" | "string_literal_content" | "verbatim_string_literal"
            ) {
                return unquote(self.t(n)).or_else(|| Some(self.t(n).to_owned()));
            }
            let mut kids = named_children(n);
            kids.reverse();
            stack.extend(kids);
        }
        None
    }

    fn type_decl(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.t(n).to_owned())
            .unwrap_or_default();
        let anns = self.annotations(node);
        let decorators: Vec<String> = anns.iter().map(|(n, _)| n.clone()).collect();
        let prefix = anns
            .iter()
            .find(|(n, _)| matches!(n.as_str(), "RequestMapping" | "Route"))
            .and_then(|(_, a)| a.clone())
            .unwrap_or_default();
        let prefix = prefix.replace(
            "[controller]",
            &name.trim_end_matches("Controller").to_ascii_lowercase(),
        );
        let mut bases = Vec::new();
        let mut fields = Vec::new();
        for c in named_children(node) {
            match c.kind() {
                "superclass" | "super_interfaces" | "base_list" | "extends_interfaces"
                | "extends_clause" => {
                    for b in named_children(c) {
                        let b = if matches!(b.kind(), "type_list") {
                            named_children(b)
                        } else {
                            vec![b]
                        };
                        for t in b {
                            bases.push(self.t(t).split('<').next().unwrap_or("").trim().to_owned());
                        }
                    }
                }
                "class_body"
                | "template_body"
                | "declaration_list"
                | "interface_body"
                | "enum_body"
                | "record_declaration_body" => {
                    for m in named_children(c) {
                        if matches!(m.kind(), "field_declaration" | "property_declaration") {
                            let ty = m.child_by_field_name("type").map(|t| clip(self.t(t), 80));
                            let mut names = Vec::new();
                            let mut stack = vec![m];
                            while let Some(n) = stack.pop() {
                                if matches!(n.kind(), "variable_declarator") {
                                    if let Some(id) = n.child_by_field_name("name") {
                                        names.push(self.t(id).to_owned());
                                    }
                                } else if m.kind() == "property_declaration"
                                    && n.id() == m.id()
                                    && let Some(id) = n.child_by_field_name("name")
                                {
                                    names.push(self.t(id).to_owned());
                                } else {
                                    stack.extend(named_children(n));
                                }
                            }
                            for n in names {
                                fields.push(FieldFact {
                                    name: n,
                                    ty: ty.clone(),
                                    value: None,
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if self.t(node).starts_with("public") || self.public(node) {
            self.facts.exports.push(name.clone());
        }
        self.facts.classes.push(ClassFact {
            name: name.clone(),
            bases,
            decorators,
            line: line(node),
            fields,
        });
        self.class_stack.push(name);
        self.prefixes.push(prefix);
        for c in named_children(node) {
            if !matches!(c.kind(), "identifier" | "modifiers" | "attribute_list") {
                self.visit(c);
            }
        }
        self.class_stack.pop();
        self.prefixes.pop();
    }

    fn public(&self, node: Node) -> bool {
        if self.lang == Language::Scala {
            return !named_children(node).iter().any(|c| {
                c.kind() == "modifiers"
                    && (self.t(*c).contains("private") || self.t(*c).contains("protected"))
            });
        }
        named_children(node)
            .iter()
            .any(|c| matches!(c.kind(), "modifiers" | "modifier") && self.t(*c).contains("public"))
            || named_children(node)
                .iter()
                .any(|c| c.kind() == "modifier" && self.t(*c) == "public")
    }

    fn method(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.t(n).to_owned())
            .unwrap_or_default();
        let class = self.class_stack.last().cloned().unwrap_or_default();
        let qualname = if class.is_empty() {
            name.clone()
        } else {
            format!("{class}.{name}")
        };
        let visibility = if self.public(node) {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let anns = self.annotations(node);
        // Routes: Spring `@GetMapping("/x")`, `@RequestMapping(method = ..)`, ASP.NET `[HttpGet("x")]`.
        let prefix = self.prefixes.last().cloned().unwrap_or_default();
        for (a, arg) in &anns {
            let verb = match a.as_str() {
                "GetMapping" | "HttpGet" => Some("GET"),
                "PostMapping" | "HttpPost" => Some("POST"),
                "PutMapping" | "HttpPut" => Some("PUT"),
                "PatchMapping" | "HttpPatch" => Some("PATCH"),
                "DeleteMapping" | "HttpDelete" => Some("DELETE"),
                "RequestMapping" | "Route" if !prefix.is_empty() || arg.is_some() => Some("ANY"),
                _ => None,
            };
            if let Some(verb) = verb {
                let tail = arg.clone().unwrap_or_default();
                let path = join_path(&prefix, &tail);
                self.facts.routes.push(RouteFact {
                    method: verb.to_owned(),
                    path,
                    framework: if self.lang == Language::Java {
                        "spring"
                    } else {
                        "aspnet"
                    }
                    .to_owned(),
                    line: line(node),
                    handler: Some(qualname.clone()),
                });
            }
        }
        let mut params = Vec::new();
        if let Some(p) = node.child_by_field_name("parameters") {
            for c in named_children(p) {
                if matches!(
                    c.kind(),
                    "formal_parameter" | "parameter" | "spread_parameter"
                ) {
                    params.push(ParamFact {
                        name: c
                            .child_by_field_name("name")
                            .map(|n| self.t(n).to_owned())
                            .unwrap_or_default(),
                        ty: c.child_by_field_name("type").map(|t| clip(self.t(t), 80)),
                        default: None,
                    });
                }
            }
        }
        let return_type = node
            .child_by_field_name("type")
            .or_else(|| node.child_by_field_name("returns"))
            .map(|t| clip(self.t(t), 80));
        let body = node.child_by_field_name("body").unwrap_or(node);
        let complexity = ark_metrics::analyze(self.lang, body, self.src, &name);
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
            decorators: anns.into_iter().map(|(n, _)| n).collect(),
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

    fn call(&mut self, node: Node) {
        // Java method_invocation: object?, name, arguments. C# invocation_expression: function, arguments.
        let (callee, receiver) = if node.kind() == "call_expression" {
            let Some(f) = node.child_by_field_name("function") else {
                return;
            };
            if f.kind() == "field_expression" {
                let Some(n) = f.child_by_field_name("field") else {
                    return;
                };
                (
                    self.t(n).to_owned(),
                    f.child_by_field_name("value").map(|o| clip(self.t(o), 48)),
                )
            } else {
                (self.t(f).split('[').next().unwrap_or("").to_owned(), None)
            }
        } else if node.kind() == "method_invocation" {
            let Some(n) = node.child_by_field_name("name") else {
                return;
            };
            (
                self.t(n).to_owned(),
                node.child_by_field_name("object")
                    .map(|o| clip(self.t(o), 48)),
            )
        } else {
            let Some(f) = node.child_by_field_name("function") else {
                return;
            };
            match f.kind() {
                "member_access_expression" => {
                    let Some(n) = f.child_by_field_name("name") else {
                        return;
                    };
                    (
                        self.t(n).to_owned(),
                        f.child_by_field_name("expression")
                            .map(|o| clip(self.t(o), 48)),
                    )
                }
                _ => (self.t(f).split('<').next().unwrap_or("").to_owned(), None),
            }
        };
        let args = node
            .child_by_field_name("arguments")
            .map(named_children)
            .unwrap_or_default();
        let first_string = args.first().and_then(|a| self.first_string(*a));
        let ident_args: Vec<String> = args
            .iter()
            .filter(|a| matches!(a.kind(), "identifier"))
            .map(|a| clip(self.t(*a), 48))
            .collect();
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

fn join_path(prefix: &str, tail: &str) -> String {
    let a = prefix.trim_matches('/');
    let b = tail.trim_matches('/');
    match (a.is_empty(), b.is_empty()) {
        (true, true) => "/".to_owned(),
        (true, false) => format!("/{b}"),
        (false, true) => format!("/{a}"),
        (false, false) => format!("/{a}/{b}"),
    }
}
