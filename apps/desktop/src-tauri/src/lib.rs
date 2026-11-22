//! Tauri shell: exposes the offline indexer, confined source reads, the ontology
//! catalogue and the optional local-model layer to the UI.

mod ai;
mod slm;

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use ark_core::{Options, Progress, RunStats, analyze as run_analyze, ark_schema};
use ark_schema::{SystemArchitectureGraph, TierLabel};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzeResult {
    graph: SystemArchitectureGraph,
    stats: RunStats,
    root: String,
}

#[tauri::command]
async fn analyze(
    app: AppHandle,
    path: String,
    overrides: Option<BTreeMap<String, TierLabel>>,
) -> Result<AnalyzeResult, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("{path} is not a folder"));
    }
    let opts = Options {
        overrides: overrides.unwrap_or_default(),
        ..Options::default()
    };
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let progress = |p: Progress| {
            let _ = app2.emit("analyze-progress", p);
        };
        run_analyze(&root, &opts, &progress)
            .map(|(graph, stats)| AnalyzeResult {
                graph,
                stats,
                root: root.to_string_lossy().into_owned(),
            })
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Resolve `rel` inside `root`, refusing anything that escapes it.
fn confined(root: &str, rel: &str) -> Result<PathBuf, String> {
    let root = Path::new(root)
        .canonicalize()
        .map_err(|e| format!("root: {e}"))?;
    let full = root
        .join(rel)
        .canonicalize()
        .map_err(|e| format!("{rel}: {e}"))?;
    if !full.starts_with(&root) {
        return Err("path escapes the analysed folder".to_owned());
    }
    Ok(full)
}

#[tauri::command]
fn read_source(root: String, path: String) -> Result<String, String> {
    let full = confined(&root, &path)?;
    let meta = std::fs::metadata(&full).map_err(|e| e.to_string())?;
    if meta.len() > 4 * 1024 * 1024 {
        return Err("file too large to preview".to_owned());
    }
    let bytes = std::fs::read(&full).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Concept {
    id: String,
    layer: ark_schema::OntologyLayer,
    concept: String,
    explain: String,
}

#[tauri::command]
fn ontology_catalog() -> Vec<Concept> {
    ark_ontology::Ontology::load()
        .rules
        .into_iter()
        .map(|r| Concept {
            id: r.id,
            layer: r.layer,
            concept: r.concept,
            explain: r.explain,
        })
        .collect()
}

#[tauri::command]
fn export_json(path: String, graph: SystemArchitectureGraph) -> Result<(), String> {
    let json = ark_schema::to_canonical_json(&graph).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

/// Update info returned to the frontend.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateInfo {
    version: String,
    body: Option<String>,
    date: Option<String>,
}

/// Check GitHub Releases for a newer version. Returns `null` when already up to date.
#[tauri::command]
async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let updater = app.updater_builder().build().map_err(|e| e.to_string())?;
    match updater.check().await.map_err(|e| e.to_string())? {
        Some(update) => Ok(Some(UpdateInfo {
            version: update.version.clone(),
            body: update.body.clone(),
            date: update.date.map(|d| d.to_string()),
        })),
        None => Ok(None),
    }
}

/// Download and silently install the pending update (restarts the app).
#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    let updater = app.updater_builder().build().map_err(|e| e.to_string())?;
    if let Some(update) = updater.check().await.map_err(|e| e.to_string())? {
        update
            .download_and_install(|_chunk, _total| {}, || {})
            .await
            .map_err(|e| e.to_string())?;
        app.restart();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .manage(ai::AiState::default())
        .invoke_handler(tauri::generate_handler![
            analyze,
            read_source,
            ontology_catalog,
            export_json,
            check_update,
            install_update,
            slm::slm_status,
            slm::slm_explain,
            slm::slm_classify,
            ai::ai_status,
            ai::ai_install,
            ai::ai_remove,
            ai::ai_start_narrator,
            ai::ai_decide,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Architekton");
    app.run(|handle, event| {
        if let tauri::RunEvent::Exit = event {
            handle.state::<ai::AiState>().shutdown();
        }
    });
}
