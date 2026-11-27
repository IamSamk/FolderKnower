use super::*;
use ark_schema::BigO;

#[test]
fn tsx_component_button_fetch() {
    let src = r#"
import { useState } from "react";
import { createTodo } from "./api";

export function App() {
  const [t, setT] = useState("");
  const add = async () => { await fetch(`/api/todos/${t}`, { method: "POST" }); };
  return (
    <div>
      <input placeholder="Title" onChange={(e) => setT(e.target.value)} />
      <button onClick={add}>Add todo</button>
      <button onClick={() => createTodo(t)}>Quick</button>
    </div>
  );
}
"#;
    let f = parse_file(Language::Tsx, "web/App.tsx", src);
    assert!(f.diagnostics.is_empty(), "{:?}", f.diagnostics);
    let app = f.functions.iter().find(|x| x.name == "App").unwrap();
    assert!(app.is_component);
    assert_eq!(app.visibility, ark_schema::Visibility::Public);
    assert!(f.functions.iter().any(|x| x.qualname == "add"));
    assert_eq!(f.imports.len(), 2);
    assert_eq!(f.imports[1].names, ["createTodo"]);
    assert_eq!(f.http_calls.len(), 1);
    assert_eq!(f.http_calls[0].method, "POST");
    assert_eq!(f.http_calls[0].url, "/api/todos/:param");
    assert_eq!(f.http_calls[0].in_fn.as_deref(), Some("add"));
    let buttons: Vec<_> = f.ui_elements.iter().filter(|u| u.tag == "button").collect();
    assert_eq!(buttons.len(), 2);
    assert_eq!(buttons[0].label.as_deref(), Some("Add todo"));
    assert_eq!(buttons[0].handler.as_deref(), Some("add"));
    assert_eq!(buttons[1].handler.as_deref(), Some("createTodo"));
}

#[test]
fn express_routes() {
    let src = r#"
const express = require("express");
const router = express.Router();
router.get("/todos", listTodos);
router.post("/todos", async (req, res) => { res.json(await db.insert(req.body)); });
function listTodos(req, res) { for (const t of todos) { res.write(t); } }
"#;
    let f = parse_file(Language::JavaScript, "server/routes.js", src);
    assert_eq!(f.imports[0].source, "express");
    assert_eq!(f.routes.len(), 2);
    assert_eq!(f.routes[0].handler.as_deref(), Some("listTodos"));
    assert_eq!(f.routes[1].method, "POST");
    assert!(
        f.routes[1]
            .handler
            .as_deref()
            .unwrap()
            .starts_with("post@L")
    );
    let lt = f.functions.iter().find(|x| x.name == "listTodos").unwrap();
    assert_eq!(lt.complexity.time_big_o, BigO::Linear);
}

#[test]
fn python_fastapi_sqlalchemy() {
    let src = r#"
from fastapi import APIRouter, Depends
from sqlalchemy import Column, Integer, String, ForeignKey
from .db import Base, get_db

router = APIRouter()

class Todo(Base):
    __tablename__ = "todos"
    id = Column(Integer, primary_key=True)
    owner_id = Column(Integer, ForeignKey("users.id"))

@router.post("/todos/{todo_id}")
async def create_todo(todo_id: int, body: TodoIn, db=Depends(get_db)) -> TodoOut:
    if body.title and not body.done:
        db.add(Todo(title=body.title))
    return body

def fib(n):
    return n if n < 2 else fib(n - 1) + fib(n - 2)

def bsearch(xs, x, lo, hi):
    mid = (lo + hi) // 2
    return bsearch(xs, x, lo, mid)
"#;
    let f = parse_file(Language::Python, "api/routes.py", src);
    assert!(f.diagnostics.is_empty(), "{:?}", f.diagnostics);
    assert_eq!(f.routes.len(), 1);
    assert_eq!(f.routes[0].path, "/todos/:param");
    assert_eq!(f.routes[0].handler.as_deref(), Some("create_todo"));
    let ct = f
        .functions
        .iter()
        .find(|x| x.name == "create_todo")
        .unwrap();
    assert!(ct.is_async);
    assert_eq!(ct.params.len(), 3);
    assert_eq!(ct.params[0].ty.as_deref(), Some("int"));
    assert_eq!(ct.params[2].default.as_deref(), Some("Depends(get_db)"));
    assert_eq!(ct.return_type.as_deref(), Some("TodoOut"));
    assert!(ct.complexity.cyclomatic >= 3);
    let todo = f.classes.iter().find(|c| c.name == "Todo").unwrap();
    assert_eq!(todo.bases, ["Base"]);
    assert_eq!(todo.fields.len(), 3);
    let fib = f.functions.iter().find(|x| x.name == "fib").unwrap();
    assert_eq!(fib.complexity.time_big_o, BigO::Exponential);
    let bs = f.functions.iter().find(|x| x.name == "bsearch").unwrap();
    assert_eq!(bs.complexity.time_big_o, BigO::Logarithmic);
}

#[test]
fn rust_axum_and_structs() {
    let src = r#"
use axum::{routing::get, Router};

#[derive(sqlx::FromRow)]
pub struct Todo { pub id: i64, pub title: String }

pub fn app() -> Router { Router::new().route("/todos", get(list).post(create)) }

async fn list() -> String {
    let mut v = Vec::new();
    for i in 0..10 { for j in 0..10 { v.push(i * j); } }
    format!("{v:?}")
}
"#;
    let f = parse_file(Language::Rust, "src/main.rs", src);
    assert_eq!(f.routes.len(), 2);
    assert!(
        f.routes
            .iter()
            .any(|r| r.method == "GET" && r.handler.as_deref() == Some("list"))
    );
    let list = f.functions.iter().find(|x| x.name == "list").unwrap();
    assert_eq!(list.complexity.time_big_o, BigO::Quadratic);
    assert_eq!(list.complexity.space_big_o, BigO::Linear);
    assert_eq!(f.classes[0].decorators, ["#[derive(sqlx::FromRow)]"]);
}

#[test]
fn go_handlers() {
    let src = r#"
package main
import "net/http"
type User struct { ID int `gorm:"primaryKey"`; Name string }
func (s *Server) List(w http.ResponseWriter, r *http.Request) { s.db.Find(&users) }
func main() { http.HandleFunc("/users", s.List); r.GET("/health", health) }
"#;
    let f = parse_file(Language::Go, "main.go", src);
    assert!(f.functions.iter().any(|x| x.qualname == "Server.List"));
    assert_eq!(f.routes.len(), 2);
    assert_eq!(f.classes[0].fields.len(), 2);
    assert_eq!(f.imports[0].source, "net/http");
}

#[test]
fn malformed_input_is_partial_not_panic() {
    let f = parse_file(
        Language::TypeScript,
        "x.ts",
        "function ok() { return 1 }\nfunction broken( {",
    );
    assert!(!f.diagnostics.is_empty());
    assert!(f.functions.iter().any(|x| x.name == "ok"));
}

#[test]
fn deterministic() {
    let src = "def a():\n  b()\n\ndef b():\n  a()\n";
    let x = parse_file(Language::Python, "m.py", src);
    let y = parse_file(Language::Python, "m.py", src);
    assert_eq!(x, y);
}

#[test]
fn enrich_kwargs_idents_and_flow_python() {
    let src = r#"
from langchain_text_splitters import RecursiveCharacterTextSplitter

splitter = RecursiveCharacterTextSplitter(chunk_size=1000, chunk_overlap=200)

def dfs(root, target):
    stack = [root]
    while stack:
        node = stack.pop()
        if node is None:
            raise ValueError("empty")
        elif node.val == target:
            return node
        else:
            stack.append(node.left)
    return None
"#;
    let f = parse_file(Language::Python, "rag.py", src);
    let top = f
        .top_calls
        .iter()
        .find(|c| c.callee == "RecursiveCharacterTextSplitter")
        .unwrap();
    assert_eq!(top.n_args, 2);
    assert_eq!(
        top.kwargs,
        vec![
            ("chunk_size".to_owned(), "1000".to_owned()),
            ("chunk_overlap".to_owned(), "200".to_owned())
        ]
    );
    let dfs = f.functions.iter().find(|x| x.name == "dfs").unwrap();
    assert!(dfs.idents.iter().any(|i| i == "left"));
    let pop = dfs.calls.iter().find(|c| c.callee == "pop").unwrap();
    assert_eq!(pop.n_args, 0);
    use ark_schema::FlowKind::*;
    let kinds: Vec<_> = dfs.flow.iter().map(|n| n.kind).collect();
    assert_eq!(kinds, [Loop, Return], "{:#?}", dfs.flow);
    let inner: Vec<_> = dfs.flow[0].children.iter().map(|n| n.kind).collect();
    assert_eq!(inner, [Call, Branch, Else, Else], "{:#?}", dfs.flow[0]);
    assert!(dfs.flow[0].children[1].children[0].error);
}

#[test]
fn enrich_js_option_object_and_flow() {
    let src = r#"
export async function ask(q) {
  const docs = await store.similaritySearch(q, 4, { k: 4, filter });
  if (!docs.length) {
    throw new Error("no docs");
  } else if (docs.length > 10) {
    return docs.slice(0, 10);
  }
  try {
    return await llm.invoke(q);
  } catch (e) {
    console.error(e);
  }
}
"#;
    let f = parse_file(Language::TypeScript, "a.ts", src);
    let ask = &f.functions[0];
    let ss = ask
        .calls
        .iter()
        .find(|c| c.callee == "similaritySearch")
        .unwrap();
    assert_eq!(ss.n_args, 3);
    assert!(ss.kwargs.contains(&("k".to_owned(), "4".to_owned())));
    use ark_schema::FlowKind::*;
    let kinds: Vec<_> = ask.flow.iter().map(|n| n.kind).collect();
    assert_eq!(kinds, [Call, Branch, Else, Try, Catch], "{:#?}", ask.flow);
}

#[test]
fn enrich_rust_and_go_flow() {
    let rs = r#"
fn load(p: &str) -> Result<String, Error> {
    let s = std::fs::read_to_string(p)?;
    if s.is_empty() {
        return Err(Error::Empty);
    }
    match s.len() {
        0 => unreachable!(),
        _ => Ok(s),
    }
}
"#;
    let f = parse_file(Language::Rust, "a.rs", rs);
    use ark_schema::FlowKind::*;
    let kinds: Vec<_> = f.functions[0].flow.iter().map(|n| n.kind).collect();
    assert_eq!(kinds, [Call, Branch, Switch], "{:#?}", f.functions[0].flow);
    assert!(f.functions[0].flow[1].children[0].error);
    let go = r#"
package main
func get(id string) (*T, error) {
	t, err := db.Find(id)
	if err != nil {
		return nil, err
	}
	return t, nil
}
"#;
    let g = parse_file(Language::Go, "a.go", go);
    let kinds: Vec<_> = g.functions[0].flow.iter().map(|n| n.kind).collect();
    assert_eq!(kinds, [Call, Branch, Return], "{:#?}", g.functions[0].flow);
    assert!(g.functions[0].flow[1].children[0].error);
}

#[test]
fn desktop_ipc_commands_and_calls() {
    let ts = r#"
import { invoke } from "@tauri-apps/api/core";
export const status = () => invoke<string>("ai_status");
export const save = (t: string) => invoke("save_note", { t });
export const notIpc = () => invoke("Hello there, world");
"#;
    let f = parse_file(Language::TypeScript, "src/api.ts", ts);
    let calls: Vec<(&str, &str)> = f
        .http_calls
        .iter()
        .map(|c| (c.method.as_str(), c.url.as_str()))
        .collect();
    assert_eq!(calls, [("IPC", "ai_status"), ("IPC", "save_note")]);
    assert_eq!(f.http_calls[0].client, "tauri");

    let rs = r#"
#[tauri::command]
fn ai_status() -> String { String::new() }
#[tauri::command(rename_all = "snake_case")]
async fn save_note(text: String) {}
fn helper() {}
"#;
    let f = parse_file(Language::Rust, "src-tauri/src/lib.rs", rs);
    let routes: Vec<(&str, &str, &str)> = f
        .routes
        .iter()
        .map(|r| (r.method.as_str(), r.path.as_str(), r.framework.as_str()))
        .collect();
    assert_eq!(
        routes,
        [("IPC", "ai_status", "tauri"), ("IPC", "save_note", "tauri")]
    );

    let electron = r#"
const { ipcMain, ipcRenderer } = require("electron");
ipcMain.handle("notes:list", listNotes);
async function load() { return ipcRenderer.invoke("notes:list"); }
"#;
    let f = parse_file(Language::JavaScript, "main.js", electron);
    assert_eq!(f.routes.len(), 1);
    assert_eq!(f.routes[0].path, "notes:list");
    assert_eq!(f.routes[0].handler.as_deref(), Some("listNotes"));
    assert_eq!(f.http_calls[0].client, "electron");
}

#[test]
fn valid_code_the_grammar_misses_is_not_a_syntax_error() {
    let f = parse_file(
        Language::TypeScript,
        "src/gen/index.ts",
        "export type * from \"./a\";\nexport type * from \"./b\";\n",
    );
    assert!(f.syntax_errors.is_empty(), "{:?}", f.syntax_errors);
    let f = parse_file(
        Language::Tsx,
        "src/Hero.tsx",
        "export const H = () => <h2>Releases & Installers</h2>;\n",
    );
    assert!(f.syntax_errors.is_empty(), "{:?}", f.syntax_errors);
    // Real breakage is still reported.
    let f = parse_file(
        Language::Tsx,
        "src/Bad.tsx",
        "export const B = () => <h2>a & b</h2>;\nconst x = ;\n",
    );
    assert!(!f.syntax_errors.is_empty());
}

#[test]
fn wrapper_call_into_own_api_is_an_http_call() {
    let src = r#"
export const req = (path, body) => fetch(path, { method: "POST", body: JSON.stringify(body) });
export const ask = (question) => req("/api/ask", { question });
export const del = (id) => remove(`/api/items/${id}`, { method: "DELETE" });
const label = t("/api/not-a-call-but-a-key");
"#;
    let f = parse_file(Language::JavaScript, "web/api.js", src);
    let calls: Vec<_> = f
        .http_calls
        .iter()
        .map(|c| (c.method.as_str(), c.url.as_str(), c.in_fn.as_deref()))
        .collect();
    assert!(
        calls.contains(&("ANY", "/api/ask", Some("ask"))),
        "{calls:?}"
    );
    assert!(
        calls.contains(&("DELETE", "/api/items/:param", Some("del"))),
        "{calls:?}"
    );
}

#[test]
fn python_flask_restx_class_routes() {
    let src = r#"
from flask_restx import Resource, Namespace

ns = Namespace("console")

@ns.route("/apps", "/apps/list")
class AppListApi(Resource):
    def get(self):
        return []

    @ns.doc("create")
    def post(self):
        return {}

class Legacy(Resource):
    def delete(self, id):
        return {}

api.add_resource(Legacy, "/legacy/<int:id>")
"#;
    let f = parse_file(Language::Python, "controllers/apps.py", src);
    let got: Vec<(&str, &str)> = f
        .routes
        .iter()
        .map(|r| (r.method.as_str(), r.path.as_str()))
        .collect();
    assert!(got.contains(&("GET", "/apps")), "{got:?}");
    assert!(got.contains(&("POST", "/apps/list")), "{got:?}");
    assert!(got.contains(&("ANY", "/legacy/<int:id>")), "{got:?}");
    assert!(
        f.routes
            .iter()
            .any(|r| r.handler.as_deref() == Some("AppListApi.post"))
    );
}

#[test]
fn rust_actix_resource_with_scope() {
    let src = r#"
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/indexes")
            .service(web::resource("/{uid}").route(web::get().to(show)).route(web::delete().to(remove))),
    );
}
"#;
    let f = parse_file(Language::Rust, "src/routes.rs", src);
    let got: Vec<(&str, &str)> = f
        .routes
        .iter()
        .map(|r| (r.method.as_str(), r.path.as_str()))
        .collect();
    assert!(got.contains(&("GET", "/indexes/:param")), "{got:?}");
    assert!(got.contains(&("DELETE", "/indexes/:param")), "{got:?}");
}

#[test]
fn trpc_procedures_become_routes() {
    let src = "export const userRouter = createTRPCRouter({\n  list: protectedProcedure.input(z.object({})).query(async () => []),\n  create: publicProcedure.mutation(async () => 1),\n  child: otherRouter,\n});\n";
    let f = parse_file(Language::TypeScript, "server/api/user.ts", src);
    let r: Vec<_> = f
        .routes
        .iter()
        .map(|r| (r.method.as_str(), r.path.as_str()))
        .collect();
    assert_eq!(
        r,
        [
            ("QUERY", "/trpc/user.list"),
            ("MUTATION", "/trpc/user.create")
        ]
    );
}
