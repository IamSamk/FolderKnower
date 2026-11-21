import type { Todo, TodoInput } from "./types";

const BASE = import.meta.env.VITE_API_URL ?? "";

export async function listTodos(): Promise<Todo[]> {
  const res = await fetch(`${BASE}/api/todos`);
  return res.json();
}

export async function createTodo(input: TodoInput): Promise<Todo> {
  const res = await fetch(`${BASE}/api/todos`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });
  return res.json();
}

export async function completeTodo(id: number): Promise<Todo> {
  const res = await fetch(`${BASE}/api/todos/${id}/complete`, { method: "PATCH" });
  return res.json();
}

export async function login(email: string, password: string): Promise<string> {
  const res = await fetch(`${BASE}/api/auth/login`, {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
  const data = await res.json();
  localStorage.setItem("token", data.access_token);
  return data.access_token;
}
