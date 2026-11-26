"""Shop API."""
from fastapi import FastAPI
import stripe

from .pricing import total
from .tax import vat_for

app = FastAPI()


@app.get("/api/items")
def list_items() -> list[dict]:
    return [{"id": 1, "name": "Mug", "price": 9.5}]


@app.post("/api/checkout")
def checkout(body: dict) -> dict:
    amount = total(body["items"]) + vat_for(body["country"])
    intent = stripe.PaymentIntent.create(amount=int(amount * 100), currency="eur")
    return {"id": intent.id}
