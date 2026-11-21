import { useState } from "react";
import { login } from "../lib/api";

export function LoginForm({ onDone }: { onDone: (token: string) => void }) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");

  async function submit() {
    const token = await login(email, password);
    onDone(token);
  }

  return (
    <form onSubmit={submit}>
      <input placeholder="Email" onChange={(e) => setEmail(e.target.value)} />
      <input placeholder="Password" type="password" onChange={(e) => setPassword(e.target.value)} />
      <button type="submit">Sign in</button>
    </form>
  );
}
