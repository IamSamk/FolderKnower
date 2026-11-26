# tauri-notes

A tiny Tauri desktop app used by the tests: a React window calls Rust commands with
`invoke`, the commands use a workspace crate that keeps notes in SQLite. One call has a
typo (`delete_nte`) so the Problems check has something to report.
