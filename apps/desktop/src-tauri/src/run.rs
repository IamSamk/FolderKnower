//! Run your project: detect start commands, launch one only after the user confirms the
//! exact command and folder, tail its logs, report listening ports and docker container
//! stats. Everything here is measured on this computer; nothing is estimated.

use std::{
    collections::VecDeque,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};

use serde::Serialize;
use tauri::State;

const LOG_LINES: usize = 300;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RunOption {
    pub id: String,
    pub label: String,
    pub command: String,
    pub cwd: String,
    pub why: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PortRow {
    pub port: u16,
    pub pid: u32,
    pub name: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RunStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub command: String,
    pub cwd: String,
    pub exit_code: Option<i32>,
    pub log: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContainerRow {
    pub name: String,
    pub cpu: String,
    pub mem_usage: String,
    pub mem_pct: String,
    pub net_io: String,
    pub block_io: String,
}

struct Job {
    child: Child,
    command: String,
    cwd: String,
    log: Arc<Mutex<VecDeque<String>>>,
    exit: Option<i32>,
}

#[derive(Default)]
pub struct RunState(Mutex<Option<Job>>);

fn exists(root: &Path, f: &str) -> bool {
    root.join(f).is_file()
}

fn push(opts: &mut Vec<RunOption>, id: &str, label: &str, command: &str, cwd: &Path, why: &str) {
    opts.push(RunOption {
        id: id.into(),
        label: label.into(),
        command: command.into(),
        cwd: cwd.to_string_lossy().into_owned(),
        why: why.into(),
    });
}

fn detect_in(dir: &Path, opts: &mut Vec<RunOption>) {
    for f in [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ] {
        if exists(dir, f) {
            push(
                opts,
                "compose",
                "Docker Compose",
                "docker compose up",
                dir,
                &format!("{f} found"),
            );
            break;
        }
    }
    if let Ok(t) = std::fs::read_to_string(dir.join("package.json"))
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&t)
    {
        let pm = if exists(dir, "pnpm-lock.yaml") {
            "pnpm"
        } else if exists(dir, "yarn.lock") {
            "yarn"
        } else {
            "npm"
        };
        for s in ["dev", "start", "serve"] {
            if v["scripts"][s].is_string() {
                let cmd = if pm == "npm" {
                    format!("npm run {s}")
                } else {
                    format!("{pm} {s}")
                };
                let why = format!(
                    "package.json script \"{s}\": {}",
                    v["scripts"][s].as_str().unwrap_or("")
                );
                push(
                    opts,
                    &format!("npm-{s}"),
                    &format!("{pm} {s}"),
                    &cmd,
                    dir,
                    &why,
                );
            }
        }
    }
    if exists(dir, "Cargo.toml") && dir.join("src/main.rs").is_file() {
        push(
            opts,
            "cargo",
            "cargo run",
            "cargo run",
            dir,
            "Cargo.toml with src/main.rs",
        );
    }
    if exists(dir, "manage.py") {
        push(
            opts,
            "django",
            "Django dev server",
            "python manage.py runserver",
            dir,
            "manage.py found",
        );
    }
    for f in ["main.py", "app.py"] {
        if exists(dir, f) {
            let cmd = format!("python {f}");
            push(
                opts,
                &format!("py-{f}"),
                &cmd,
                &cmd,
                dir,
                &format!("{f} found"),
            );
            break;
        }
    }
}

/// Start commands we can see in the project. Nothing runs until the user confirms one.
#[tauri::command]
pub fn run_detect(root: String) -> Vec<RunOption> {
    let root = PathBuf::from(root);
    let mut opts = Vec::new();
    detect_in(&root, &mut opts);
    if let Ok(rd) = std::fs::read_dir(&root) {
        let mut subs: Vec<_> = rd.flatten().filter(|e| e.path().is_dir()).collect();
        subs.sort_by_key(|e| e.file_name());
        for e in subs.into_iter().take(40) {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.starts_with('.')
                || matches!(
                    n.as_str(),
                    "node_modules" | "target" | "dist" | "build" | "venv" | ".venv"
                )
            {
                continue;
            }
            let before = opts.len();
            detect_in(&e.path(), &mut opts);
            for o in &mut opts[before..] {
                o.id = format!("{n}/{}", o.id);
                o.label = format!("{n}: {}", o.label);
            }
        }
    }
    opts
}

fn pump<R: std::io::Read + Send + 'static>(r: R, log: Arc<Mutex<VecDeque<String>>>) {
    std::thread::spawn(move || {
        for line in BufReader::new(r).lines().map_while(Result::ok) {
            if let Ok(mut g) = log.lock() {
                if g.len() >= LOG_LINES {
                    g.pop_front();
                }
                g.push_back(line);
            }
        }
    });
}

fn status(j: &mut Job) -> RunStatus {
    if j.exit.is_none()
        && let Ok(Some(s)) = j.child.try_wait()
    {
        j.exit = Some(s.code().unwrap_or(-1));
    }
    RunStatus {
        running: j.exit.is_none(),
        pid: Some(j.child.id()),
        command: j.command.clone(),
        cwd: j.cwd.clone(),
        exit_code: j.exit,
        log: j
            .log
            .lock()
            .map(|g| g.iter().cloned().collect())
            .unwrap_or_default(),
    }
}

fn kill_tree(j: &mut Job) {
    let pid = j.child.id();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000)
            .output();
    }
    #[cfg(unix)]
    {
        // The job leads its own process group, so one signal reaches the whole tree.
        let _ = Command::new("kill")
            .args(["-TERM", &format!("-{pid}")])
            .output();
    }
    let _ = j.child.kill();
    let _ = j.child.wait();
}

/// Launch `command` in `cwd`. The UI only calls this after the user confirmed both.
#[tauri::command]
pub fn run_start(
    state: State<'_, RunState>,
    command: String,
    cwd: String,
) -> Result<RunStatus, String> {
    let mut g = state.0.lock().map_err(|_| "run lock poisoned".to_owned())?;
    if let Some(j) = g.as_mut() {
        let s = status(j);
        if s.running {
            return Err("A project is already running. Stop it first.".into());
        }
    }
    if !Path::new(&cwd).is_dir() {
        return Err(format!("Folder not found: {cwd}"));
    }
    #[cfg(windows)]
    let mut c = {
        use std::os::windows::process::CommandExt;
        let mut c = Command::new("cmd");
        c.args(["/C", &command]).creation_flags(0x0800_0000);
        c
    };
    #[cfg(unix)]
    let mut c = {
        use std::os::unix::process::CommandExt;
        let mut c = Command::new("sh");
        c.args(["-c", &command]).process_group(0);
        c
    };
    let mut child = c
        .current_dir(&cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start: {e}"))?;
    let log = Arc::new(Mutex::new(VecDeque::new()));
    if let Some(o) = child.stdout.take() {
        pump(o, log.clone());
    }
    if let Some(e) = child.stderr.take() {
        pump(e, log.clone());
    }
    let mut j = Job {
        child,
        command,
        cwd,
        log,
        exit: None,
    };
    let s = status(&mut j);
    *g = Some(j);
    Ok(s)
}

#[tauri::command]
pub fn run_status(state: State<'_, RunState>) -> Option<RunStatus> {
    state.0.lock().ok()?.as_mut().map(status)
}

#[tauri::command]
pub fn run_stop(state: State<'_, RunState>) -> Option<RunStatus> {
    let mut g = state.0.lock().ok()?;
    let j = g.as_mut()?;
    if j.exit.is_none() {
        kill_tree(j);
        j.exit = Some(-1);
    }
    Some(status(j))
}

/// Every TCP/UDP socket that is listening, with its owner, so a running app can be found by port.
#[tauri::command]
pub fn run_ports() -> Vec<PortRow> {
    let mut v: Vec<PortRow> = listeners::get_all()
        .map(|s| {
            s.into_iter()
                .map(|l| PortRow {
                    port: l.socket.port(),
                    pid: l.process.pid,
                    name: l.process.name,
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|p| (p.port, p.pid));
    v.dedup_by_key(|p| (p.port, p.pid));
    v
}

/// One `docker stats` sample. Empty when docker is missing or not running.
#[tauri::command]
pub async fn run_containers() -> Vec<ContainerRow> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut c = Command::new("docker");
        c.args(["stats", "--no-stream", "--format", "{{json .}}"]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x0800_0000);
        }
        let Ok(out) = c.output() else {
            return Vec::new();
        };
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .map(|v| {
                let s = |k: &str| v[k].as_str().unwrap_or("").to_owned();
                ContainerRow {
                    name: s("Name"),
                    cpu: s("CPUPerc"),
                    mem_usage: s("MemUsage"),
                    mem_pct: s("MemPerc"),
                    net_io: s("NetIO"),
                    block_io: s("BlockIO"),
                }
            })
            .collect()
    })
    .await
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_compose_and_npm_scripts() {
        let d = std::env::temp_dir().join(format!("ak-run-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("compose.yaml"), "services: {}").unwrap();
        std::fs::write(d.join("pnpm-lock.yaml"), "").unwrap();
        std::fs::write(
            d.join("package.json"),
            r#"{"scripts":{"dev":"vite","build":"x"}}"#,
        )
        .unwrap();
        let o = run_detect(d.to_string_lossy().into_owned());
        let cmds: Vec<_> = o.iter().map(|x| x.command.as_str()).collect();
        assert_eq!(cmds, ["docker compose up", "pnpm dev"]);
        std::fs::remove_dir_all(&d).ok();
    }
}
