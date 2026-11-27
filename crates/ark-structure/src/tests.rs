use super::*;
use ark_link::{LinkInputs, ParsedFile, link};
use ark_scan::{Language, ScannedFile};

fn pf(path: &str, src: &str) -> ParsedFile {
    let lang = Language::from_path(path).unwrap();
    ParsedFile {
        scan: ScannedFile {
            path: path.to_owned(),
            size: src.len() as u64,
            hash: format!("{:x}", src.len()),
            language: Some(lang),
            other: None,
        },
        facts: ark_parse::parse_file(lang, path, src),
    }
}

fn fixture() -> Vec<ParsedFile> {
    let mut v = vec![
        // UI lives in a folder called "shared" - the folder name must not win.
        pf(
            "shared/TodoList.tsx",
            r#"import React from "react";
import { addTodo } from "../client/api";
export function TodoList() { return <button onClick={() => addTodo("x")}>Add</button>; }"#,
        ),
        pf(
            "client/api.ts",
            r#"export async function addTodo(t: string) { return fetch("/api/todos", { method: "POST" }); }"#,
        ),
        pf(
            "client/format.ts",
            r#"import { addTodo } from "./api";
export function fmt(s: string) { return s.trim(); }"#,
        ),
        // Backend hidden in a folder called "web".
        pf(
            "web/routes.py",
            r#"from fastapi import APIRouter
from .models import Todo
from .schemas import TodoCreate
router = APIRouter()
@router.post("/todos")
def create(body: TodoCreate, db):
    db.add(Todo(title=body.title))
    db.commit()
"#,
        ),
        pf(
            "web/models.py",
            r#"from sqlalchemy import Column, Integer, String, ForeignKey
from sqlalchemy.orm import relationship
from .db import Base
class User(Base):
    __tablename__ = "users"
    id = Column(Integer, primary_key=True)
    todos = relationship("Todo")
class Todo(Base):
    __tablename__ = "todos"
    id = Column(Integer, primary_key=True)
    title = Column(String, nullable=False)
    owner_id = Column(Integer, ForeignKey("users.id"))
"#,
        ),
        pf(
            "web/schemas.py",
            "from pydantic import BaseModel\nclass TodoCreate(BaseModel):\n    title: str\n    owner_id: int\n",
        ),
        pf(
            "jobs/emails.py",
            "from celery import Celery\napp = Celery()\n@app.task\ndef send_digest(user_id):\n    pass\n",
        ),
    ];
    v.sort_by(|a, b| a.scan.path.cmp(&b.scan.path));
    v
}

#[test]
fn tiers_are_semantic_not_folder_based() {
    let files = fixture();
    let linked = link(&files, &LinkInputs::default());
    let s = build(&StructureInput {
        files: &files,
        linked: &linked,
        schema_sources: &[],
        overrides: &BTreeMap::new(),
    });
    let t = |p: &str| s.file_tiers[p].tier;
    assert_eq!(t("shared/TodoList.tsx"), TierLabel::Frontend);
    assert_eq!(t("client/api.ts"), TierLabel::Frontend);
    assert_eq!(
        t("client/format.ts"),
        TierLabel::Frontend,
        "propagated from import neighbour"
    );
    assert_eq!(t("web/routes.py"), TierLabel::Backend);
    assert_eq!(t("web/models.py"), TierLabel::Storage);
    assert_eq!(t("jobs/emails.py"), TierLabel::Workers);
    assert_eq!(s.summary.kind, ProjectKind::FullStack);
    assert!(
        s.summary.headline.contains("FastAPI"),
        "{}",
        s.summary.headline
    );
    assert_eq!(
        s.file_tiers["web/routes.py"].role.as_deref(),
        Some("route-handler")
    );
}

#[test]
fn er_models_relations_mirrors_and_db_edges() {
    let files = fixture();
    let linked = link(&files, &LinkInputs::default());
    let s = build(&StructureInput {
        files: &files,
        linked: &linked,
        schema_sources: &[(
            "db/schema.sql".to_owned(),
            "CREATE TABLE tags (id SERIAL PRIMARY KEY, todo_id INT NOT NULL REFERENCES todos(id));"
                .to_owned(),
        )],
        overrides: &BTreeMap::new(),
    });
    let todo = s.data_models.iter().find(|m| m.name == "Todo").unwrap();
    assert_eq!(todo.table.as_deref(), Some("todos"));
    assert_eq!(todo.relations[0].target, "model:web/models.py#User");
    assert_eq!(todo.relations[0].kind, ark_schema::RelationKind::ManyToOne);
    assert!(todo.fields.iter().any(|f| f.name == "id" && f.primary_key));
    assert!(
        todo.mirrors
            .contains(&"model:web/schemas.py#TodoCreate".to_owned())
    );
    let tags = s.data_models.iter().find(|m| m.name == "tags").unwrap();
    assert_eq!(tags.relations[0].target, "model:web/models.py#Todo");
    assert!(
        s.db_edges.iter().any(
            |e| e.source == "fn:web/routes.py#create" && e.target == "model:web/models.py#Todo"
        )
    );
}

#[test]
fn overrides_and_determinism() {
    let files = fixture();
    let linked = link(&files, &LinkInputs::default());
    let mut ov = BTreeMap::new();
    ov.insert("jobs/".to_owned(), TierLabel::Backend);
    let a = build(&StructureInput {
        files: &files,
        linked: &linked,
        schema_sources: &[],
        overrides: &ov,
    });
    assert_eq!(a.file_tiers["jobs/emails.py"].tier, TierLabel::Backend);
    assert_eq!(a.file_tiers["jobs/emails.py"].confidence, 1.0);
    let b = build(&StructureInput {
        files: &files,
        linked: &linked,
        schema_sources: &[],
        overrides: &ov,
    });
    assert_eq!(format!("{:?}", a.tiers), format!("{:?}", b.tiers));
}

#[test]
fn proto_messages_become_schema_models() {
    let src = "syntax = \"proto3\";\nmessage Money {\n  string currency_code = 1;\n  int64 units = 2;\n}\nmessage CartItem {\n  string product_id = 1;\n  repeated Money prices = 2;\n  message Inner { int32 x = 1; }\n  optional string note = 3;\n}\n";
    let m = crate::models::proto_models("pb/demo.proto", src);
    assert_eq!(m.len(), 3);
    let cart = &m.iter().find(|(d, _)| d.name == "CartItem").unwrap().0;
    assert_eq!(cart.fields.len(), 2);
    assert!(cart.fields.iter().any(|f| f.name == "note" && f.nullable));
    let rels = &m.iter().find(|(d, _)| d.name == "CartItem").unwrap().1;
    assert_eq!(rels[0].0, "Money");
}
