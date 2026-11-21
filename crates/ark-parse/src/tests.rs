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
