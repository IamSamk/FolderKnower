//! `ark-telemetry`: live, **unprivileged** telemetry.
//!
//! - [`Sampler`]: system memory/swap/per-core CPU and per-process RSS, virtual size,
//!   CPU %, thread count, disk I/O, parent/children (sysinfo; same-user processes).
//! - [`linux`]: richer memory breakdown from `/proc/<pid>/smaps_rollup` and `status`
//!   (anonymous vs file-backed vs shared, PSS, swap, stack) and per-thread names.
//! - [`runtime`]: parsers for local model-server introspection endpoints (Ollama
//!   `/api/ps`, llama-server `/metrics` + `/slots`, vLLM `/metrics`), so the app can
//!   show which models are loaded, how much RAM/VRAM they hold and how full the KV
//!   cache is. Fetching is the caller's job (the desktop app, with a short timeout).
//!
//! Nothing here needs elevation. Deep tracing (eBPF, ETW kernel providers) stays in
//! the separate opt-in `ark-probe` helper; see `docs/security/telemetry-elevation.md`.

pub mod linux;
pub mod runtime;

use serde::Serialize;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSnapshot {
    pub total_memory: u64,
    pub used_memory: u64,
    pub available_memory: u64,
    pub total_swap: u64,
    pub used_swap: u64,
    /// Usage per logical core, 0-100.
    pub cpus: Vec<f32>,
    pub cpu_brand: String,
    pub physical_cores: Option<usize>,
    pub os: String,
    pub process_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessRow {
    pub pid: u32,
    pub parent: Option<u32>,
    pub name: String,
    pub cmd: String,
    /// Resident set size (Linux/macOS) or working set (Windows), bytes.
    pub memory: u64,
    pub virtual_memory: u64,
    /// Percent of one core (can exceed 100 on multi-core).
    pub cpu: f32,
    /// OS threads, where the platform reports them (Linux).
    pub threads: Option<u32>,
    pub disk_read: u64,
    pub disk_written: u64,
    pub run_time: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDetail {
    pub row: ProcessRow,
    pub children: Vec<ProcessRow>,
    /// RSS of the process plus all descendants, bytes.
    pub tree_memory: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breakdown: Option<linux::MemoryBreakdown>,
    pub thread_names: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Memory,
    Cpu,
}

/// Holds a `sysinfo::System` between refreshes: CPU % needs two samples.
pub struct Sampler {
    sys: System,
}

fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_owned()
    } else {
        let mut o: String = s.chars().take(n).collect();
        o.push('…');
        o
    }
}

impl Sampler {
    pub fn new() -> Option<Self> {
        let mut s = Sampler {
            sys: System::new().ok()?,
        };
        s.refresh();
        Some(s)
    }

    /// Refresh memory, CPU and all processes. Call every 1-2 s, never faster than
    /// `sysinfo::MINIMUM_CPU_UPDATE_INTERVAL`.
    pub fn refresh(&mut self) {
        self.sys.refresh_memory();
        self.sys.refresh_cpu_usage();
        self.sys.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_memory()
                .with_cpu()
                .with_disk_usage()
                .with_tasks()
                .with_cmd(UpdateKind::OnlyIfNotSet),
        );
    }

    pub fn system(&self) -> SystemSnapshot {
        SystemSnapshot {
            total_memory: self.sys.total_memory(),
            used_memory: self.sys.used_memory(),
            available_memory: self.sys.available_memory(),
            total_swap: self.sys.total_swap(),
            used_swap: self.sys.used_swap(),
            cpus: self.sys.cpus().iter().map(|c| c.usage()).collect(),
            cpu_brand: self
                .sys
                .cpus()
                .first()
                .map(|c| c.brand().trim().to_owned())
                .unwrap_or_default(),
            physical_cores: System::physical_core_count().ok(),
            os: System::long_os_version().unwrap_or_default(),
            process_count: self.sys.processes().len(),
        }
    }

    fn row(&self, p: &sysinfo::Process) -> ProcessRow {
        let d = p.disk_usage();
        let cmd = p
            .cmd()
            .iter()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        ProcessRow {
            pid: p.pid().as_u32(),
            parent: p.parent().map(|x| x.as_u32()),
            name: p.name().to_string_lossy().into_owned(),
            cmd: clip(&cmd, 240),
            memory: p.memory(),
            virtual_memory: p.virtual_memory(),
            cpu: p.cpu_usage(),
            threads: p.tasks().map(|t| t.len() as u32),
            disk_read: d.read_bytes,
            disk_written: d.written_bytes,
            run_time: p.run_time(),
            status: format!("{:?}", p.status()),
        }
    }

    /// Top `limit` processes. Threads (Linux tasks) are folded into their process.
    pub fn processes(&self, limit: usize, sort: SortBy) -> Vec<ProcessRow> {
        let mut rows: Vec<ProcessRow> = self
            .sys
            .processes()
            .values()
            .filter(|p| p.thread_kind().is_none())
            .map(|p| self.row(p))
            .collect();
        match sort {
            SortBy::Memory => rows.sort_by(|a, b| b.memory.cmp(&a.memory).then(a.pid.cmp(&b.pid))),
            SortBy::Cpu => rows.sort_by(|a, b| {
                b.cpu
                    .partial_cmp(&a.cpu)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.pid.cmp(&b.pid))
            }),
        }
        rows.truncate(limit);
        rows
    }

    pub fn process(&self, pid: u32) -> Option<ProcessDetail> {
        let p = self.sys.process(Pid::from_u32(pid))?;
        let row = self.row(p);
        let mut children: Vec<ProcessRow> = Vec::new();
        let mut tree_memory = row.memory;
        let mut frontier = vec![pid];
        while let Some(cur) = frontier.pop() {
            for q in self.sys.processes().values() {
                if q.thread_kind().is_none() && q.parent().map(|x| x.as_u32()) == Some(cur) {
                    let r = self.row(q);
                    tree_memory += r.memory;
                    frontier.push(r.pid);
                    if children.len() < 64 {
                        children.push(r);
                    }
                }
            }
        }
        children.sort_by_key(|r| std::cmp::Reverse(r.memory));
        Some(ProcessDetail {
            row,
            children,
            tree_memory,
            breakdown: linux::memory_breakdown(pid),
            thread_names: linux::thread_names(pid, 64),
        })
    }

    /// Processes whose name or command line contains `needle` (case-insensitive).
    pub fn find(&self, needle: &str, limit: usize) -> Vec<ProcessRow> {
        let n = needle.to_ascii_lowercase();
        let mut v: Vec<ProcessRow> = self
            .sys
            .processes()
            .values()
            .filter(|p| p.thread_kind().is_none())
            .map(|p| self.row(p))
            .filter(|r| {
                r.name.to_ascii_lowercase().contains(&n) || r.cmd.to_ascii_lowercase().contains(&n)
            })
            .collect();
        v.sort_by_key(|r| std::cmp::Reverse(r.memory));
        v.truncate(limit);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_this_process() {
        let mut s = Sampler::new().expect("sysinfo available");
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        s.refresh();
        let sys = s.system();
        assert!(sys.total_memory > 0);
        assert!(!sys.cpus.is_empty());
        let me = std::process::id();
        let d = s.process(me).expect("own process visible");
        assert!(d.row.memory > 0);
        assert!(d.tree_memory >= d.row.memory);
        let top = s.processes(5, SortBy::Memory);
        assert!(!top.is_empty() && top.len() <= 5);
        if cfg!(target_os = "linux") {
            let b = d.breakdown.expect("smaps_rollup on linux");
            assert!(b.rss > 0);
            assert!(d.row.threads.unwrap_or(0) >= 1);
        }
    }
}
