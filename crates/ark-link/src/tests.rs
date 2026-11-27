use super::*;
use ark_scan::{Language, ScannedFile};

fn pf(path: &str, src: &str) -> ParsedFile {
    let lang = Language::from_path(path).unwrap();
    ParsedFile {
        scan: ScannedFile {
            path: path.to_owned(),
            size: src.len() as u64,
            hash: blake_free_hash(src),
            language: Some(lang),
            other: None,
        },
        facts: ark_parse::parse_file(lang, path, src),
    }
}

fn blake_free_hash(s: &str) -> String {
    format!("{:x}", s.len())
}

fn fixture() -> Vec<ParsedFile> {
    let mut v = vec![
        pf(
            "web/src/api.ts",
            r#"export async function createTodo(title: string) {
  return fetch(`${BASE}/api/todos`, { method: "POST", body: JSON.stringify({ title }) });
}
export const listTodos = () => fetch("/api/todos");"#,
        ),
        pf(
            "web/src/App.tsx",
            r#"import { createTodo } from "./api";
import * as api from "./api";
export function App() {
  return <div><button onClick={() => createTodo("x")}>Add</button><button onClick={api.listTodos}>Load</button></div>;
}"#,
        ),
        pf(
            "server/app/main.py",
            r#"from fastapi import FastAPI
from .routers import todos
app = FastAPI()
app.include_router(todos.router, prefix="/api")"#,
        ),
        pf(
            "server/app/routers/todos.py",
            r#"from fastapi import APIRouter
from ..services import save_todo
router = APIRouter()
@router.post("/todos")
def create(body):
    return save_todo(body)
@router.get("/todos")
def list_all():
    return []"#,
        ),
        pf(
            "server/app/services.py",
            "def save_todo(body):\n    return helper(body)\n\ndef helper(x):\n    return x\n",
        ),
    ];
    v.sort_by(|a, b| a.scan.path.cmp(&b.scan.path));
    v
}

#[test]
fn imports_resolve_across_languages() {
    let files = fixture();
    let l = link(&files, &LinkInputs::default());
    assert_eq!(
        l.file_imports["web/src/App.tsx"]
            .iter()
            .map(|e| e.target.as_str())
            .collect::<Vec<_>>(),
        ["file:web/src/api.ts"]
    );
    assert_eq!(
        l.file_imports["server/app/routers/todos.py"][0].target,
        "file:server/app/services.py"
    );
    assert_eq!(
        l.file_imports["server/app/main.py"][0].target,
        "file:server/app/routers/todos.py"
    );
    assert!(l.externals["server/app/main.py"].contains("fastapi"));
}

#[test]
fn calls_ui_routes_and_http_edges() {
    let files = fixture();
    let l = link(&files, &LinkInputs::default());
    let create = &l.fn_calls["fn:server/app/routers/todos.py#create"];
    assert_eq!(create[0].target, "fn:server/app/services.py#save_todo");
    assert_eq!(create[0].provenance, Provenance::TsResolved);
    assert!(
        l.called_by["fn:server/app/services.py#helper"]
            .contains("fn:server/app/services.py#save_todo")
    );

    let ui: Vec<_> = l.ui_elements.iter().map(|u| u.handler.as_deref()).collect();
    assert_eq!(
        ui,
        [
            Some("fn:web/src/api.ts#createTodo"),
            Some("fn:web/src/api.ts#listTodos")
        ]
    );

    assert_eq!(l.routes.len(), 2);
    assert!(l.routes.iter().all(|r| r.handler.is_some()));
    let pairs: Vec<(&str, &str)> = l
        .http_edges
        .iter()
        .map(|e| (e.source.as_str(), e.target.as_str()))
        .collect();
    assert!(pairs.contains(&(
        "fn:web/src/api.ts#createTodo",
        "fn:server/app/routers/todos.py#create"
    )));
    assert!(pairs.contains(&(
        "fn:web/src/api.ts#listTodos",
        "fn:server/app/routers/todos.py#list_all"
    )));
    assert_eq!(pairs.len(), 2, "{pairs:?}");
}

#[test]
fn url_normalisation() {
    assert_eq!(
        normalize_url_segments("https://x.io:8080/api/todos/:param?q=1"),
        ["api", "todos", "*"]
    );
    assert_eq!(normalize_url_segments(":param/users/{id}"), ["users", "*"]);
}

#[test]
fn nextjs_pages_api_methods_come_from_checks() {
    let f = pf(
        "web/pages/api/todos.ts",
        "export default function handler(req, res) {\n  if (req.method !== \"POST\") return res.status(405).end();\n  res.json({});\n}\n",
    );
    let routes = resolve::file_based_routes("web/pages/api/todos.ts", &f.facts);
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].method, "POST");
    let g = pf(
        "web/pages/api/any.ts",
        "export default function handler(req, res) { res.end(); }\n",
    );
    let r = resolve::file_based_routes("web/pages/api/any.ts", &g.facts);
    assert_eq!(r[0].method, "ANY");
}

#[test]
fn macro_routes_compose_through_sub_mounts() {
    let files = vec![
        pf(
            "app/src/routes/mod.rs",
            "#[routes::routes(routes(\n  \"/health\" => get(get_health),\n  \"/indexes\" => sub(indexes::IndexesApi),\n))]\npub struct Api;\nfn get_health() {}\n",
        ),
        pf(
            "app/src/routes/indexes/mod.rs",
            "#[routes::routes(routes(\n  \"\" => [get(list), post(create)],\n  \"/{uid}\" => get(one),\n))]\npub struct IndexesApi;\nfn list() {}\nfn create() {}\nfn one() {}\n",
        ),
    ];
    let linked = link(&files, &LinkInputs::default());
    let got: Vec<String> = linked
        .routes
        .iter()
        .map(|r| format!("{} {}", r.method, r.path))
        .collect();
    assert!(got.contains(&"GET /health".to_owned()), "{got:?}");
    assert!(got.contains(&"GET /indexes".to_owned()), "{got:?}");
    assert!(got.contains(&"POST /indexes".to_owned()), "{got:?}");
    assert!(got.contains(&"GET /indexes/{uid}".to_owned()), "{got:?}");
}
