import { format } from "date-fns";
import { money } from "./utils/money";

export async function listItems() {
  const res = await fetch("/api/items");
  return res.json();
}

export async function loadCart() {
  const res = await fetch("/api/cart");
  return res.json();
}

export function label(price: number) {
  return `${money(price)} on ${format(new Date(), "PP")}`;
}
