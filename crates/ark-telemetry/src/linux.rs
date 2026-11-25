//! Linux-only memory detail. On other platforms these return `None` / empty, and the
//! UI falls back to RSS + virtual size from sysinfo.

use serde::Serialize;

/// Where a process's resident memory comes from. Bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryBreakdown {
    /// Resident set size: pages of this process currently in RAM.
    pub rss: u64,
    /// Proportional set size: shared pages divided by the number of sharers.
    pub pss: u64,
    /// Heap, stacks, anonymous mmaps (private, not backed by a file).
    pub anonymous: u64,
    /// File-backed pages in RAM: code, shared libraries, memory-mapped files (e.g. model weights).
    pub file_backed: u64,
    /// Shared memory (shm, tmpfs, GPU driver buffers).
    pub shmem: u64,
    /// Pages of this process pushed out to swap.
    pub swap: u64,
    /// Main thread stack size.
    pub stack: u64,
    /// Peak RSS (high-water mark).
    pub peak_rss: u64,
    /// Locked (never swapped) memory, e.g. mlock'ed model weights.
    pub locked: u64,
}

/// Parse `/proc/<pid>/smaps_rollup` plus `/proc/<pid>/status` text.
pub fn parse_breakdown(smaps_rollup: &str, status: &str) -> MemoryBreakdown {
    let kb = |text: &str, key: &str| -> u64 {
        text.lines()
            .find_map(|l| l.strip_prefix(key))
            .and_then(|r| r.trim().trim_end_matches("kB").trim().parse::<u64>().ok())
            .map(|v| v * 1024)
            .unwrap_or(0)
    };
    MemoryBreakdown {
        rss: kb(smaps_rollup, "Rss:"),
        pss: kb(smaps_rollup, "Pss:"),
        anonymous: kb(smaps_rollup, "Anonymous:"),
        file_backed: kb(status, "RssFile:"),
        shmem: kb(status, "RssShmem:"),
        swap: kb(smaps_rollup, "Swap:"),
        stack: kb(status, "VmStk:"),
        peak_rss: kb(status, "VmHWM:"),
        locked: kb(smaps_rollup, "Locked:"),
    }
}

#[cfg(target_os = "linux")]
pub fn memory_breakdown(pid: u32) -> Option<MemoryBreakdown> {
    let s = std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup")).ok()?;
    let st = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap_or_default();
    Some(parse_breakdown(&s, &st))
}

#[cfg(not(target_os = "linux"))]
pub fn memory_breakdown(_pid: u32) -> Option<MemoryBreakdown> {
    None
}

/// Thread names (`comm`) of a process, e.g. `tokio-runtime-w`, `python3`, `cuda-EvtHandlr`.
#[cfg(target_os = "linux")]
pub fn thread_names(pid: u32, limit: usize) -> Vec<String> {
    let Ok(dir) = std::fs::read_dir(format!("/proc/{pid}/task")) else {
        return Vec::new();
    };
    let mut v: Vec<String> = dir
        .flatten()
        .filter_map(|e| std::fs::read_to_string(e.path().join("comm")).ok())
        .map(|s| s.trim().to_owned())
        .collect();
    v.sort();
    v.truncate(limit);
    v
}

#[cfg(not(target_os = "linux"))]
pub fn thread_names(_pid: u32, _limit: usize) -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_rollup() {
        let rollup = "Rss:              2048 kB\nPss:              1024 kB\nAnonymous:         512 kB\nSwap:                4 kB\nLocked:              0 kB\n";
        let status = "VmHWM:\t    4096 kB\nVmStk:\t     132 kB\nRssFile:\t    1536 kB\nRssShmem:\t       0 kB\n";
        let b = super::parse_breakdown(rollup, status);
        assert_eq!(b.rss, 2048 * 1024);
        assert_eq!(b.file_backed, 1536 * 1024);
        assert_eq!(b.stack, 132 * 1024);
        assert_eq!(b.peak_rss, 4096 * 1024);
    }
}
