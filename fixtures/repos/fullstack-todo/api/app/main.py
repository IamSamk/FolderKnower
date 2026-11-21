from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from .core.db import Base, engine
from .routers import auth, todos

app = FastAPI(title="Todo API")
app.add_middleware(CORSMiddleware, allow_origins=["*"])
app.include_router(todos.router, prefix="/api")
app.include_router(auth.router, prefix="/api")


@app.on_event("startup")
def create_tables():
    Base.metadata.create_all(bind=engine)
