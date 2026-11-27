//! Python extraction.

use ark_scan::Language;
use ark_schema::Visibility;
use tree_sitter::Node;

use crate::facts::*;
use crate::util::*;

struct X<'a, 'f> {
    src: &'a [u8],
    facts: &'f mut FileFacts,
    fn_stack: Vec<usize>,
    class_stack: Vec<String>,
}

pub fn extract(root: Node, src: &[u8], facts: &mut FileFacts) {
    let mut x = X {
        src,
        facts,
        fn_stack: Vec::new(),
        class_stack: Vec::new(),
    };
    x.visit(root, &[]);
    let public: Vec<String> = x
        .facts
        .functions
        .iter()
        .filter(|f| !f.qualname.contains('.') && !f.name.starts_with('_'))
        .map(|f| f.name.clone())
        .chain(
            x.facts
                .classes
                .iter()
                .filter(|c| !c.name.starts_with('_'))
                .map(|c| c.name.clone()),
        )
        .collect();
    x.facts.exports.extend(public);
}

const HTTP_CLIENTS: &[&str] = &["requests", "httpx", "session", "client", "aiohttp", "http"];

impl X<'_, '_> {
    fn t(&self, n: Node) -> &str {
        text(n, self.src)
    }

    fn visit(&mut self, node: Node, decorators: &[Node]) {
        match node.kind() {
            "decorated_definition" => {
                let decs: Vec<Node> = named_children(node)
                    .into_iter()
                    .filter(|c| c.kind() == "decorator")
                    .collect();
                if let Some(def) = node.child_by_field_name("definition") {
                    self.visit(def, &decs);
                }
                return;
            }
            "function_definition" => {
                self.function(node, decorators);
                return;
            }
            "class_definition" => {
                self.class(node, decorators);
                return;
            }
            "import_statement" => {
                for c in named_children(node) {
                    let (module, alias) = if c.kind() == "aliased_import" {
                        (
                            c.child_by_field_name("name").map(|n| self.t(n).to_owned()),
                            c.child_by_field_name("alias").map(|n| self.t(n).to_owned()),
                        )
                    } else {
                        (Some(self.t(c).to_owned()), None)
                    };
                    if let Some(m) = module {
                        let local =
                            alias.unwrap_or_else(|| m.split('.').next().unwrap_or(&m).to_owned());
                        self.facts.imports.push(ImportFact {
                            source: m,
                            names: vec![local],
                            line: line(node),
                        });
                    }
                }
            }
            "import_from_statement" => {
                let source = node
                    .child_by_field_name("module_name")
                    .map(|m| self.t(m).to_owned())
                    .unwrap_or_default();
                let mut names = Vec::new();
                let mut c = node.walk();
                for n in node.children_by_field_name("name", &mut c) {
                    let local = if n.kind() == "aliased_import" {
                        n.child_by_field_name("alias")
                    } else {
                        Some(n)
                    };
                    if let Some(l) = local {
                        names.push(self.t(l).to_owned());
                    }
                }
                self.facts.imports.push(ImportFact {
                    source,
                    names,
                    line: line(node),
                });
            }
            "call" => self.call(node),
            _ => {}
        }
        for c in named_children(node) {
            self.visit(c, &[]);
        }
    }

    fn function(&mut self, node: Node, decorators: &[Node]) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.t(n).to_owned())
            .unwrap_or_default();
        let qualname = match self.class_stack.last() {
            Some(c) => format!("{c}.{name}"),
            None => match self.fn_stack.last() {
                Some(&i) => format!("{}.{name}", self.facts.functions[i].qualname),
                None => name.clone(),
            },
        };
        let visibility = if name.starts_with("__") && name.ends_with("__") {
            Visibility::Public
        } else if name.starts_with('_') {
            Visibility::Private
        } else {
            Visibility::Public
        };
        let mut params = Vec::new();
        if let Some(p) = node.child_by_field_name("parameters") {
            for c in named_children(p) {
                let (pname, ty, default) = match c.kind() {
                    "identifier" => (self.t(c).to_owned(), None, None),
                    "typed_parameter" => (
                        named_children(c)
                            .first()
                            .map(|n| self.t(*n).to_owned())
                            .unwrap_or_default(),
                        c.child_by_field_name("type").map(|t| clip(self.t(t), 80)),
                        None,
                    ),
                    "default_parameter" | "typed_default_parameter" => (
                        c.child_by_field_name("name")
                            .map(|n| self.t(n).to_owned())
                            .unwrap_or_default(),
                        c.child_by_field_name("type").map(|t| clip(self.t(t), 80)),
                        c.child_by_field_name("value").map(|v| clip(self.t(v), 40)),
                    ),
                    "list_splat_pattern" | "dictionary_splat_pattern" => {
                        (self.t(c).to_owned(), None, None)
                    }
                    _ => continue,
                };
                if pname == "self" || pname == "cls" {
                    continue;
                }
                params.push(ParamFact {
                    name: pname,
                    ty,
                    default,
                });
            }
        }
        let return_type = node
            .child_by_field_name("return_type")
            .map(|r| clip(self.t(r), 80));
        let body = node.child_by_field_name("body").unwrap_or(node);
        let complexity = ark_metrics::analyze(Language::Python, body, self.src, &name);
        let deco_text: Vec<String> = decorators.iter().map(|d| clip(self.t(*d), 160)).collect();
        let is_async = self.t(node).trim_start().starts_with("async");

        // Route decorators: @app.get("/x"), @router.post(...), @app.route("/x", methods=[...])
        for d in decorators {
            let Some(call) = named_children(*d).into_iter().find(|c| c.kind() == "call") else {
                continue;
            };
            let Some(func) = call.child_by_field_name("function") else {
                continue;
            };
            if func.kind() != "attribute" {
                continue;
            }
            let attr = func
                .child_by_field_name("attribute")
                .map(|a| self.t(a).to_ascii_lowercase())
                .unwrap_or_default();
            let args = call
                .child_by_field_name("arguments")
                .map(named_children)
                .unwrap_or_default();
            let Some(path) = args
                .iter()
                .find(|a| a.kind() == "string")
                .and_then(|a| unquote(self.t(*a)))
            else {
                continue;
            };
            let (method, framework) = if is_http_method(&attr) {
                (attr.to_ascii_uppercase(), "fastapi")
            } else if attr == "route" || attr == "api_route" {
                let methods = args
                    .iter()
                    .find(|a| a.kind() == "keyword_argument" && self.t(**a).starts_with("methods"))
                    .map(|a| {
                        self.t(*a)
                            .split(['[', ']', ',', '"', '\'', '(', ')', ' '])
                            .filter(|s| is_http_method(s))
                            .map(str::to_ascii_uppercase)
                            .collect::<Vec<_>>()
                            .join("|")
                    })
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "GET".to_owned());
                (methods, "flask")
            } else if attr == "websocket" {
                ("WS".to_owned(), "fastapi")
            } else {
                continue;
            };
            self.facts.routes.push(RouteFact {
                method,
                path,
                framework: framework.to_owned(),
                line: line(*d),
                handler: Some(qualname.clone()),
            });
        }

        self.facts.functions.push(FnFact {
            name,
            qualname,
            visibility,
            params,
            return_type,
            line_start: decorators.first().map(|d| line(*d)).unwrap_or(line(node)),
            line_end: node.end_position().row as u32 + 1,
            byte_start: node.start_byte() as u32,
            byte_end: node.end_byte() as u32,
            calls: Vec::new(),
            decorators: deco_text,
            complexity,
            is_component: false,
            is_async,
            idents: Vec::new(),
            flow: Vec::new(),
        });
        self.fn_stack.push(self.facts.functions.len() - 1);
        let saved = std::mem::take(&mut self.class_stack);
        for c in named_children(node) {
            if c.kind() != "parameters" {
                self.visit(c, &[]);
            }
        }
        self.class_stack = saved;
        self.fn_stack.pop();
    }

    fn class(&mut self, node: Node, decorators: &[Node]) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.t(n).to_owned())
            .unwrap_or_default();
        let bases = node
            .child_by_field_name("superclasses")
            .map(|s| {
                named_children(s)
                    .into_iter()
                    .filter(|c| c.kind() != "keyword_argument")
                    .map(|c| clip(self.t(c), 60))
                    .collect()
            })
            .unwrap_or_default();
        let mut fields = Vec::new();
        if let Some(body) = node.child_by_field_name("body") {
            for stmt in named_children(body) {
                if stmt.kind() != "expression_statement" {
                    continue;
                }
                let Some(a) = named_children(stmt).into_iter().next() else {
                    continue;
                };
                if a.kind() != "assignment" {
                    continue;
                }
                let Some(left) = a.child_by_field_name("left") else {
                    continue;
                };
                if left.kind() != "identifier" {
                    continue;
                }
                fields.push(FieldFact {
                    name: self.t(left).to_owned(),
                    ty: a.child_by_field_name("type").map(|t| clip(self.t(t), 80)),
                    value: a.child_by_field_name("right").map(|r| clip(self.t(r), 160)),
                });
            }
        }
        // Flask-RESTX / Flask-RESTful resources: `@ns.route("/apps")` on a class whose
        // `get`/`post`/... methods are the handlers.
        let verbs = self.resource_methods(node);
        for d in decorators {
            let Some(call) = named_children(*d).into_iter().find(|c| c.kind() == "call") else {
                continue;
            };
            let is_route = call
                .child_by_field_name("function")
                .filter(|f| f.kind() == "attribute")
                .and_then(|f| f.child_by_field_name("attribute"))
                .is_some_and(|a| self.t(a) == "route");
            if !is_route {
                continue;
            }
            let paths: Vec<String> = call
                .child_by_field_name("arguments")
                .map(named_children)
                .unwrap_or_default()
                .into_iter()
                .filter(|a| a.kind() == "string")
                .filter_map(|a| unquote(self.t(a)))
                .collect();
            for path in paths {
                if verbs.is_empty() {
                    self.push_resource_route("ANY", &path, &name, line(*d));
                }
                for v in &verbs {
                    self.push_resource_route(
                        &v.to_ascii_uppercase(),
                        &path,
                        &format!("{name}.{v}"),
                        line(*d),
                    );
                }
            }
        }
        self.facts.classes.push(ClassFact {
            name: name.clone(),
            bases,
            decorators: decorators.iter().map(|d| clip(self.t(*d), 120)).collect(),
            line: line(node),
            fields,
        });
        self.class_stack.push(name);
        if let Some(body) = node.child_by_field_name("body") {
            for c in named_children(body) {
                self.visit(c, &[]);
            }
        }
        self.class_stack.pop();
    }

    /// HTTP verbs a resource class defines as methods (`get`, `post`, ...).
    fn resource_methods(&self, class: Node) -> Vec<String> {
        let Some(body) = class.child_by_field_name("body") else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for stmt in named_children(body) {
            let f = if stmt.kind() == "decorated_definition" {
                named_children(stmt)
                    .into_iter()
                    .find(|c| c.kind() == "function_definition")
            } else if stmt.kind() == "function_definition" {
                Some(stmt)
            } else {
                None
            };
            if let Some(n) = f.and_then(|f| f.child_by_field_name("name")) {
                let n = self.t(n);
                if is_http_method(n) && !out.iter().any(|o| o == n) {
                    out.push(n.to_owned());
                }
            }
        }
        out
    }

    fn push_resource_route(&mut self, method: &str, path: &str, handler: &str, line: u32) {
        self.facts.routes.push(RouteFact {
            method: method.to_owned(),
            path: path.to_owned(),
            framework: "flask-restx".to_owned(),
            line,
            handler: Some(handler.to_owned()),
        });
    }

    fn call(&mut self, node: Node) {
        let Some(func) = node.child_by_field_name("function") else {
            return;
        };
        let Some(callee) =
            ark_metrics::callee_name(Language::Python, node, self.src).map(str::to_owned)
        else {
            return;
        };
        let receiver = (func.kind() == "attribute")
            .then(|| func.child_by_field_name("object"))
            .flatten()
            .map(|o| clip(self.t(o), 48));
        let args = node
            .child_by_field_name("arguments")
            .map(named_children)
            .unwrap_or_default();
        let first_string = args
            .first()
            .filter(|a| a.kind() == "string")
            .and_then(|a| unquote(self.t(*a)));
        let ident_args: Vec<String> = args
            .iter()
            .filter(|a| matches!(a.kind(), "identifier" | "attribute"))
            .map(|a| clip(self.t(*a), 48))
            .collect();

        // Django: path("todos/", views.list_todos)
        if self.fn_stack.is_empty()
            && receiver.is_none()
            && matches!(callee.as_str(), "path" | "re_path")
            && let Some(p) = &first_string
        {
            self.facts.routes.push(RouteFact {
                method: "ANY".to_owned(),
                path: format!("/{}", p.trim_start_matches('^').trim_start_matches('/')),
                framework: "django".to_owned(),
                line: line(node),
                handler: ident_args
                    .first()
                    .map(|h| h.rsplit('.').next().unwrap_or(h).to_owned()),
            });
        }

        // Flask-RESTful: api.add_resource(TodoList, "/todos", "/todos/<id>")
        if callee == "add_resource" && receiver.is_some() {
            let handler = ident_args
                .first()
                .map(|h| h.rsplit('.').next().unwrap_or(h).to_owned());
            for a in args.iter().skip(1).filter(|a| a.kind() == "string") {
                if let (Some(p), Some(h)) = (unquote(self.t(*a)), &handler) {
                    self.push_resource_route("ANY", &p, h, line(node));
                }
            }
        }

        // HTTP clients: requests.get(url), httpx.post(url), self.session.get(url)
        if let (Some(r), Some(url)) = (&receiver, &first_string) {
            let rl = r.rsplit('.').next().unwrap_or(r).to_ascii_lowercase();
            if is_http_method(&callee)
                && HTTP_CLIENTS.iter().any(|c| rl.contains(c))
                && (url.starts_with("http") || url.starts_with('/') || url.starts_with(":param"))
            {
                self.facts.http_calls.push(HttpCallFact {
                    method: callee.to_ascii_uppercase(),
                    url: url.clone(),
                    client: rl,
                    line: line(node),
                    in_fn: self
                        .fn_stack
                        .last()
                        .map(|&i| self.facts.functions[i].qualname.clone()),
                });
            }
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
