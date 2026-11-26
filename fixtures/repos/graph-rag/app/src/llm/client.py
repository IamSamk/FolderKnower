import os

import httpx

from ..config import CHAT_MODEL

BASE = os.environ["LLM_URL"]


def chat(messages):
    r = httpx.post(f"{BASE}/v1/chat/completions", json={"model": CHAT_MODEL, "messages": messages})
    return r.json()["choices"][0]["message"]["content"]
