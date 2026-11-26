use std::sync::Mutex;

use notes_core::{Note, NoteStore};
use tauri::{Manager, State};

#[derive(Default)]
struct Db(Mutex<Option<NoteStore>>);

#[tauri::command]
fn save_note(db: State<'_, Db>, text: String) -> Result<Note, String> {
    let mut g = db.0.lock().map_err(|e| e.to_string())?;
    let store = g.as_mut().ok_or("not open")?;
    store.add(&text).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_notes(db: State<'_, Db>) -> Result<Vec<Note>, String> {
    let g = db.0.lock().map_err(|e| e.to_string())?;
    g.as_ref().ok_or("not open")?.all().map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_note(db: State<'_, Db>, id: i64) -> Result<(), String> {
    let g = db.0.lock().map_err(|e| e.to_string())?;
    g.as_ref().ok_or("not open")?.remove(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

pub fn run() {
    tauri::Builder::default()
        .manage(Db::default())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let store = NoteStore::open(&dir.join("notes.sqlite"))?;
            *app.state::<Db>().0.lock().unwrap() = Some(store);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![save_note, list_notes, delete_note, app_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
