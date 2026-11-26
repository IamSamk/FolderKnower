from sentence_transformers import CrossEncoder

from .config import RERANK_MODEL

model = CrossEncoder(RERANK_MODEL)


def rerank(question, passages):
    scores = model.predict([(question, p) for p in passages])
    return [p for _, p in sorted(zip(scores, passages), reverse=True)]
