import { useState } from "react";

const API = import.meta.env.VITE_API_URL ?? "/api";

export default function App() {
  const [question, setQuestion] = useState("");
  const [answer, setAnswer] = useState("");

  async function ask() {
    const res = await fetch(`${API}/ask`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ question, session: "demo" }),
    });
    const data = await res.json();
    setAnswer(data.answer);
  }

  return (
    <main>
      <h1>Ask the book</h1>
      <input value={question} onChange={(e) => setQuestion(e.target.value)} placeholder="What is chunk overlap?" />
      <button type="button" onClick={ask}>Ask</button>
      <p>{answer}</p>
    </main>
  );
}
