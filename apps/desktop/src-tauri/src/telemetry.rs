//! Live telemetry commands: system + process memory/CPU/threads (unprivileged) and
//! local model-server status (Ollama, llama-server, vLLM) for the Live lens.

use std::{sync::Mutex, time::Duration};

use ark_telemetry::{
    ProcessDetail, ProcessRow, Sampler, SortBy, SystemSnapshot,
    runtime::{self, RuntimeStatus},
};
use tauri::State;

#[derive(Default)]
pub struct TelemetryState(Mutex<Option<Sampler>>);

fn with<T>(state: &TelemetryState, f: impl FnOnce(&mut Sampler) -> T) -> Result<T, String> {
    let mut g = state
        .0
        .lock()
        .map_err(|_| "telemetry lock poisoned".to_owned())?;
    if g.is_none() {
        *g = Sampler::new();
    }
    let s = g
        .as_mut()
        .ok_or("system telemetry is not available on this platform")?;
    Ok(f(s))
}

/// Refresh and return the machine-wide snapshot. The UI polls this about once a second.
#[tauri::command]
pub fn telemetry_system(state: State<'_, TelemetryState>) -> Result<SystemSnapshot, String> {
    with(&state, |s| {
        s.refresh();
        s.system()
    })
}

#[tauri::command]
pub fn telemetry_processes(
    state: State<'_, TelemetryState>,
    limit: Option<usize>,
    sort: Option<String>,
) -> Result<Vec<ProcessRow>, String> {
    let sort = if sort.as_deref() == Some("cpu") {
        SortBy::Cpu
    } else {
        SortBy::Memory
    };
    with(&state, |s| s.processes(limit.unwrap_or(40).min(500), sort))
}

#[tauri::command]
pub fn telemetry_process(
    state: State<'_, TelemetryState>,
    pid: u32,
) -> Result<Option<ProcessDetail>, String> {
    with(&state, |s| s.process(pid))
}

#[tauri::command]
pub fn telemetry_find(
    state: State<'_, TelemetryState>,
    needle: String,
) -> Result<Vec<ProcessRow>, String> {
    with(&state, |s| s.find(&needle, 20))
}

fn get(agent: &ureq::Agent, url: &str) -> Option<String> {
    agent.get(url).call().ok()?.body_mut().read_to_string().ok()
}

/// Probe well-known local model servers on 127.0.0.1. Each probe has a short timeout,
/// and nothing leaves the machine.
#[tauri::command]
pub async fn telemetry_runtimes() -> Result<Vec<RuntimeStatus>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_millis(350)))
            .http_status_as_error(true)
            .build()
            .into();
        let mut out = Vec::new();
        if let Some(b) = get(&agent, "http://127.0.0.1:11434/api/ps")
            && let Some(mut s) = runtime::parse_ollama_ps(&b)
        {
            s.endpoint = "http://127.0.0.1:11434".into();
            out.push(s);
        }
        for port in [8080u16, 8081] {
            let base = format!("http://127.0.0.1:{port}");
            if get(&agent, &format!("{base}/health")).is_none() {
                continue;
            }
            let metrics = get(&agent, &format!("{base}/metrics"));
            if metrics.as_deref().is_some_and(|m| m.contains("vllm:")) {
                let mut s = runtime::parse_vllm_metrics(metrics.as_deref().unwrap_or_default());
                s.endpoint = base;
                out.push(s);
                continue;
            }
            let slots = get(&agent, &format!("{base}/slots"));
            let mut s = runtime::parse_llama_server(metrics.as_deref(), slots.as_deref());
            s.endpoint = base;
            out.push(s);
        }
        if let Some(m) = get(&agent, "http://127.0.0.1:8000/metrics")
            && m.contains("vllm:")
        {
            let mut s = runtime::parse_vllm_metrics(&m);
            s.endpoint = "http://127.0.0.1:8000".into();
            out.push(s);
        }
        out
    })
    .await
    .map_err(|e| e.to_string())
}
