import { useState } from "react";
import { ask } from "./lib/api";

export default function App() {
  const [q, setQ] = useState("");
  const [a, setA] = useState("");
  async function onAsk() {
    const r = await ask(q);
    setA((await r.json()).answer);
  }
  return (
    <div>
      <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Ask the books" />
      <button type="button" onClick={onAsk}>Ask</button>
      <p>{a}</p>
    </div>
  );
}
