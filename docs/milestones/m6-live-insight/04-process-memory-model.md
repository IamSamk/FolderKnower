# 04. Process memory model for students

**Status: done** (Linux breakdown). `ark-telemetry::linux`.

The Live view never shows a bare number. Each kind of memory gets one sentence:

| Field | Source | Sentence on screen |
|---|---|---|
| Resident (RSS) | sysinfo | What sits in RAM now. |
| Virtual size | sysinfo | Every address reserved, most never touched, which is why it looks huge. |
| PSS | `smaps_rollup` Pss | Fair share: shared pages divided among the processes sharing them. |
| Anonymous | `smaps_rollup` Anonymous | Memory the program asked for itself: objects, arrays, tensors. |
| File-backed | Rss − Anonymous − Shmem | Mapped files: shared libraries, and model weights loaded with mmap. The OS can drop and re-read these. |
| Shared memory | `smaps_rollup` Shmem (Shared_*) | Pages several processes see at once. |
| Swap | `smaps_rollup` Swap | Pages pushed to disk; touching them costs a disk read. |
| Peak RSS | `status` VmHWM | Highest resident size so far. |
| Stack | `status` VmStk | Main thread stack: every call pushes a frame and pops it on return. |
| Locked | `status` VmLck | Pages pinned in RAM (mlock), common for model servers. |

## Why mapped files matter for LLMs

llama.cpp and Ollama mmap the GGUF weights, so a 3.3 GB model shows mostly as
*file-backed*, not heap. Students otherwise conclude "Python leaks 3 GB". The
breakdown bar makes the difference visible in one glance.

## Platform notes

- **Windows:** working set and private bytes (`PROCESS_MEMORY_COUNTERS_EX`). We will
  label them by their Windows names rather than pretend they are RSS/PSS.
- **macOS:** `proc_pid_rusage` (`ri_phys_footprint`) is the number Activity Monitor
  shows; we will use it as the main figure.

## Threads

`thread_names(pid, 64)` reads `/proc/<pid>/task/*/comm`. The view groups them by name
("llama-worker ×8", "ThreadPoolExecutor ×8"), which shows thread pools at a glance.
