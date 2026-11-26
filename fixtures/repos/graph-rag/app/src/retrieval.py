from .config import MAX_ROUNDS, RRF_K, TOP_RERANK
from .embed_client import embed
from .graph import expand_entities
from .llm.client import chat
from .pg_store import dense_search, lexical_search
from .rerank import rerank


def rrf_fuse(lists, k=RRF_K):
    scores = {}
    for lst in lists:
        for rank, cid in enumerate(lst):
            scores[cid] = scores.get(cid, 0) + 1 / (k + rank + 1)
    return sorted(scores, key=scores.get, reverse=True)


def evidence_is_sufficient(question, passages):
    verdict = chat([{"role": "user", "content": f"Is this enough to answer {question}? {passages}"}])
    return verdict.strip().lower().startswith("yes")


def answer(conn, question):
    passages = []
    for _ in range(MAX_ROUNDS):
        dense = dense_search(conn, embed([question])[0])
        lexical = expand_entities(conn, lexical_search(conn, question))
        passages = rerank(question, rrf_fuse([lexical, dense]))[:TOP_RERANK]
        if evidence_is_sufficient(question, passages):
            break
    return chat([{"role": "user", "content": f"Answer with citations: {question} {passages}"}])
