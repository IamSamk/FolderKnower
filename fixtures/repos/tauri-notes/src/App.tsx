import { useState } from "react";
import { appVersion, deleteNote, listNotes, saveNote } from "./api";
import type { Note } from "./types";

export function App() {
  const [notes, setNotes] = useState<Note[]>([]);
  const [text, setText] = useState("");
  const [version, setVersion] = useState("");
  const refresh = async () => setNotes(await listNotes());
  const save = async () => {
    localStorage.setItem("draft", "");
    await saveNote(text);
    await refresh();
  };
  return (
    <main>
      <h1>Notes & Ideas</h1>
      <input value={text} onChange={(e) => setText(e.target.value)} />
      <button type="button" onClick={save}>
        Save
      </button>
      <button type="button" onClick={refresh}>
        Refresh
      </button>
      <button type="button" onClick={async () => setVersion(await appVersion())}>
        About {version}
      </button>
      {notes.map((n) => (
        <button key={n.id} type="button" onClick={() => deleteNote(n.id)}>
          Delete {n.text}
        </button>
      ))}
    </main>
  );
}
