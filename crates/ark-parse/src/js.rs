//! TypeScript / TSX / JavaScript extraction.

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
    exported: std::collections::BTreeSet<String>,
}

pub fn extract(lang: Language, root: Node, src: &[u8], _path: &str, facts: &mut FileFacts) {
    let mut x = X {
        lang,
        src,
        facts,
        fn_stack: Vec::new(),
        class_stack: Vec::new(),
        exported: Default::default(),
    };
    x.visit(root);
    let exported = std::mem::take(&mut x.exported);
    for f in &mut x.facts.functions {
        let top = f.qualname.split('.').next().unwrap_or(&f.qualname);
        if exported.contains(top) && f.visibility == Visibility::Internal {
            f.visibility = Visibility::Public;
        }
    }
    x.facts.exports.extend(exported);
}

fn is_fn(kind: &str) -> bool {
    matches!(
        kind,
        "function_declaration"
            | "generator_function_declaration"
            | "function_expression"
            | "function"
            | "arrow_function"
            | "method_definition"
    )
}

fn strip_type(t: &str) -> String {
    clip(t.trim_start_matches(':').trim(), 80)
}

impl X<'_, '_> {
    fn t(&self, n: Node) -> &str {
        text(n, self.src)
    }

    fn cur_fn(&self) -> Option<String> {
        self.fn_stack
            .last()
            .map(|&i| self.facts.functions[i].qualname.clone())
    }

    fn visit(&mut self, node: Node) {
        let kind = node.kind();
        match kind {
            "import_statement" => self.import(node),
            "export_statement" => self.export(node),
            "class_declaration" | "class" | "abstract_class_declaration" => {
                self.class(node);
                return;
            }
            "variable_declarator" => self.zod_schema(node),
            "call_expression" => self.call(node),
            "jsx_element" | "jsx_self_closing_element" => self.jsx(node),
            _ => {}
        }
        if is_fn(kind)
            && let Some(name) = self.fn_name(node)
        {
            self.function(node, name);
            return;
        }
        for c in named_children(node) {
            self.visit(c);
        }
    }

    /// Name for a function node, or None for anonymous inline callbacks
    /// (their body is attributed to the enclosing function).
    fn fn_name(&self, node: Node) -> Option<String> {
        if let Some(n) = node.child_by_field_name("name") {
            return Some(self.t(n).to_owned());
        }
        let parent = node.parent()?;
        match parent.kind() {
            "variable_declarator" => parent
                .child_by_field_name("name")
                .filter(|n| n.kind() == "identifier")
                .map(|n| self.t(n).to_owned()),
            "pair" => parent
                .child_by_field_name("key")
                .map(|n| self.t(n).trim_matches(['"', '\'']).to_owned()),
            "assignment_expression" => parent.child_by_field_name("left").map(|n| {
                let l = self.t(n);
                l.rsplit('.').next().unwrap_or(l).to_owned()
            }),
            "public_field_definition" | "field_definition" => parent
                .child_by_field_name("property")
                .or_else(|| parent.child_by_field_name("name"))
                .map(|n| self.t(n).to_owned()),
            // Inline callback at module level, e.g. `app.get("/x", (req, res) => ...)`.
            "arguments" if self.fn_stack.is_empty() => {
                let call = parent.parent()?;
                let callee = ark_metrics::callee_name(self.lang, call, self.src)?;
                Some(format!("{callee}@L{}", line(node)))
            }
            // `export default function () {}` / `export default () => ...`
            "export_statement" => Some("default".to_owned()),
            _ => None,
        }
    }

    fn function(&mut self, node: Node, name: String) {
        let qualname = match self.class_stack.last() {
            Some(c) => format!("{c}.{name}"),
            None => name.clone(),
        };
        let text_all = self.t(node);
        let visibility = if node.kind() == "method_definition" {
            let has = |k: &str| named_children(node).iter().any(|c| self.t(*c) == k);
            if name.starts_with('#') || has("private") {
                Visibility::Private
            } else if has("protected") {
                Visibility::Protected
            } else {
                Visibility::Public
            }
        } else {
            Visibility::Internal
        };
        let params = node
            .child_by_field_name("parameters")
            .map(|p| self.params(p))
            .or_else(|| {
                // `x => ...` single bare parameter
                node.child_by_field_name("parameter").map(|p| {
                    vec![ParamFact {
                        name: self.t(p).to_owned(),
                        ty: None,
                        default: None,
                    }]
                })
            })
            .unwrap_or_default();
        let return_type = node
            .child_by_field_name("return_type")
            .map(|r| strip_type(self.t(r)));
        let body = node.child_by_field_name("body").unwrap_or(node);
        let complexity = ark_metrics::analyze(self.lang, body, self.src, &name);
        let decorators = named_children(node)
            .into_iter()
            .filter(|c| c.kind() == "decorator")
            .map(|c| clip(self.t(c), 120))
            .collect();
        let is_async = text_all.trim_start().starts_with("async")
            || named_children(node).iter().any(|c| self.t(*c) == "async");
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
        for c in named_children(node) {
            if c.kind() != "formal_parameters" {
                self.visit(c);
            }
        }
        self.fn_stack.pop();
    }

    fn params(&self, p: Node) -> Vec<ParamFact> {
        let mut out = Vec::new();
        for c in named_children(p) {
            let (name, ty, default) = match c.kind() {
                "identifier" => (self.t(c).to_owned(), None, None),
                "required_parameter" | "optional_parameter" => {
                    let name = c
                        .child_by_field_name("pattern")
                        .map(|n| clip(self.t(n), 40))
                        .unwrap_or_default();
                    let ty = c.child_by_field_name("type").map(|t| strip_type(self.t(t)));
                    let def = c.child_by_field_name("value").map(|v| clip(self.t(v), 40));
                    (name, ty, def)
                }
                "assignment_pattern" => (
                    c.child_by_field_name("left")
                        .map(|n| clip(self.t(n), 40))
                        .unwrap_or_default(),
                    None,
                    c.child_by_field_name("right").map(|v| clip(self.t(v), 40)),
                ),
                "comment" => continue,
                _ => (clip(self.t(c), 40), None, None),
            };
            if name == "this" {
                continue;
            }
            out.push(ParamFact { name, ty, default });
        }
        out
    }

    fn import(&mut self, node: Node) {
        let Some(src_node) = node.child_by_field_name("source") else {
            return;
        };
        let Some(source) = unquote(self.t(src_node)) else {
            return;
        };
        let mut names = Vec::new();
        fn collect(n: Node, src: &[u8], out: &mut Vec<String>) {
            match n.kind() {
                "import_specifier" => {
                    let local = n
                        .child_by_field_name("alias")
                        .or_else(|| n.child_by_field_name("name"));
                    if let Some(l) = local {
                        out.push(text(l, src).to_owned());
                    }
                }
                "identifier" => out.push(text(n, src).to_owned()),
                _ => {
                    for c in named_children(n) {
                        collect(c, src, out);
                    }
                }
            }
        }
        for c in named_children(node) {
            if c.kind() == "import_clause" {
                collect(c, self.src, &mut names);
            }
        }
        self.facts.imports.push(ImportFact {
            source,
            names,
            line: line(node),
        });
    }

    fn export(&mut self, node: Node) {
        if let Some(decl) = node.child_by_field_name("declaration") {
            if let Some(n) = decl.child_by_field_name("name") {
                self.exported.insert(self.t(n).to_owned());
            }
            for c in named_children(decl) {
                if c.kind() == "variable_declarator"
                    && let Some(n) = c.child_by_field_name("name")
                {
                    self.exported.insert(self.t(n).to_owned());
                }
            }
        }
        for c in named_children(node) {
            if c.kind() == "export_clause" {
                for s in named_children(c) {
                    if let Some(n) = s.child_by_field_name("name") {
                        self.exported.insert(self.t(n).to_owned());
                    }
                }
            }
        }
        if self.t(node).starts_with("export default") {
            if let Some(v) = node.child_by_field_name("value")
                && v.kind() == "identifier"
            {
                self.exported.insert(self.t(v).to_owned());
            }
            self.exported.insert("default".to_owned());
        }
    }

    fn class(&mut self, node: Node) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.t(n).to_owned())
            .unwrap_or_else(|| format!("Class@L{}", line(node)));
        let mut bases = Vec::new();
        let mut decorators = Vec::new();
        for c in named_children(node) {
            match c.kind() {
                "class_heritage" => {
                    for h in named_children(c) {
                        for v in named_children(h) {
                            bases.push(clip(self.t(v), 60));
                        }
                    }
                }
                "decorator" => decorators.push(clip(self.t(c), 120)),
                _ => {}
            }
        }
        // Decorators may also sit on the parent export statement.
        if let Some(p) = node.parent()
            && p.kind() == "export_statement"
        {
            for c in named_children(p) {
                if c.kind() == "decorator" {
                    decorators.push(clip(self.t(c), 120));
                }
            }
        }
        let mut fields = Vec::new();
        if let Some(body) = node.child_by_field_name("body") {
            for m in named_children(body) {
                if matches!(m.kind(), "public_field_definition" | "field_definition") {
                    let fname = m
                        .child_by_field_name("property")
                        .or_else(|| m.child_by_field_name("name"))
                        .map(|n| self.t(n).to_owned())
                        .unwrap_or_default();
                    let ty = m.child_by_field_name("type").map(|t| strip_type(self.t(t)));
                    let decs: Vec<String> = named_children(m)
                        .into_iter()
                        .filter(|c| c.kind() == "decorator")
                        .map(|c| clip(self.t(c), 120))
                        .collect();
                    let value = if decs.is_empty() {
                        m.child_by_field_name("value").map(|v| clip(self.t(v), 120))
                    } else {
                        Some(decs.join(" "))
                    };
                    fields.push(FieldFact {
                        name: fname,
                        ty,
                        value,
                    });
                }
            }
        }
        self.facts.classes.push(ClassFact {
            name: name.clone(),
            bases,
            decorators,
            line: line(node),
            fields,
        });
        self.class_stack.push(name);
        for c in named_children(node) {
            self.visit(c);
        }
        self.class_stack.pop();
    }

    /// `const Todo = z.object({ title: z.string() })` is a schema shape.
    fn zod_schema(&mut self, node: Node) {
        let (Some(name), Some(value)) = (
            node.child_by_field_name("name"),
            node.child_by_field_name("value"),
        ) else {
            return;
        };
        if value.kind() != "call_expression" {
            return;
        }
        let Some(func) = value.child_by_field_name("function") else {
            return;
        };
        let ft = self.t(func);
        if !(ft == "z.object" || ft == "yup.object" || ft.ends_with("z.object")) {
            return;
        }
        let mut fields = Vec::new();
        if let Some(args) = value.child_by_field_name("arguments")
            && let Some(obj) = named_children(args).into_iter().next()
        {
            for p in named_children(obj) {
                if p.kind() == "pair"
                    && let (Some(k), Some(v)) =
                        (p.child_by_field_name("key"), p.child_by_field_name("value"))
                {
                    fields.push(FieldFact {
                        name: self.t(k).trim_matches(['"', '\'']).to_owned(),
                        ty: Some(clip(self.t(v), 60)),
                        value: None,
                    });
                }
            }
        }
        self.facts.classes.push(ClassFact {
            name: self.t(name).to_owned(),
            bases: vec!["zod".to_owned()],
            decorators: Vec::new(),
            line: line(node),
            fields,
        });
    }

    fn call(&mut self, node: Node) {
        let Some(func) = node.child_by_field_name("function") else {
            return;
        };
        let args = node.child_by_field_name("arguments");
        let arg_nodes = args.map(named_children).unwrap_or_default();
        let first_string = arg_nodes
            .first()
            .filter(|a| matches!(a.kind(), "string" | "template_string"))
            .and_then(|a| unquote(self.t(*a)));

        // require("x") / import("x")
        if (self.t(func) == "require" || func.kind() == "import")
            && let Some(source) = first_string.clone()
        {
            let names = node
                .parent()
                .filter(|p| p.kind() == "variable_declarator")
                .and_then(|p| p.child_by_field_name("name"))
                .map(|n| {
                    if n.kind() == "identifier" {
                        vec![self.t(n).to_owned()]
                    } else {
                        named_children(n)
                            .into_iter()
                            .map(|c| clip(self.t(c), 40))
                            .collect()
                    }
                })
                .unwrap_or_default();
            self.facts.imports.push(ImportFact {
                source,
                names,
                line: line(node),
            });
            return;
        }

        let Some(callee) = ark_metrics::callee_name(self.lang, node, self.src).map(str::to_owned)
        else {
            return;
        };
        let receiver = (func.kind() == "member_expression")
            .then(|| func.child_by_field_name("object"))
            .flatten()
            .map(|o| clip(self.t(o), 48));
        let mut ident_args: Vec<String> = arg_nodes
            .iter()
            .filter(|a| matches!(a.kind(), "identifier" | "member_expression"))
            .map(|a| clip(self.t(*a), 48))
            .collect();
        // Callbacks passed in option objects: useMutation({ mutationFn: createTodo }).
        for a in &arg_nodes {
            if a.kind() != "object" {
                continue;
            }
            for p in named_children(*a) {
                let v = match p.kind() {
                    "pair" => p.child_by_field_name("value"),
                    "shorthand_property_identifier" => Some(p),
                    _ => None,
                };
                if let Some(v) = v
                    && matches!(
                        v.kind(),
                        "identifier" | "member_expression" | "shorthand_property_identifier"
                    )
                {
                    ident_args.push(clip(self.t(v), 48));
                }
            }
        }
        let fact = CallFact {
            callee: callee.clone(),
            receiver: receiver.clone(),
            line: line(node),
            first_string: first_string.clone(),
            ident_args: ident_args.clone(),
        };

        let recv_last = receiver
            .as_deref()
            .map(|r| r.rsplit('.').next().unwrap_or(r).to_ascii_lowercase());

        // Server routes: app.get("/path", handler), router.post(...)
        if let (Some(r), Some(path)) = (&recv_last, &first_string)
            && path.starts_with('/')
            && (is_http_method(&callee) || callee == "all")
            && (matches!(
                r.as_str(),
                "app" | "server" | "api" | "r" | "fastify" | "hono"
            ) || r.contains("router")
                || r.contains("route"))
        {
            let handler = ident_args.last().cloned().or_else(|| {
                arg_nodes
                    .iter()
                    .rev()
                    .find(|a| matches!(a.kind(), "arrow_function" | "function_expression"))
                    .map(|a| format!("{callee}@L{}", line(*a)))
            });
            self.facts.routes.push(RouteFact {
                method: callee.to_ascii_uppercase(),
                path: path.clone(),
                framework: "express".to_owned(),
                line: line(node),
                handler,
            });
        }

        // HTTP clients: fetch(url, { method }), axios.get(url), api.post(url)
        if let Some(url) = &first_string
            && (url.starts_with('/') || url.starts_with("http") || url.starts_with(":param"))
        {
            let mut method = None;
            let mut client = None;
            if callee == "fetch" || callee == "$fetch" || callee == "ofetch" {
                client = Some(callee.clone());
                method = Some(
                    arg_nodes
                        .get(1)
                        .and_then(|o| self.object_prop(*o, "method"))
                        .unwrap_or_else(|| "GET".to_owned()),
                );
            } else if let Some(r) = &recv_last
                && is_http_method(&callee)
                && (r.contains("axios")
                    || r.contains("api")
                    || r.contains("http")
                    || r.contains("client")
                    || r == "ky"
                    || r.contains("request"))
            {
                client = Some(r.clone());
                method = Some(callee.to_ascii_uppercase());
            } else if callee == "axios" || callee == "useSWR" || callee == "useFetch" {
                client = Some(callee.clone());
                method = Some("GET".to_owned());
            }
            if let (Some(method), Some(client)) = (method, client) {
                self.facts.http_calls.push(HttpCallFact {
                    method: method.to_ascii_uppercase(),
                    url: url.clone(),
                    client,
                    line: line(node),
                    in_fn: self.cur_fn(),
                });
            }
        }

        match self.fn_stack.last() {
            Some(&i) => self.facts.functions[i].calls.push(fact),
            None => self.facts.top_calls.push(fact),
        }
    }

    fn object_prop(&self, obj: Node, key: &str) -> Option<String> {
        if obj.kind() != "object" {
            return None;
        }
        for p in named_children(obj) {
            if p.kind() == "pair"
                && p.child_by_field_name("key")
                    .map(|k| self.t(k).trim_matches(['"', '\'']))
                    == Some(key)
            {
                return p
                    .child_by_field_name("value")
                    .and_then(|v| unquote(self.t(v)));
            }
        }
        None
    }

    fn jsx(&mut self, node: Node) {
        if let Some(&i) = self.fn_stack.last() {
            let f = &mut self.facts.functions[i];
            if f.name
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_uppercase())
            {
                f.is_component = true;
            }
        }
        let open = if node.kind() == "jsx_element" {
            node.child_by_field_name("open_tag")
        } else {
            Some(node)
        };
        let Some(open) = open else { return };
        let Some(tag_node) = open.child_by_field_name("name") else {
            return;
        };
        let tag = self.t(tag_node).to_owned();
        // Rendering a component is a dependency on it: <TodoList /> -> TodoList.
        if tag.starts_with(|c: char| c.is_ascii_uppercase())
            && let Some(&i) = self.fn_stack.last()
        {
            let (receiver, callee) = match tag.rsplit_once('.') {
                Some((r, c)) => (Some(r.to_owned()), c.to_owned()),
                None => (None, tag.clone()),
            };
            self.facts.functions[i].calls.push(CallFact {
                callee,
                receiver,
                line: line(node),
                first_string: None,
                ident_args: Vec::new(),
            });
        }
        let mut event = None;
        let mut handler = None;
        let mut label_attr = None;
        for a in named_children(open) {
            if a.kind() != "jsx_attribute" {
                continue;
            }
            let kids = named_children(a);
            let Some(name) = kids.first().map(|n| self.t(*n).to_owned()) else {
                continue;
            };
            let value = kids.get(1).copied();
            if name.len() > 2
                && name.starts_with("on")
                && name[2..].starts_with(|c: char| c.is_ascii_uppercase())
            {
                if event.is_none() {
                    event = Some(name);
                    handler = value.and_then(|v| self.handler_of(v));
                }
            } else if matches!(
                name.as_str(),
                "aria-label"
                    | "placeholder"
                    | "title"
                    | "alt"
                    | "label"
                    | "value"
                    | "href"
                    | "type"
            ) && label_attr.is_none()
            {
                label_attr = value.and_then(|v| unquote(self.t(v)));
            }
        }
        let interactive = matches!(
            tag.as_str(),
            "button" | "a" | "form" | "input" | "select" | "textarea" | "Link" | "Button"
        );
        if event.is_none() && !interactive {
            return;
        }
        let label = if node.kind() == "jsx_element" {
            let t: Vec<String> = named_children(node)
                .into_iter()
                .filter(|c| c.kind() == "jsx_text")
                .map(|c| self.t(c).trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect();
            if t.is_empty() {
                label_attr
            } else {
                Some(clip(&t.join(" "), 40))
            }
        } else {
            label_attr
        };
        self.facts.ui_elements.push(UiFact {
            tag,
            label,
            event,
            handler,
            line: line(node),
            column: node.start_position().column as u32,
            in_fn: self.cur_fn(),
        });
    }

    /// `{handleAdd}` -> `handleAdd`; `{() => save(id)}` -> `save`.
    fn handler_of(&self, v: Node) -> Option<String> {
        let inner = if v.kind() == "jsx_expression" {
            named_children(v).into_iter().next()?
        } else {
            v
        };
        match inner.kind() {
            "identifier" | "member_expression" => Some(clip(self.t(inner), 60)),
            "call_expression" => inner
                .child_by_field_name("function")
                .map(|f| clip(self.t(f), 60)),
            "arrow_function" | "function_expression" => {
                fn first_call(n: Node, src: &[u8]) -> Option<String> {
                    if n.kind() == "call_expression" {
                        return n
                            .child_by_field_name("function")
                            .map(|f| clip(text(f, src), 60));
                    }
                    for c in named_children(n) {
                        if let Some(s) = first_call(c, src) {
                            return Some(s);
                        }
                    }
                    None
                }
                first_call(inner.child_by_field_name("body")?, self.src)
            }
            _ => None,
        }
    }
}
