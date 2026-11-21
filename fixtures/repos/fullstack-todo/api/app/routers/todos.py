from fastapi import APIRouter, Depends, HTTPException

from ..core.db import get_db
from ..core.security import auth_middleware
from ..models import Todo
from ..schemas import TodoCreate, TodoOut
from worker.tasks import send_digest

router = APIRouter(dependencies=[Depends(auth_middleware)])


@router.get("/todos", response_model=list[TodoOut])
def list_todos(db=Depends(get_db)):
    return db.query(Todo).order_by(Todo.id).all()


@router.post("/todos", response_model=TodoOut)
def create_todo(body: TodoCreate, db=Depends(get_db)) -> TodoOut:
    todo = Todo(title=body.title)
    db.add(todo)
    db.commit()
    send_digest.delay(todo.owner_id)
    return todo


@router.patch("/todos/{todo_id}/complete", response_model=TodoOut)
def complete_todo(todo_id: int, db=Depends(get_db)) -> TodoOut:
    todo = db.get(Todo, todo_id)
    if todo is None:
        raise HTTPException(status_code=404)
    todo.done = True
    db.commit()
    return todo


def rank_todos(todos: list) -> list:
    out = []
    for a in todos:
        for b in todos:
            if a.title < b.title:
                out.append((a, b))
    return sorted(out)
