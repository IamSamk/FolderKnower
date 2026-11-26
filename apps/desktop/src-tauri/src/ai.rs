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
//!
//! Users who already have these files (LM Studio, Ollama, the Hugging Face cache, a
//! llama.cpp build) can point the app at them instead of downloading. A linked file is
//! checked before it is used: a GGUF header, and the pinned SHA-256 when it should be
//! the exact model. Linked files are only ever read: removing one forgets the link and
//! never deletes the user's file.

use std::{
    collections::BTreeMap,
    fs,
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
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

const DECIDER_FILE: &str = "laya-q8_0.gguf";
const NARRATOR_FILE: &str = "qwen2.5-0.5b-instruct-q4_k_m.gguf";

/// A file of the user's that stands in for a download.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Link {
    path: String,
    /// "exact": same SHA-256 as the pinned download. "compatible": a different GGUF
    /// chat model, or a llama-server build that answered `--version`.
    check: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model_name: Option<String>,
}

fn read_links(dir: &Path) -> BTreeMap<String, Link> {
    fs::read_to_string(dir.join("links.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_links(dir: &Path, links: &BTreeMap<String, Link>) -> Result<(), String> {
    let s = serde_json::to_string_pretty(links).map_err(|e| e.to_string())?;
    fs::write(dir.join("links.json"), s).map_err(|e| e.to_string())
}

/// The linked file when it is still there.
fn live_link(dir: &Path, id: &str) -> Option<Link> {
    read_links(dir)
        .remove(id)
        .filter(|l| Path::new(&l.path).is_file())
}

fn runtime_path(dir: &Path) -> Option<PathBuf> {
    live_link(dir, "runtime")
        .map(|l| PathBuf::from(l.path))
        .or_else(|| server_exe(dir))
}

fn model_path(dir: &Path, id: &str) -> Option<PathBuf> {
    let file = if id == "decider" {
        DECIDER_FILE
    } else {
        NARRATOR_FILE
    };
    live_link(dir, id)
        .map(|l| PathBuf::from(l.path))
        .or_else(|| Some(dir.join("models").join(file)).filter(|p| p.is_file()))
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
    /// Set when the component is the user's own file rather than a download.
    #[serde(skip_serializing_if = "Option::is_none")]
    linked: Option<Link>,
    /// A file the user pointed to that is no longer there (moved, renamed, unplugged).
    #[serde(skip_serializing_if = "Option::is_none")]
    missing_path: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiStatus {
    components: Vec<Component>,
    decider_ready: bool,
    narrator_ready: bool,
    narrator_base_url: Option<String>,
    /// Where downloads are saved.
    install_dir: String,
}

fn status(app: &AppHandle) -> Result<AiStatus, String> {
    let dir = ai_dir(app)?;
    let rt = runtime_artifact();
    let runtime = runtime_path(&dir).is_some();
    let decider = model_path(&dir, "decider").is_some();
    let narrator = model_path(&dir, "narrator").is_some();
    let link = |id: &str| live_link(&dir, id);
    let missing = |id: &str| {
        read_links(&dir)
            .remove(id)
            .map(|l| l.path)
            .filter(|p| !Path::new(p).is_file())
    };
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
                linked: link("runtime"),
                missing_path: missing("runtime"),
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
                linked: link("decider"),
                missing_path: missing("decider"),
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
                linked: link("narrator"),
                missing_path: missing("narrator"),
            },
        ],
        decider_ready: runtime && decider,
        narrator_ready: runtime && narrator,
        narrator_base_url: (runtime && narrator)
            .then(|| format!("http://127.0.0.1:{NARRATOR_PORT}")),
        install_dir: dir.display().to_string(),
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
            "decider" => download(&app2, &id, &DECIDER, &models.join(DECIDER_FILE))?,
            "narrator" => download(&app2, &id, &NARRATOR, &models.join(NARRATOR_FILE))?,
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
    let mut links = read_links(&dir);
    if links.remove(&id).is_some() {
        // The user's own file: forget it, never delete it.
        write_links(&dir, &links)?;
        return status(&app);
    }
    match id.as_str() {
        "runtime" => {
            let _ = fs::remove_dir_all(dir.join("runtime"));
        }
        "decider" => {
            let _ = fs::remove_file(dir.join("models").join(DECIDER_FILE));
        }
        "narrator" => {
            let _ = fs::remove_file(dir.join("models").join(NARRATOR_FILE));
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
    let (port, extra): (u16, &[&str]) = match which {
        "decider" => (
            DECIDER_PORT,
            &[
                "--ubatch-size",
                "4096",
                "--batch-size",
                "4096",
                "-c",
                "4096",
            ],
        ),
        _ => (NARRATOR_PORT, &["-c", "8192"]),
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
    let exe = runtime_path(&dir).ok_or("the local AI runtime is not installed")?;
    let model_path =
        model_path(&dir, which).ok_or_else(|| format!("the {which} model is not installed"))?;
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

// ---- Using files the user already has -------------------------------------------

const GGUF_MAGIC: &[u8; 4] = b"GGUF";
/// Model families that only embed text: they cannot write an explanation.
const EMBED_ONLY: &[&str] = &["bert", "nomic-bert", "jina-bert-v2", "t5encoder", "clip"];

/// `general.architecture` and `general.name` from a GGUF header, reading only the
/// first metadata entries. Errors when the file is not GGUF.
fn gguf_meta(path: &Path) -> Result<(Option<String>, Option<String>), String> {
    let f = fs::File::open(path).map_err(|e| format!("cannot open the file: {e}"))?;
    let mut r = BufReader::new(f);
    let mut magic = [0u8; 4];
    r.read_exact(&mut magic)
        .map_err(|_| "the file is too short to be a model".to_owned())?;
    if &magic != GGUF_MAGIC {
        return Err("this is not a GGUF model file".to_owned());
    }
    let u32_ = |r: &mut BufReader<fs::File>| -> Result<u32, String> {
        let mut b = [0u8; 4];
        r.read_exact(&mut b).map_err(|e| e.to_string())?;
        Ok(u32::from_le_bytes(b))
    };
    let u64_ = |r: &mut BufReader<fs::File>| -> Result<u64, String> {
        let mut b = [0u8; 8];
        r.read_exact(&mut b).map_err(|e| e.to_string())?;
        Ok(u64::from_le_bytes(b))
    };
    let version = u32_(&mut r)?;
    if version < 2 {
        return Ok((None, None)); // GGUF v1 used 32-bit counts; too old to inspect
    }
    let _tensors = u64_(&mut r)?;
    let kvs = u64_(&mut r)?;
    fn read_str(r: &mut BufReader<fs::File>, len: u64) -> Result<String, String> {
        if len > 1 << 20 {
            return Err("damaged GGUF header".to_owned());
        }
        let mut b = vec![0u8; len as usize];
        r.read_exact(&mut b).map_err(|e| e.to_string())?;
        Ok(String::from_utf8_lossy(&b).into_owned())
    }
    fn skip(r: &mut BufReader<fs::File>, ty: u32, depth: u8) -> Result<(), String> {
        let fixed = match ty {
            0 | 1 | 7 => Some(1),
            2 | 3 => Some(2),
            4..=6 => Some(4),
            10..=12 => Some(8),
            _ => None,
        };
        let mut b8 = [0u8; 8];
        if let Some(n) = fixed {
            return r.seek_relative(n).map_err(|e| e.to_string());
        }
        match ty {
            8 => {
                r.read_exact(&mut b8).map_err(|e| e.to_string())?;
                r.seek_relative(u64::from_le_bytes(b8) as i64)
                    .map_err(|e| e.to_string())
            }
            9 if depth < 3 => {
                let mut b4 = [0u8; 4];
                r.read_exact(&mut b4).map_err(|e| e.to_string())?;
                let inner = u32::from_le_bytes(b4);
                r.read_exact(&mut b8).map_err(|e| e.to_string())?;
                let n = u64::from_le_bytes(b8);
                let size = match inner {
                    0 | 1 | 7 => 1,
                    2 | 3 => 2,
                    4..=6 => 4,
                    10..=12 => 8,
                    _ => 0,
                };
                if size > 0 {
                    return r
                        .seek_relative((n * size) as i64)
                        .map_err(|e| e.to_string());
                }
                for _ in 0..n {
                    skip(r, inner, depth + 1)?;
                }
                Ok(())
            }
            _ => Err("damaged GGUF header".to_owned()),
        }
    }
    let (mut arch, mut name) = (None, None);
    for _ in 0..kvs.min(64) {
        let klen = u64_(&mut r)?;
        let key = read_str(&mut r, klen)?;
        let ty = u32_(&mut r)?;
        if ty == 8 && (key == "general.architecture" || key == "general.name") {
            let vlen = u64_(&mut r)?;
            let v = read_str(&mut r, vlen)?;
            if key == "general.architecture" {
                arch = Some(v);
            } else {
                name = Some(v);
            }
            if arch.is_some() && name.is_some() {
                break;
            }
        } else {
            skip(&mut r, ty, 0)?;
        }
    }
    Ok((arch, name))
}

fn sha256_file(app: &AppHandle, id: &str, path: &Path, total: u64) -> Result<String, String> {
    let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    let (mut done, mut last) = (0u64, Instant::now());
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        done += n as u64;
        if last.elapsed() > Duration::from_millis(150) {
            last = Instant::now();
            let _ = app.emit(
                "ai-progress",
                InstallProgress {
                    id: id.to_owned(),
                    done,
                    total,
                    phase: "check",
                },
            );
        }
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// `llama-server` inside a folder the user picked, or the file itself.
fn find_server_in(p: &Path) -> Option<PathBuf> {
    if p.is_file() {
        return Some(p.to_path_buf());
    }
    let name = if cfg!(windows) {
        "llama-server.exe"
    } else {
        "llama-server"
    };
    let mut stack = vec![(p.to_path_buf(), 0u8)];
    while let Some((d, depth)) = stack.pop() {
        for e in fs::read_dir(&d).ok()?.flatten() {
            let q = e.path();
            if q.is_dir() && depth < 4 {
                stack.push((q, depth + 1));
            } else if q.file_name().is_some_and(|n| n == name) {
                return Some(q);
            }
        }
    }
    None
}

/// Run `exe --version` for up to 15 seconds and require llama.cpp's answer.
fn check_server(exe: &Path) -> Result<(), String> {
    let mut cmd = Command::new(exe);
    cmd.arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("could not run this file: {e}"))?;
    let t0 = Instant::now();
    loop {
        if child.try_wait().map_err(|e| e.to_string())?.is_some() {
            break;
        }
        if t0.elapsed() > Duration::from_secs(15) {
            let _ = child.kill();
            return Err("the program did not answer `--version`".to_owned());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let mut out = String::new();
    if let Some(mut o) = child.stdout.take() {
        let _ = o.read_to_string(&mut out);
    }
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut out);
    }
    if out.contains("version:") || out.contains("llama") {
        Ok(())
    } else {
        Err("this program is not llama.cpp's llama-server".to_owned())
    }
}

fn verify_link(app: &AppHandle, id: &str, path: &Path) -> Result<Link, String> {
    if id == "runtime" {
        let exe = find_server_in(path).ok_or("no llama-server program in that folder")?;
        check_server(&exe)?;
        return Ok(Link {
            path: exe.display().to_string(),
            check: "compatible".into(),
            model_name: None,
        });
    }
    let pinned = match id {
        "decider" => &DECIDER,
        "narrator" => &NARRATOR,
        _ => return Err("unknown component".to_owned()),
    };
    let size = fs::metadata(path).map_err(|e| e.to_string())?.len();
    let (arch, name) = gguf_meta(path)?;
    if size == pinned.size && sha256_file(app, id, path, size)? == pinned.sha256 {
        return Ok(Link {
            path: path.display().to_string(),
            check: "exact".into(),
            model_name: name,
        });
    }
    if id == "decider" {
        return Err(
            "Laya answers through its own decision interface, so it must be the exact Laya-Q8_0.gguf file. This one differs; install it instead.".to_owned(),
        );
    }
    if let Some(a) = arch.as_deref()
        && EMBED_ONLY.contains(&a)
    {
        return Err(format!(
            "this is an embedding model ({a}); it turns text into numbers and cannot write explanations"
        ));
    }
    if size > 24 * (1 << 30) {
        return Err("this model is larger than 24 GB and would not run well on a CPU".to_owned());
    }
    Ok(Link {
        path: path.display().to_string(),
        check: "compatible".into(),
        model_name: name.or(arch),
    })
}

/// Use a file the user already has instead of downloading it.
#[tauri::command]
pub async fn ai_link(app: AppHandle, id: String, path: String) -> Result<AiStatus, String> {
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let link = verify_link(&app2, &id, Path::new(&path))?;
        app2.state::<AiState>().shutdown();
        let dir = ai_dir(&app2)?;
        let mut links = read_links(&dir);
        links.insert(id, link);
        write_links(&dir, &links)?;
        status(&app2)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    id: &'static str,
    path: String,
    size: u64,
    /// Same size as the pinned file (or Ollama's blob name is its SHA-256): almost
    /// certainly the exact model. Linking still checks.
    likely_exact: bool,
    label: String,
}

/// Look for models and llama-server in the places other tools keep them.
#[tauri::command]
pub async fn ai_find(app: AppHandle, folder: Option<String>) -> Result<Vec<Found>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let home = app.path().home_dir().map_err(|e| e.to_string())?;
        let mut roots = vec![
            home.join(".cache/huggingface/hub"),
            home.join(".lmstudio/models"),
            home.join(".cache/lm-studio/models"),
            home.join(".ollama/models/blobs"),
            home.join(".cache/llama.cpp"),
            home.join("models"),
            home.join("Models"),
            home.join("Downloads"),
        ];
        // A folder the user chose is searched alone: they know where the files went.
        let chosen = folder.map(PathBuf::from);
        if let Some(d) = &chosen {
            roots = vec![d.clone()];
        }
        let mut out: Vec<Found> = Vec::new();
        let mut visited = 0usize;
        for root in roots {
            let mut stack = vec![(root, 0u8)];
            while let Some((d, depth)) = stack.pop() {
                let Ok(rd) = fs::read_dir(&d) else { continue };
                for e in rd.flatten() {
                    visited += 1;
                    if visited > 50_000 || out.len() >= 40 {
                        break;
                    }
                    let p = e.path();
                    let Ok(ft) = e.file_type() else { continue };
                    if ft.is_dir() {
                        if depth < 6 {
                            stack.push((p, depth + 1));
                        }
                        continue;
                    }
                    let name = e.file_name().to_string_lossy().into_owned();
                    let blob = name.strip_prefix("sha256-").map(str::to_owned);
                    let gguf = name.to_ascii_lowercase().ends_with(".gguf");
                    if !gguf && blob.is_none() {
                        continue;
                    }
                    // Hugging Face snapshots are links into blobs/: follow them for size.
                    let Ok(meta) = fs::metadata(&p) else { continue };
                    let size = meta.len();
                    if size < 50_000_000 {
                        continue;
                    }
                    // Ollama names blobs by hash: only GGUF ones are models, and the
                    // header carries a readable name.
                    let name = if blob.is_some() {
                        match gguf_meta(&p) {
                            Ok((_, Some(n))) => format!("{n} (Ollama)"),
                            Ok(_) => name.clone(),
                            Err(_) => continue,
                        }
                    } else {
                        name.clone()
                    };
                    for (id, a) in [("decider", &DECIDER), ("narrator", &NARRATOR)] {
                        let exact = blob.as_deref().is_some_and(|h| h == a.sha256)
                            || (size == a.size && blob.is_none());
                        if exact {
                            out.push(Found {
                                id,
                                path: p.display().to_string(),
                                size,
                                likely_exact: true,
                                label: name.clone(),
                            });
                        }
                    }
                    if gguf && size != DECIDER.size && size != NARRATOR.size {
                        out.push(Found {
                            id: "narrator",
                            path: p.display().to_string(),
                            size,
                            likely_exact: false,
                            label: name.clone(),
                        });
                    }
                }
            }
        }
        // llama-server on PATH or in the usual install folders.
        let exe = if cfg!(windows) {
            "llama-server.exe"
        } else {
            "llama-server"
        };
        let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).collect())
            .unwrap_or_default();
        dirs.extend([
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
            home.join(".local/bin"),
        ]);
        let on_path = match &chosen {
            Some(d) => find_server_in(d),
            None => dirs.iter().map(|d| d.join(exe)).find(|p| p.is_file()),
        };
        if let Some(d) = on_path {
            out.push(Found {
                id: "runtime",
                path: d.display().to_string(),
                size: fs::metadata(&d).map(|m| m.len()).unwrap_or(0),
                likely_exact: false,
                label: exe.to_owned(),
            });
        }
        out.sort_by(|a, b| (a.id, !a.likely_exact, &a.path).cmp(&(b.id, !b.likely_exact, &b.path)));
        out.dedup_by(|a, b| a.id == b.id && a.path == b.path);
        Ok(out)
    })
    .await
    .map_err(|e| e.to_string())?
}

// ---- What it takes to run this ---------------------------------------------------

const GB: u64 = 1 << 30;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    what: &'static str,
    minimum: String,
    recommended: &'static str,
    yours: String,
    /// "good", "ok" (meets the minimum) or "low".
    verdict: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Requirements {
    rows: Vec<Requirement>,
    suggestion: &'static str,
}

fn verdict(have: u64, minimum: u64, recommended: u64) -> &'static str {
    match have {
        h if h >= recommended => "good",
        h if h >= minimum => "ok",
        _ => "low",
    }
}

/// Advice by installed memory. The built-in runtime is CPU only.
fn suggest(ram_gb: u64, threads: u64) -> &'static str {
    // A machine sold as 8, 16 or 32 GB reports a little less.
    if ram_gb < 7 || threads < 4 {
        "Skip the local models on this computer. Every diagram, trace and health check works without them."
    } else if ram_gb < 15 {
        "Use the built-in Laya and Qwen2.5 0.5B models. Explanations are short; check them against the cited code."
    } else if ram_gb < 30 {
        "The built-in models run comfortably. For clearly better explanations, point the narrator at a 3B instruct model such as Qwen2.5-Coder 3B Q4_K_M (about 2 GB)."
    } else {
        "You have room for a 7B instruct model such as Qwen2.5-Coder 7B Q4_K_M (about 4.7 GB) as the narrator. It explains best, but answers slowly on the CPU."
    }
}

/// Minimum and recommended hardware next to what this computer has. Memory and disk
/// figures are estimates with room for the app and the system.
#[tauri::command]
pub fn ai_requirements(app: AppHandle) -> Result<Requirements, String> {
    let dir = ai_dir(&app)?;
    let mut sys = sysinfo::System::new().map_err(|e| e.to_string())?;
    sys.refresh_memory();
    // Marketed sizes: an "8 GB" machine reports a little under 8 GiB.
    let ram = (sys.total_memory() + GB / 2) / GB;
    let threads = std::thread::available_parallelism().map_or(0, |n| n.get() as u64);
    let disks = sysinfo::Disks::new_with_refreshed_list().ok();
    let free = disks
        .iter()
        .flat_map(|d| d.iter())
        .filter(|d| dir.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(|d| d.available_space());
    let download = runtime_artifact().map_or(0, |a| a.size) + DECIDER.size + NARRATOR.size;
    let mut rows = vec![
        Requirement {
            what: "Memory (RAM)",
            minimum: "8 GB".to_owned(),
            recommended: "16 GB",
            yours: format!("{ram} GB"),
            verdict: verdict(ram, 7, 15),
        },
        Requirement {
            what: "Processor threads",
            minimum: "4".to_owned(),
            recommended: "8 or more",
            yours: threads.to_string(),
            verdict: verdict(threads, 4, 8),
        },
    ];
    if let Some(free) = free {
        rows.push(Requirement {
            what: "Free disk for downloads",
            minimum: format!("2 GB (the downloads are {} MB)", download / 1_000_000),
            recommended: "5 GB",
            yours: format!("{} GB", free / GB),
            verdict: verdict(free / GB, 2, 5),
        });
    }
    Ok(Requirements {
        rows,
        suggestion: suggest(ram, threads),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gguf(kvs: &[(&str, u32, Vec<u8>)]) -> Vec<u8> {
        let mut b = b"GGUF".to_vec();
        b.extend(3u32.to_le_bytes());
        b.extend(0u64.to_le_bytes());
        b.extend((kvs.len() as u64).to_le_bytes());
        for (k, ty, v) in kvs {
            b.extend((k.len() as u64).to_le_bytes());
            b.extend(k.as_bytes());
            b.extend(ty.to_le_bytes());
            b.extend(v);
        }
        b
    }

    fn s(v: &str) -> Vec<u8> {
        let mut b = (v.len() as u64).to_le_bytes().to_vec();
        b.extend(v.as_bytes());
        b
    }

    #[test]
    fn advice_follows_the_hardware() {
        assert!(suggest(4, 8).starts_with("Skip"));
        assert!(suggest(8, 8).starts_with("Use the built-in"));
        assert!(suggest(16, 8).contains("3B"));
        assert!(suggest(31, 24).contains("7B"));
        assert!(suggest(64, 16).contains("7B"));
        assert_eq!(verdict(8, 8, 16), "ok");
    }

    #[test]
    fn reads_architecture_and_name_past_other_entries() {
        let dir = std::env::temp_dir().join(format!("ak-gguf-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("m.gguf");
        // u32 entry, then a string array, then the two strings we want.
        let mut arr = 8u32.to_le_bytes().to_vec();
        arr.extend(2u64.to_le_bytes());
        arr.extend(s("a"));
        arr.extend(s("bc"));
        fs::write(
            &p,
            gguf(&[
                ("general.alignment", 4, 32u32.to_le_bytes().to_vec()),
                ("tokenizer.ggml.tokens", 9, arr),
                ("general.architecture", 8, s("qwen2")),
                ("general.name", 8, s("Qwen2.5 0.5B Instruct")),
            ]),
        )
        .unwrap();
        let (arch, name) = gguf_meta(&p).unwrap();
        assert_eq!(arch.as_deref(), Some("qwen2"));
        assert_eq!(name.as_deref(), Some("Qwen2.5 0.5B Instruct"));
        fs::write(&p, b"PK\x03\x04 not a model").unwrap();
        assert!(gguf_meta(&p).unwrap_err().contains("not a GGUF"));
        let _ = fs::remove_dir_all(&dir);
    }
}
