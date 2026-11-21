import { useState } from "react";
import { LoginForm } from "./components/LoginForm";
import { TodoList } from "./components/TodoList";

export default function App() {
  const [token, setToken] = useState<string | null>(null);
  return token ? <TodoList /> : <LoginForm onDone={setToken} />;
}
