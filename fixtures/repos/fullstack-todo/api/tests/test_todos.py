from app.routers.todos import rank_todos


def test_rank_empty():
    assert rank_todos([]) == []
