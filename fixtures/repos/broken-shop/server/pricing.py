"""Price maths. The second function is missing a closing bracket."""


def total(items: list[dict]) -> float:
    return sum(i["price"] * i.get("qty", 1) for i in items)


def discount(items: list[dict], pct: float) -> float:
    return total(items) * (1 - pct / 100
