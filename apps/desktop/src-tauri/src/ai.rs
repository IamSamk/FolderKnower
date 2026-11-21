//! Built-in local AI, so users don't need Ollama.
//!
//! Optional components, installed only when the user opts in from the setup screen:
//! - `runtime`: llama.cpp `llama-server` (MIT), pinned release, CPU build.
//! - `decider`: Laya (Apache-2.0), a non-generative decision model served on
//!   `/v1/systemone`. It answers typed questions with calibrated probabilities, so it
//!   can only ever pick one of the options we give it.
//! - `narrator`: Qwen2.5-0.5B-Instruct Q4_K_M (Apache-2.0) for short, cited
//!   explanations through the existing grounded `slm_explain` path.
//!
//! Every download is pinned to an exact file and verified with SHA-256. Servers bind to
//! 127.0.0.1 only and are killed when the app exits.

use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager, State};

pub const LLAMA_BUILD: &str = "b12606";

struct Artifact {
    url: &'static str,
    sha256: &'static str,
    size: u64,
}

fn runtime_artifact() -> Option<Artifact> {
    let (os, arch) = (std::env::consts::OS, std::env::consts::ARCH);
    Some(match (os, arch) {
        ("windows", "x86_64") => Artifact {
            url: "https://github.com/ggml-org/llama.cpp/releases/download/b12606/llama-b12606-bin-win-cpu-x64.zip",
            sha256: "425eeb2a94a9600dadf369a7c9c7dd0141948921efc0fa9ec7872970fffc845d",
            size: 25_009_049,
        },
        ("macos", "aarch64") => Artifact {
            url: "https://github.com/ggml-org/llama.cpp/releases/download/b12606/llama-b12606-bin-macos-arm64.tar.gz",
            sha256: "8d277d8a1289202e0553cf6489801303624973807ae6d07c666a61ea70a0e1c4",
            size: 16_576_375,
        },
        ("macos", "x86_64") => Artifact {
            url: "https://github.com/ggml-org/llama.cpp/releases/download/b12606/llama-b12606-bin-macos-x64.tar.gz",
            sha256: "cff40de586fc6979013d4ea0b5617e32c2bd181c11386f321aa96ba022920c21",
            size: 12_362_291,
        },
        ("linux", "x86_64") => Artifact {
            url: "https://github.com/ggml-org/llama.cpp/releases/download/b12606/llama-b12606-bin-ubuntu-x64.tar.gz",
            sha256: "23ad17d2052e35a7c0a16a7d84a46b3f44c0f90d8ef438e27712c1c158321024",
            size: 23_185_300,
        },
        _ => return None,
    })
}

const DECIDER: Artifact = Artifact {
    url: "https://huggingface.co/ggml-org/Laya-GGUF/resolve/main/Laya-Q8_0.gguf",
    sha256: "ac6a51fd7a7907bc9dd61e85f7e6b989f57a635e2232a8f0345f5c51ca6cb3ea",
    size: 449_397_728,
};

const NARRATOR: Artifact = Artifact {
    url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf",
    sha256: "74a4da8c9fdbcd15bd1f6d01d621410d31c6fc00986f5eb687824e7b93d7a9db",
    size: 491_400_032,
};

const DECIDER_PORT: u16 = 39_931;
const NARRATOR_PORT: u16 = 39_932;

#[derive(Default)]
pub struct AiState {
    servers: Mutex<BTreeMap<&'static str, Child>>,
}

impl AiState {
    pub fn shutdown(&self) {
        if let Ok(mut s) = self.servers.lock() {
            for (_, mut c) in std::mem::take(&mut *s) {
                let _ = c.kill();
                let _ = c.wait();
            }
        }
    }
}

fn ai_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let d = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("ai");
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

fn server_exe(dir: &Path) -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    };
    let mut stack = vec![dir.join("runtime")];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).ok()?.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().is_some_and(|n| n == name) {
                return Some(p);
            }
        }
    }
    None
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    id: &'static str,
    name: &'static str,
    purpose: &'static str,
    license: &'static str,
    license_url: &'static str,
    source_url: &'static str,
    size: u64,
    installed: bool,
    supported: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStatus {
    components: Vec<Component>,
    decider_ready: bool,
    narrator_ready: bool,
    narrator_base_url: Option<String>,
}

fn status(app: &AppHandle) -> Result<AiStatus, String> {
    let dir = ai_dir(app)?;
    let rt = runtime_artifact();
    let runtime = server_exe(&dir).is_some();
    let decider = dir.join("models/laya-q8_0.gguf").is_file();
    let narrator = dir
        .join("models/qwen2.5-0.5b-instruct-q4_k_m.gguf")
        .is_file();
    Ok(AiStatus {
        components: vec![
            Component {
                id: "runtime",
                name: "llama.cpp runtime",
                purpose: "Runs the models below on your CPU, on this computer only.",
                license: "MIT",
                license_url: "https://github.com/ggml-org/llama.cpp/blob/master/LICENSE",
                source_url: "https://github.com/ggml-org/llama.cpp",
                size: rt.as_ref().map(|a| a.size).unwrap_or(0),
                installed: runtime,
                supported: rt.is_some(),
            },
            Component {
                id: "decider",
                name: "Laya decision model",
                purpose: "Settles folders the rules were unsure about. It picks one of the given answers with a probability and never writes free text.",
                license: "Apache-2.0",
                license_url: "https://huggingface.co/convaiinnovations/laya",
                source_url: "https://huggingface.co/ggml-org/Laya-GGUF",
                size: DECIDER.size,
                installed: decider,
                supported: rt.is_some(),
            },
            Component {
                id: "narrator",
                name: "Qwen2.5 0.5B narrator",
                purpose: "Explains what you selected in plain language, citing the parts of the graph it used.",
                license: "Apache-2.0",
                license_url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct",
                source_url: "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF",
                size: NARRATOR.size,
                installed: narrator,
                supported: rt.is_some(),
            },
        ],
        decider_ready: runtime && decider,
        narrator_ready: runtime && narrator,
        narrator_base_url: (runtime && narrator)
            .then(|| format!("http://127.0.0.1:{NARRATOR_PORT}")),
    })
}

#[tauri::command]
pub fn ai_status(app: AppHandle) -> Result<AiStatus, String> {
    status(&app)
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct InstallProgress {
    id: String,
    done: u64,
    total: u64,
    phase: &'static str,
}

fn download(app: &AppHandle, id: &str, a: &Artifact, dest: &Path) -> Result<(), String> {
    let part = dest.with_extension("part");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(20)))
        .http_status_as_error(true)
        .build()
        .into();
    let mut resp = agent
        .get(a.url)
        .call()
        .map_err(|e| format!("download failed: {e}"))?;
    let mut reader = resp.body_mut().with_config().limit(a.size + 1024).reader();
    let mut file = fs::File::create(&part).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    let mut done = 0u64;
    let mut last = Instant::now();
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("download interrupted: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if last.elapsed() > Duration::from_millis(150) {
            last = Instant::now();
            let _ = app.emit(
                "ai-progress",
                InstallProgress {
                    id: id.to_owned(),
                    done,
                    total: a.size,
                    phase: "download",
                },
            );
        }
    }
    drop(file);
    let got: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if got != a.sha256 {
        let _ = fs::remove_file(&part);
        return Err(format!(
            "checksum mismatch for {id}: the download was corrupted or tampered with"
        ));
    }
    fs::rename(&part, dest).map_err(|e| e.to_string())?;
    let _ = app.emit(
        "ai-progress",
        InstallProgress {
            id: id.to_owned(),
            done: a.size,
            total: a.size,
            phase: "verified",
        },
    );
    Ok(())
}

fn extract(archive: &Path, out: &Path) -> Result<(), String> {
    let _ = fs::remove_dir_all(out);
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let f = fs::File::open(archive).map_err(|e| e.to_string())?;
    if archive.extension().is_some_and(|e| e == "zip") {
        let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
        z.extract(out).map_err(|e| e.to_string())?;
    } else {
        let gz = flate2::read::GzDecoder::new(f);
        let mut t = tar::Archive::new(gz);
        t.unpack(out).map_err(|e| e.to_string())?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut stack = vec![out.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(&d).map_err(|e| e.to_string())?.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if let Ok(m) = fs::metadata(&p) {
                    let mut perm = m.permissions();
                    perm.set_mode(perm.mode() | 0o755);
                    let _ = fs::set_permissions(&p, perm);
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn ai_install(app: AppHandle, id: String) -> Result<AiStatus, String> {
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let dir = ai_dir(&app2)?;
        let models = dir.join("models");
        fs::create_dir_all(&models).map_err(|e| e.to_string())?;
        match id.as_str() {
            "runtime" => {
                let a = runtime_artifact().ok_or("no llama.cpp build for this platform")?;
                let ext = if a.url.ends_with(".zip") {
                    "zip"
                } else {
                    "tar.gz"
                };
                let archive = dir.join(format!("llama-{LLAMA_BUILD}.{ext}"));
                download(&app2, &id, &a, &archive)?;
                let _ = app2.emit(
                    "ai-progress",
                    InstallProgress {
                        id: id.clone(),
                        done: a.size,
                        total: a.size,
                        phase: "extract",
                    },
                );
                extract(&archive, &dir.join("runtime"))?;
                let _ = fs::remove_file(&archive);
                if server_exe(&dir).is_none() {
                    return Err("llama-server was not found in the runtime archive".to_owned());
                }
            }
            "decider" => download(&app2, &id, &DECIDER, &models.join("laya-q8_0.gguf"))?,
            "narrator" => download(
                &app2,
                &id,
                &NARRATOR,
                &models.join("qwen2.5-0.5b-instruct-q4_k_m.gguf"),
            )?,
            _ => return Err("unknown component".to_owned()),
        }
        status(&app2)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn ai_remove(
    app: AppHandle,
    state: State<'_, AiState>,
    id: String,
) -> Result<AiStatus, String> {
    state.shutdown();
    let dir = ai_dir(&app)?;
    match id.as_str() {
        "runtime" => {
            let _ = fs::remove_dir_all(dir.join("runtime"));
        }
        "decider" => {
            let _ = fs::remove_file(dir.join("models/laya-q8_0.gguf"));
        }
        "narrator" => {
            let _ = fs::remove_file(dir.join("models/qwen2.5-0.5b-instruct-q4_k_m.gguf"));
        }
        _ => return Err("unknown component".to_owned()),
    }
    status(&app)
}

fn healthy(port: u16) -> bool {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_millis(800)))
        .http_status_as_error(true)
        .build()
        .into();
    agent
        .get(format!("http://127.0.0.1:{port}/health"))
        .call()
        .is_ok()
}

fn ensure_server(app: &AppHandle, state: &AiState, which: &'static str) -> Result<u16, String> {
    let (port, model, extra): (u16, &str, &[&str]) = match which {
        "decider" => (
            DECIDER_PORT,
            "laya-q8_0.gguf",
            &[
                "--ubatch-size",
                "4096",
                "--batch-size",
                "4096",
                "-c",
                "4096",
            ],
        ),
        _ => (
            NARRATOR_PORT,
            "qwen2.5-0.5b-instruct-q4_k_m.gguf",
            &["-c", "8192"],
        ),
    };
    {
        let mut servers = state.servers.lock().map_err(|_| "lock poisoned")?;
        if let Some(c) = servers.get_mut(which) {
            if c.try_wait().map_err(|e| e.to_string())?.is_none() && healthy(port) {
                return Ok(port);
            }
            servers.remove(which);
        }
    }
    let dir = ai_dir(app)?;
    let exe = server_exe(&dir).ok_or("the local AI runtime is not installed")?;
    let model_path = dir.join("models").join(model);
    if !model_path.is_file() {
        return Err(format!("the {which} model is not installed"));
    }
    let mut cmd = Command::new(&exe);
    cmd.arg("-m")
        .arg(&model_path)
        .args([
            "--host",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--threads",
            "4",
            "--no-webui",
        ])
        .args(extra)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(parent) = exe.parent() {
        cmd.current_dir(parent);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let child = cmd
        .spawn()
        .map_err(|e| format!("could not start the local AI runtime: {e}"))?;
    state
        .servers
        .lock()
        .map_err(|_| "lock poisoned")?
        .insert(which, child);
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(90) {
        if healthy(port) {
            return Ok(port);
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    Err("the local AI runtime did not start within 90 seconds".to_owned())
}

#[tauri::command]
pub async fn ai_start_narrator(app: AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AiState>();
        ensure_server(&app, &state, "narrator").map(|p| format!("http://127.0.0.1:{p}"))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Ask Laya a typed question about a `state`. `questions` follows the System One API.
#[tauri::command]
pub async fn ai_decide(
    app: AppHandle,
    state_text: Value,
    questions: Value,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let st = app.state::<AiState>();
        let port = ensure_server(&app, &st, "decider")?;
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(120)))
            .http_status_as_error(true)
            .build()
            .into();
        let mut resp = agent
            .post(format!("http://127.0.0.1:{port}/v1/systemone"))
            .send_json(serde_json::json!({ "state": state_text, "questions": questions }))
            .map_err(|e| format!("decision model: {e}"))?;
        resp.body_mut()
            .read_json::<Value>()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
