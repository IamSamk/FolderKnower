use ark_schema::Complexity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FileFacts {
    pub lines: u32,
    pub functions: Vec<FnFact>,
    pub imports: Vec<ImportFact>,
    pub classes: Vec<ClassFact>,
    pub routes: Vec<RouteFact>,
    pub http_calls: Vec<HttpCallFact>,
    pub ui_elements: Vec<UiFact>,
    /// Exported names (TS/JS `export`, Rust `pub`, Go capitalized, Python `__all__` or public).
    pub exports: Vec<String>,
    /// Calls at module level (e.g. `app.use(cors())`, `urlpatterns = [path(...)]`).
    pub top_calls: Vec<CallFact>,
    pub diagnostics: Vec<String>,
    /// HTTP verbs the file compares against, e.g. `req.method === "POST"` or `case "GET"`.
    #[serde(default)]
    pub verbs_checked: Vec<String>,
    /// Where the parser had to recover: (line, the text there), first few only.
    #[serde(default)]
    pub syntax_errors: Vec<SyntaxErrorFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntaxErrorFact {
    pub line: u32,
    /// `missing )` or the clipped source text the parser could not place.
    pub text: String,
}

impl FileFacts {
    /// Sort everything for byte-stable output.
    pub fn finish(&mut self) {
        self.functions
            .sort_by(|a, b| (a.byte_start, &a.qualname).cmp(&(b.byte_start, &b.qualname)));
        // Duplicate qualnames (overloads, same-named closures) get a #n suffix.
        let mut seen = std::collections::BTreeMap::<String, u32>::new();
        for f in &mut self.functions {
            let n = seen.entry(f.qualname.clone()).or_insert(0);
            *n += 1;
            if *n > 1 {
                f.qualname = format!("{}#{}", f.qualname, n);
            }
        }
        self.imports
            .sort_by(|a, b| (a.line, &a.source).cmp(&(b.line, &b.source)));
        self.classes.sort_by_key(|c| c.line);
        self.routes
            .sort_by(|a, b| (a.line, &a.path).cmp(&(b.line, &b.path)));
        self.http_calls
            .sort_by(|a, b| (a.line, &a.url).cmp(&(b.line, &b.url)));
        self.ui_elements.sort_by_key(|u| (u.line, u.column));
        self.exports.sort();
        self.exports.dedup();
        self.top_calls
            .sort_by(|a, b| (a.line, &a.callee).cmp(&(b.line, &b.callee)));
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FnFact {
    pub name: String,
    /// `Class.method`, `Type::method`, or the bare name.
    pub qualname: String,
    pub visibility: ark_schema::Visibility,
    pub params: Vec<ParamFact>,
    pub return_type: Option<String>,
    pub line_start: u32,
    pub line_end: u32,
    pub byte_start: u32,
    pub byte_end: u32,
    pub calls: Vec<CallFact>,
    pub decorators: Vec<String>,
    pub complexity: Complexity,
    /// Returns JSX: a UI component.
    pub is_component: bool,
    pub is_async: bool,
    /// Distinct identifiers, field and type names used in the body (sorted, capped).
    /// Feeds the data-structure and AI-pipeline recognizers.
    #[serde(default)]
    pub idents: Vec<String>,
    /// Control-flow outline for the ideal-path view.
    #[serde(default)]
    pub flow: Vec<ark_schema::FlowNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamFact {
    pub name: String,
    pub ty: Option<String>,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallFact {
    /// Last identifier, e.g. `get` in `router.get(...)`.
    pub callee: String,
    /// Receiver text, e.g. `router`, `self.repo`, `db.session`.
    pub receiver: Option<String>,
    pub line: u32,
    /// First argument when it is a plain string literal.
    pub first_string: Option<String>,
    /// Identifiers passed as arguments (handlers, models).
    pub ident_args: Vec<String>,
    /// Number of arguments (positional + keyword).
    #[serde(default)]
    pub n_args: u32,
    /// Keyword / option-object arguments with literal-ish values: `chunk_size=1000`, `{ k: 4 }`.
    #[serde(default)]
    pub kwargs: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportFact {
    /// Module specifier as written: `./api`, `..models`, `react`, `crate::db`.
    pub source: String,
    /// Local names bound by the import.
    pub names: Vec<String>,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassFact {
    pub name: String,
    pub bases: Vec<String>,
    /// Decorators/attributes as raw text, e.g. `@Entity()`, `#[derive(sqlx::FromRow)]`.
    pub decorators: Vec<String>,
    pub line: u32,
    pub fields: Vec<FieldFact>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldFact {
    pub name: String,
    pub ty: Option<String>,
    /// Right-hand side / tag / decorator text, truncated.
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteFact {
    pub method: String,
    pub path: String,
    pub framework: String,
    pub line: u32,
    /// Name of the handler function (or the decorated function).
    pub handler: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpCallFact {
    pub method: String,
    /// URL with `${x}`/`{x}` replaced by `:param`.
    pub url: String,
    pub client: String,
    pub line: u32,
    /// Enclosing function qualname.
    pub in_fn: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiFact {
    pub tag: String,
    pub label: Option<String>,
    pub event: Option<String>,
    /// Handler as written: identifier or the first call inside an inline arrow.
    pub handler: Option<String>,
    pub line: u32,
    pub column: u32,
    pub in_fn: Option<String>,
}
