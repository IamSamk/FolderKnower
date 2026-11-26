EXPAND = """
SELECT DISTINCT m2.page_id FROM mentions m1
JOIN relations r ON r.src = m1.entity_id
JOIN mentions m2 ON m2.entity_id = r.dst
WHERE m1.page_id = ANY(%s)
"""


def expand_entities(conn, seed_pages):
    return seed_pages + [r[0] for r in conn.execute(EXPAND, (seed_pages,))]
