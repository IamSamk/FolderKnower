"""HTTP entry point: the web page posts a question here."""
from fastapi import FastAPI
from pydantic import BaseModel

from .agent import app as agent
from .memory import cached_answer, history, normalize, remember, store_answer

app = FastAPI()


class Ask(BaseModel):
    question: str
    session: str


@app.post("/ask")
def ask(body: Ask) -> dict:
    q = normalize(body.question)
    hit = cached_answer(q)
    if hit:
        return {"answer": hit, "cached": True}
    remember(body.session, "user", body.question)
    out = agent.invoke({"question": q, "history": history(body.session)})
    answer = out["messages"][-1].content
    remember(body.session, "assistant", answer)
    store_answer(q, answer)
    return {"answer": answer, "cached": False}
