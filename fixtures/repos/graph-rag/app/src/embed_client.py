import os

import httpx

BASE = os.environ["EMBED_URL"]


def embed(texts):
    r = httpx.post(f"{BASE}/v1/embeddings", json={"input": texts})
    return [d["embedding"] for d in r.json()["data"]]
