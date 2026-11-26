import { invoke } from "@tauri-apps/api/core";
import type { Note } from "./types";

export function saveNote(text: string): Promise<Note> {
  return invoke<Note>("save_note", { text });
}

export function listNotes(): Promise<Note[]> {
  return invoke<Note[]>("list_notes");
}

export function deleteNote(id: number): Promise<void> {
  return invoke("delete_nte", { id });
}

export function appVersion(): Promise<string> {
  return invoke<string>("app_version");
}
