export interface Todo {
  id: number;
  title: string;
  done: boolean;
  ownerId: number;
}

export interface TodoInput {
  title: string;
}
