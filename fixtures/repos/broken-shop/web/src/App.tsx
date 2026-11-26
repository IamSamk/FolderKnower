import { useEffect, useState } from "react";
import { listItems, loadCart } from "./api";

export default function App() {
  const [items, setItems] = useState<{ id: number; name: string }[]>([]);
  useEffect(() => {
    listItems().then(setItems);
  }, []);
  return (
    <main>
      <ul>{items.map((i) => <li key={i.id}>{i.name}</li>)}</ul>
      <button type="button" onClick={() => loadCart()}>Open cart</button>
    </main>
  );
}
