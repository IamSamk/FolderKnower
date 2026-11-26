import { useState } from "react";

const API = import.meta.env.VITE_API_URL ?? "/api";

export default function App() {
  const [question, setQuestion] = useState("");
  const [answer, setAnswer] = useState("");

  const [turns, setTurns] = useState<{ role: string; text: string }[]>([]);

  async function ask() {
    localStorage.setItem("lastQuestion", question);
    const res = await fetch(`${API}/ask`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ question, session: "demo" }),
    });
    const data = await res.json();
    setAnswer(data.answer);
  }

  async function showHistory() {
    const res = await fetch(`${API}/history/demo`);
    setTurns(await res.json());
  }

  return (
    <main>
      <h1>Ask the book</h1>
      <input value={question} onChange={(e) => setQuestion(e.target.value)} placeholder="What is chunk overlap?" />
      <button type="button" onClick={ask}>Ask</button>
      <p>{answer}</p>
      <button type="button" onClick={showHistory}>Show history</button>
      <ul>{turns.map((t) => <li key={t.text}>{t.role}: {t.text}</li>)}</ul>
    </main>
  );
}
