"""Hybrid retrieval: dense + BM25, fused with RRF, then reranked by a cross-encoder."""
from langchain.retrievers import EnsembleRetriever
from sentence_transformers import CrossEncoder

TOP_K = 8
TOP_N = 3

reranker = CrossEncoder("cross-encoder/ms-marco-MiniLM-L-6-v2")


def hybrid_search(store, keywords, question: str):
    dense = store.as_retriever(search_type="similarity", search_kwargs={"k": 8})
    fused = EnsembleRetriever(retrievers=[dense, keywords], weights=[0.6, 0.4])
    candidates = fused.invoke(question)
    if not candidates:
        return []
    scores = reranker.predict([(question, c.page_content) for c in candidates])
    ranked = sorted(zip(scores, candidates), key=lambda p: p[0], reverse=True)
    return [doc for _, doc in ranked[:TOP_N]]
