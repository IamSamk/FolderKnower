import psycopg

DENSE = "SELECT chunk_id FROM chunks ORDER BY embedding <=> %s::vector LIMIT 50"
LEXICAL = "SELECT page_id, ts_rank(tsv, q) AS r FROM pages, plainto_tsquery('simple', %s) q WHERE tsv @@ q ORDER BY r DESC LIMIT 20"


def dense_search(conn, qvec):
    return [r[0] for r in conn.execute(DENSE, (qvec,))]


def lexical_search(conn, question):
    return [r[0] for r in conn.execute(LEXICAL, (question,))]


def insert_chunks(chunks, vectors):
    pass
