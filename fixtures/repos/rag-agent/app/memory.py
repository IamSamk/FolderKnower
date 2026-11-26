"""Conversation memory: the last turns live in Redis, answers are cached by question."""
import hashlib
import json
from functools import lru_cache

import redis

r = redis.Redis.from_url("redis://cache:6379/0")

HISTORY_TURNS = 6
ANSWER_TTL = 3600


def remember(session: str, role: str, text: str) -> None:
    key = f"chat:{session}"
    r.rpush(key, json.dumps({"role": role, "text": text}))
    r.ltrim(key, -HISTORY_TURNS * 2, -1)


def history(session: str) -> list[dict]:
    return [json.loads(x) for x in r.lrange(f"chat:{session}", 0, -1)]


def cached_answer(question: str) -> str | None:
    hit = r.get("ans:" + hashlib.sha256(question.encode()).hexdigest())
    return hit.decode() if hit else None


def store_answer(question: str, answer: str) -> None:
    r.setex("ans:" + hashlib.sha256(question.encode()).hexdigest(), ANSWER_TTL, answer)


@lru_cache(maxsize=1024)
def normalize(question: str) -> str:
    return " ".join(question.lower().split())
