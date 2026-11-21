import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { completeTodo, createTodo, listTodos } from "../lib/api";
import type { Todo } from "../lib/types";

export function TodoList() {
  const qc = useQueryClient();
  const [title, setTitle] = useState("");
  const todos = useQuery({ queryKey: ["todos"], queryFn: listTodos });
  const add = useMutation({ mutationFn: createTodo, onSuccess: () => qc.invalidateQueries() });

  const handleAdd = () => {
    if (title.trim().length === 0) return;
    add.mutate({ title });
    setTitle("");
  };

  return (
    <section>
      <input placeholder="What needs doing?" value={title} onChange={(e) => setTitle(e.target.value)} />
      <button onClick={handleAdd}>Add todo</button>
      <ul>
        {(todos.data ?? []).map((t: Todo) => (
          <TodoRow key={t.id} todo={t} />
        ))}
      </ul>
    </section>
  );
}

function TodoRow({ todo }: { todo: Todo }) {
  return (
    <li>
      <span>{todo.title}</span>
      <button onClick={() => completeTodo(todo.id)}>Done</button>
    </li>
  );
}
