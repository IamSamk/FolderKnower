# Telemetry least-privilege elevation model

Status: design, written ahead of Phase 4. Sources are in [`../research/notes/telemetry_security.md`](../research/notes/telemetry_security.md).

## Trust boundaries

```
┌────────────── user session (unprivileged) ──────────────┐      ┌──── privileged (opt-in, installed once) ────┐
│ WebView (React)  ──Tauri IPC (capabilities)──►  Rust app │ ──►  │ ark-probe  (signed; no network; no FS writes) │
│  untrusted-ish UI     validated commands         sysinfo │ IPC  │  Linux: systemd svc, CAP_BPF+CAP_PERFMON      │
│                                                  libproc │ auth │  Windows: service, ETW kernel/user sessions   │
└──────────────────────────────────────────────────────────┘      │  macOS: none in v1 (deferred)                 │
         ▲ reads untrusted repo files (never executes them)        └───────────────────────────────────────────────┘
```

## Default: baseline mode (no elevation)
- `sysinfo` plus OS APIs, limited to processes owned by the current user: CPU, RSS, threads and sockets.
- This mode works on every OS. Deep mode is a separate, explicit opt-in screen that explains what will be installed.

## Deep mode per OS

| | Linux | Windows | macOS |
|---|---|---|---|
| Install | Package postinst or a one-time `pkexec` to install a systemd unit | MSI installs a service | Deferred (ES entitlement is Apple-restricted; `task_for_pid` is debugger-only) |
| Privilege | `AmbientCapabilities=CAP_BPF CAP_PERFMON`, `CapabilityBoundingSet` the same, `NoNewPrivileges=yes`, `ProtectSystem=strict`, `ProtectHome=read-only`, `PrivateTmp=yes`, `RestrictAddressFamilies=AF_UNIX`, `DynamicUser` + group `architekton` | LocalSystem or a virtual service account with only the ETW privileges it needs; app manifest `asInvoker` | n/a |
| IPC authn | Unix socket `/run/architekton/probe.sock`, mode 0660, group `architekton`; `SO_PEERCRED` uid | Named pipe with a DACL for Interactive Users; identify the client by impersonating it and reading its token **SID**, not by PID | (future) XPC with `xpc_connection_set_peer_code_signing_requirement` (Team ID + bundle id) |
| Authz | target PID's uid == peer uid | target process token user == client SID | same-user |

## Probe API (complete list)
`Attach{pid, probes: subset of [sched, syscalls, tcp, cpu_samples], hz ≤ 99}` → session id. `Poll{session, max ≤ 4096}` → events. `Detach{session}`. `Caps{}`.

There is nothing else. The API has no "load program", no path arguments and no shell.

## Hardening rules
1. Data minimization: no argv, env, file contents or payloads. Only syscall numbers, socket 4-tuples, thread ids and stack addresses (symbolized in the app, not the helper).
2. Rate limits and bounded per-session ring buffers. Drop and count on overflow; never block the kernel side.
3. At most 4 concurrent sessions. Sessions are torn down when the peer disconnects.
4. BPF bytecode is compiled into the signed helper. Verify the license section (GPL-compatible where GPL-only helpers are used).
5. Respect host policy (`unprivileged_bpf_disabled`, `perf_event_paranoid`, lockdown). Report the setting and never change it.
6. Fuzz the IPC decoder (cargo-fuzz) and keep the helper small (target under 3k LOC excluding eBPF).
7. The Linux helper is root-equivalent for confidentiality (BPF can read kernel memory). Treat any helper bug as a local privilege-escalation or information-disclosure risk.

## Tauri app hardening
- A capability per window. `fs` is read-only and scoped to the chosen repo root plus `$APPDATA/architekton`. `shell` may run only the declared sidecars with argument validators. No `http` plugin.
- CSP: `default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'` (rough SVG inline styles), and no remote origins.
- Every command canonicalizes paths and rejects anything outside the repo root.
- The repository is untrusted input: no code execution, no build scripts, no symlink escapes, and size and time caps per file.

## Threat model (top items)

| Threat | Mitigation |
|---|---|
| Another local user drives the helper to spy on your processes | uid/SID authz; the target must be owned by the caller |
| A malicious repo exploits a parser | Tree-sitter runs in-process without elevation; per-file caps; fuzz grammars on fixtures; never in the helper |
| XSS in the WebView invokes dangerous commands | Minimal capabilities, CSP, no generic fs/shell, commands validate inputs |
| Helper turned into a generic BPF loader | Fixed embedded programs; no load API |
| PID reuse race | Re-check the uid of the PID at attach and on each poll; abort on mismatch |
| Supply chain | cargo-deny advisories and licenses, pinned versions, pnpm 12 build-script allowlist and min-release-age |
