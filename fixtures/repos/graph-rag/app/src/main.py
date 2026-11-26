from fastapi import FastAPI

from .retrieval import answer

app = FastAPI()


@app.post("/api/ask")
def ask(q: dict):
    return {"answer": answer(None, q["question"])}
