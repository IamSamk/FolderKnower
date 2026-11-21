# Least-Privilege Elevation Models for Kernel/Process Telemetry (Linux eBPF, Windows ETW, macOS libproc/Mach/ES) for a Tauri v2 / Rust Desktop App

Research date: 2026-11-21. About 22 tool calls. Note on confidence: several primary pages (docs.ebpf.io token page, OBI security page) came back as navigation only when fetched, so some Linux facts come from secondary but reputable sources (Grafana Beyla docs, Canonical LXD docs, kernel mailing list patches). Items marked **[background, unverified this session]** are from prior knowledge and should be checked before they are relied on.

## Linux: minimal capabilities and elevation pattern for aya eBPF (tracepoints/kprobes/perf), sysctls, BPF token, ptrace_scope, lockdown, BTF

### Takeaway
On kernel 5.8 or later, a tracing helper needs CAP_BPF + CAP_PERFMON, not CAP_SYS_ADMIN. Add CAP_SYS_RESOURCE only on kernels before 5.11 (memlock). Add CAP_SYS_PTRACE / CAP_DAC_READ_SEARCH / CAP_CHECKPOINT_RESTORE only if the helper must read other users' /proc entries. Debian/Ubuntu-style perf_event_paranoid=3 and older kernels (<5.8) force a CAP_SYS_ADMIN fallback. BPF token (6.9+) is designed for delegation into user-namespaced containers, not for desktop helpers. The best fit for a desktop app is a systemd system service with `AmbientCapabilities=`/`CapabilityBoundingSet=` limited to these caps and reached over a Unix socket checked with SO_PEERCRED. Avoid a setcap'd binary that the user can run directly.

### Cited Findings
- CAP_BPF is "required for every feature on kernels 5.8 and later". CAP_PERFMON is used for "loading eBPF probes in the kernel and pointer arithmetic", including `BPF_PROG_TYPE_KPROBE` programs. — [Grafana Beyla security docs](https://grafana.com/docs/beyla/latest/requirements-and-security/security.md)
- Kernels before 5.8 (e.g. RHEL 8 4.18) "don't have CAP_BPF or CAP_PERFMON, so CAP_SYS_ADMIN covers those operations". CAP_SYS_RESOURCE (locked-memory limit) is needed only on kernels below 5.11. — [Grafana Beyla security docs](https://grafana.com/docs/beyla/latest/requirements-and-security/security.md)
- Capabilities for reading other processes: CAP_CHECKPOINT_RESTORE to read `/proc/<pid>/map_files` of other processes, CAP_SYS_PTRACE for `/proc/pid/exe` and executable modules, CAP_DAC_READ_SEARCH to bypass file/dir read permission checks. CAP_NET_RAW / CAP_NET_ADMIN are needed only for socket filter / TC network programs. — [Grafana Beyla security docs](https://grafana.com/docs/beyla/latest/requirements-and-security/security.md)
- `kernel.perf_event_paranoid` "controls access to performance monitoring for processes that don't have CAP_PERFMON". Upstream default is 2. Debian-based distributions use 3, which "disallows access to perf_event_open() without CAP_SYS_ADMIN". The fix is to lower it to 2 or grant CAP_SYS_ADMIN instead of CAP_PERFMON. — [Grafana Beyla security docs](https://grafana.com/docs/beyla/latest/requirements-and-security/security.md)
- Also listed in the same capability set: [OpenTelemetry OBI security page](https://opentelemetry.io/docs/zero-code/obi/security/). OBI is the donated Beyla, and the fetched body was empty, so the content is unconfirmed.
- `kernel.unprivileged_bpf_disabled` values: 0 = unprivileged bpf() allowed. 1 = disabled, and it cannot be cleared without a reboot. 2 = disabled, but an admin can toggle it back. Kconfig `BPF_UNPRIV_DEFAULT_OFF` makes 2 the boot default. Writing the sysctl requires CAP_SYS_ADMIN, and apps can use CAP_BPF instead of full admin. — [Borkmann patch, bpf list, May 2021](https://lore.kernel.org/bpf/74ec548079189e4e4dffaeb42b8987bb3c852eee.1620765074.git.daniel@iogearbox.net); Ubuntu adopted default 2: [Ubuntu kernel-team list](https://lists.ubuntu.com/archives/kernel-team/2021-May/120897.html)
- BPF Token appeared in Linux v6.9. A bpffs mount with `delegate_cmds`, `delegate_maps`, `delegate_progs`, `delegate_attachs` defines what is delegated. The app calls `bpf(BPF_TOKEN_CREATE)` on that mount and passes the token fd to later bpf() calls, so the kernel checks the token instead of the caller's credentials. The caller must hold CAP_BPF in its current user namespace. Apps must be token-aware, and libbpf supports tokens. — [Canonical LXD docs: BPF token](https://documentation.ubuntu.com/lxd/en/latest/_sources/explanation/bpf.md.txt); also [Incus BPF token delegation](https://linuxcontainers.org/incus/docs/main/explanation/bpf-tokens/) and [docs.ebpf.io token concept](https://docs.ebpf.io/linux/concepts/token/). The docs.ebpf.io body was not retrievable.
- Original BPF token / bpffs delegation patch series: [lore v9 series](https://lore.kernel.org/all/202311040829.XrnpSV8z-lkp@intel.com/T/)
- Yama `ptrace_scope` values:
  - 0 = attach to any same-UID dumpable process.
  - 1 = descendants only, or a target opts in via `prctl(PR_SET_PTRACER)`.
  - 2 = CAP_SYS_PTRACE only.
  - 3 = no attach at all, irreversible.
  - Writing the sysctl requires CAP_SYS_PTRACE.
  
  — [kernel.org Yama doc](https://docs.kernel.org/_sources/admin-guide/LSM/Yama.rst.txt)

### Inferences
- Recommended Linux pattern: a systemd system unit (e.g. `architekton-telemetryd.service`) running as a dedicated non-root user, with `AmbientCapabilities=CAP_BPF CAP_PERFMON`, a matching `CapabilityBoundingSet=`, `NoNewPrivileges=yes`, `ProtectSystem=strict`, `ProtectHome=yes`, `PrivateTmp=yes`, `RestrictAddressFamilies=AF_UNIX`, `SystemCallFilter=`, and `LockPersonality=`. Install is a one-time privileged step through the distro package or pkexec. Expose a Unix socket (`RuntimeDirectory=`, mode 0660, group-restricted) and verify the client with SO_PEERCRED uid/pid plus pidfd. **[background, unverified this session: exact systemd directive names are standard, see systemd.exec(5)]**
- `setcap cap_bpf,cap_perfmon+ep` on a helper binary is simpler, but anyone who can execute the binary gets the capabilities. An attacker's process could then run it with chosen arguments. If used, restrict execution to a group (mode 0750, root:group) and make the helper refuse arbitrary program loading, only attaching its own embedded, pre-verified BPF objects. pkexec/polkit per-session elevation brings password prompts and pkexec's history of CVEs (PwnKit, CVE-2021-4034). **[background, unverified this session]**
- CAP_BPF + CAP_PERFMON lets the helper load tracing programs that can call `bpf_probe_read_kernel`, which in effect allows reading kernel memory. Treat the helper as root-equivalent for confidentiality, because the reduced capability set is still not low privilege. **[inference + background]**
- BPF token is a poor fit for desktop helpers. It is built around user-namespace delegation from a privileged parent (container managers like LXD/Incus). It also needs kernel 6.9+, which LTS-era distros (RHEL 9 / Ubuntu 22.04) lack. Keep it on the roadmap only.
- aya uses BTF/CO-RE for portable programs, which needs `CONFIG_DEBUG_INFO_BTF=y` (`/sys/kernel/btf/vmlinux` present). Most 2024–2026 mainstream distro kernels ship it. Tracepoints with stable formats, and raw tracepoints / `sched_process_exec`/`exit`, are more stable than kprobes on internal functions. **[background, unverified this session]**
- Kernel lockdown: in `confidentiality` mode (often enabled with Secure Boot on some distros, e.g. Fedora/RHEL or Ubuntu with certain configs), `bpf_probe_read_kernel` / `bpf_read` of kernel memory is blocked. Kprobe-based kernel memory reads may fail, while tracepoint argument access usually works. `integrity` mode mostly does not block BPF tracing. **[background, unverified this session; verify against kernel `security/lockdown` and distro docs]**
- Same-user /proc reads (stat, status, io, fd counts) work unprivileged, subject to `hidepid=` mounts. `/proc/pid/environ`, `mem`, `maps` and `io` of other UIDs are gated by `PTRACE_MODE_READ` checks. A fully unprivileged mode should therefore limit itself to the user's own processes. **[background, unverified this session; see proc(5)]**

### Gaps
- Could not retrieve docs.ebpf.io's per-program-type capability table (kprobe vs tracepoint vs perf_event vs raw_tp) or the aya book's privilege guidance. Exact per-attach-type checks (e.g. whether `perf_event_open` for a tracepoint needs CAP_PERFMON only, or also depends on perf_event_paranoid) are not confirmed from primary source.
- The status of BPF token LSM hooks in mainline by late 2026, and whether any distro enables unprivileged token use outside containers, was not found.
- No primary source was fetched for kernel lockdown's exact effect on BPF helpers.

## Windows: ETW privileges (Administrator vs Performance Log Users), NT Kernel Logger / SystemTraceProvider, unprivileged user-mode providers, elevated service + named pipe ACLs, UAC

### Takeaway
ETW access is governed by per-GUID security descriptors (WMIGUID/TRACELOG rights). By default, Performance Log Users can create real-time sessions, enable providers and consume real-time data, but cannot access the NT Kernel Logger (they lack TRACELOG_ACCESS_KERNEL_LOGGER). Kernel/system-provider events (process, thread, image load, context switch) therefore need Administrators/SYSTEM, or an explicit grant of the profile privilege plus EventAccessControl on the SystemTraceProvider GUID. The right shape is a LocalSystem or virtual-account Windows service that runs the ETW session and serves data over a named pipe with a strict DACL. Authenticate clients by token/SID, not PID.

### Cited Findings
- ETW providers and loggers are securable WmiGuid objects. Their descriptors live under `HKLM\System\CurrentControlSet\Control\WMI\Security`. — [Geoff Chappell, ETW security](https://geoffchappell.com/notes/windows/etw/security.htm)
- Default access by group:
  - Administrators, SYSTEM, LOCAL SERVICE and NETWORK SERVICE get full control.
  - **Performance Log Users** get query, notification, create real-time and on-disk loggers, enable providers (TRACELOG_GUID_ENABLE), log events, access real-time sessions (TRACELOG_ACCESS_REALTIME) and register GUIDs. They lack WMIGUID_SET, READ_DESCRIPTION, EXECUTE and **TRACELOG_ACCESS_KERNEL_LOGGER**.
  - Everyone/Users get only TRACELOG_REGISTER_GUIDS.
  - Some provider descriptors allow only Administrators/SYSTEM.
  
  — [Geoff Chappell, ETW security](https://geoffchappell.com/notes/windows/etw/security.htm)
- The NT Kernel Logger requires TRACELOG_ACCESS_KERNEL_LOGGER, and GENERIC_ALL does not grant it. — [Geoff Chappell, ETW security](https://geoffchappell.com/notes/windows/etw/security.htm)
- Granting a non-admin, non-TCB process a SystemTraceProvider profiling session takes two steps: grant the user the profile privilege, and add the user to both the logger session GUID and the SystemTraceProvider GUID via `EventAccessControl`. — [Microsoft Learn: Configuring and Starting a SystemTraceProvider Session](https://learn.microsoft.com/en-ca/Windows/win32/etw/configuring-and-starting-a-systemtraceprovider-session)
- From Windows 8, SystemTraceProvider can be multiplexed across up to 8 system logger sessions (`EVENT_TRACE_SYSTEM_LOGGER_MODE`, private logger name, a new GUID that is not SystemTraceControlGuid). Two slots are taken by the NT Kernel Logger and the Circular Kernel Context Logger. On builds ≥20348, separate System Providers can be enabled via EnableTraceEx2. — [Microsoft Learn: SystemTraceProvider session](https://learn.microsoft.com/en-ca/Windows/win32/etw/configuring-and-starting-a-systemtraceprovider-session)
- EventAccessControl / EventAccessQuery / EventAccessRemove manage ETW GUID permissions. — [Microsoft Learn: EventAccessControl](https://learn.microsoft.com/en-us/windows/win32/api/evntcons/nf-evntcons-eventaccesscontrol)
- Named-pipe client PID (`GetNamedPipeClientProcessId`) is spoofable:
  - The PID is that of the process that opened the pipe, not necessarily the one doing I/O.
  - The local SMB redirector path can set arbitrary PIDs, or a fixed 0xFEFF.
  - Handle passing plus PID recycling defeats post-read PID checks.
  - Forshaw's recommendation: do not use the client PID for security decisions.
  
  — [Project Zero, Forshaw 2019](https://projectzero.google/2019/09/windows-exploitation-tricks-spoofing.html); API: [GetNamedPipeClientProcessId](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getnamedpipeclientprocessid)
- `ImpersonateNamedPipeClient` lets a server act with the client's token. This is how a service can obtain the client's SID/token for authorization. — [Microsoft Learn: ImpersonateNamedPipeClient](https://learn.microsoft.com/en-us/windows/win32/api/namedpipeapi/nf-namedpipeapi-impersonatenamedpipeclient)
- Named pipe impersonation is a known privilege-escalation technique (a client-side risk when a privileged process connects to an attacker's pipe). — [Elastic detection rule](https://www.elastic.co/guide/en/security/8.19/privilege-escalation-via-named-pipe-impersonation.html)

### Inferences
- Unprivileged path (no admin): use per-user, non-kernel data sources. Examples are `NtQuerySystemInformation`/Toolhelp32/PDH counters for process lists, CPU and memory (same-user handles with `PROCESS_QUERY_LIMITED_INFORMATION`), plus user-mode manifest providers whose DACL allows the user. Adding the user to Performance Log Users (one admin action) unlocks real-time sessions for most user-mode providers, such as Microsoft-Windows-Kernel-Process. **[background, unverified this session: whether Microsoft-Windows-Kernel-Process (a manifest provider, distinct from the NT Kernel Logger) is enable-able by Performance Log Users by default is not confirmed]**
- Elevated pattern: a Windows service (ServiceType own-process, preferably `NT SERVICE\<name>` virtual account with SeSystemProfilePrivilege granted, else LocalSystem) owns the ETW session. Hardening steps:
  - Create the pipe with `FILE_FLAG_FIRST_PIPE_INSTANCE` (prevents squatting), `PIPE_REJECT_REMOTE_CLIENTS`, and a DACL granting only the interactive user's SID (or an app-specific group) plus SYSTEM.
  - Authenticate by impersonation or `GetNamedPipeClientSessionId` + token SID check, never PID alone.
  - Optionally verify the client image's Authenticode signature as defense in depth, without relying on it, because PID-to-image mapping is racy.
  - Install the service via the MSI/NSIS installer (elevated once). The Tauri app itself runs `asInvoker` and never requires elevation. Use a `requireAdministrator` manifest only on a separate small "install/enable telemetry service" stub.
  
  **[background, unverified this session for the flag names; standard Win32]**
- ETW sessions are a global, limited resource (64 sessions by default; 8 system-logger slots). The helper should use a uniquely named session, stop stale sessions on start, and handle `ERROR_ALREADY_EXISTS`. **[background, unverified this session for the 64 figure]**

### Gaps
- Could not confirm from Microsoft Learn which specific manifest providers (Kernel-Process, Kernel-File, Kernel-Network) have DACLs allowing Performance Log Users by default on Windows 11 24H2/25H2.
- Did not find 2025–2026 Microsoft guidance specific to "Secure ETW" / PPL-only providers (e.g. Microsoft-Windows-Threat-Intelligence requires a PPL antimalware signer). This is **[background, unverified]**, but such providers are out of reach for third-party desktop apps.

## macOS: libproc/proc_pidinfo, task_for_pid, Endpoint Security, SMAppService helpers, XPC code-signing checks, kperf/kdebug

### Takeaway
For same-user processes, libproc (`proc_listpids`, `proc_pidinfo`, `proc_pid_rusage`) gives CPU, memory and identity data unprivileged. Other users' and root processes return EPERM or limited data without root. `task_for_pid` is effectively reserved for debuggers (needs `com.apple.security.cs.debugger` and a target with `get-task-allow`, or root with SIP limits), and Apple DTS advises against building products on it. Endpoint Security needs a restricted, Apple-approved entitlement, root (launchd system domain), a Developer ID provisioning profile, and user-granted Full Disk Access. The modern privileged-helper path is `SMAppService.daemon` (macOS 13+) with an XPC Mach service protected by a code-signing requirement check (`xpc_connection_set_peer_code_signing_requirement` / `NSXPCConnection.setCodeSigningRequirement`).

### Cited Findings
- `task_for_pid` "effectively allows you complete control over the target process". Apple has restricted the policy so it is useful only for developer tools, and non-debugger products built on it face ongoing compatibility problems (Quinn, Apple DTS). — [Apple Dev Forums 734461](https://developer.apple.com/forums/thread/734461); entitlement doc: [com.apple.security.cs.debugger](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.cs.debugger.md)
- Endpoint Security client requirements:
  - **Entitlement:** `com.apple.developer.endpoint-security.client` is restricted and must be authorized by a provisioning profile. A team may get only a development entitlement.
  - **Root:** running without root gives `ES_NEW_CLIENT_RESULT_ERR_NOT_PRIVILEGED`, so run as a launchd system-domain daemon.
  - **Packaging:** daemon-based ES clients are fully supported, but they must be packaged as an app-like bundle with `embedded.provisionprofile` (a Developer ID profile), and the signing certificate must match the profile.
  - **TCC:** `ES_NEW_CLIENT_RESULT_ERR_NOT_PERMITTED` means TCC Full Disk Access is missing. FDA is user-granted in System Settings, or via MDM `SystemPolicyAllFiles`.
  
  — [Apple Dev Forums 791996](https://developer.apple.com/forums/thread/791996)
- FDA/TCC friction for ES system extensions continues on macOS 26. — [Apple Dev Forums 844239](https://developer.apple.com/forums/thread/844239)
- SMAppService:
  - Introduced in macOS 13 Ventura to replace SMJobBless.
  - Daemon plists and binaries stay inside the app bundle (`Contents/Library/LaunchDaemons/`).
  - After `register()`, the user must approve the item in System Settings > Login Items, followed by an admin authentication. Status values include `requiresApproval`.
  - Security caveats: re-registering a previously approved daemon did not re-prompt, and on macOS 13.6 a user-"disallowed" job kept running (FB13206906).
  
  — [theevilbit, SMAppService analysis](https://theevilbit.github.io/posts/smappservice); Apple migration guide: [Updating helper executables from earlier versions of macOS](https://developer.apple.com/documentation/servicemanagement/updating-helper-executables-from-earlier-versions-of-macos.md); [SMAppService docs](https://developer.apple.com/documentation/servicemanagement/smappservice.md)
- `xpc_connection_set_peer_code_signing_requirement` checks the peer cryptographically as a code-signing operation, not by string matching. An example requirement is `anchor apple generic and certificate leaf[subject.OU] = "TEAMID"`, optionally tightened further (e.g. by identifier, or by rejecting development-signed clients). Quinn warns that doing the check yourself (e.g. `SecCodeCheckValidity` on a PID) "exposes you to TOC/TOU problems", so the XPC API should be used. — [Apple Dev Forums 773573](https://developer.apple.com/forums/thread/773573); requirement language: [TN3127 Inside Code Signing: Requirements](https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements.md)
- A newer API, `xpc_connection_set_peer_platform_identity_requirement`, exists for platform-identity peer checks. — [Apple docs](https://developer.apple.com/documentation/xpc/xpc_connection_set_peer_platform_identity_requirement(_:_:).md)
- macOS 26 discussions cover launch constraints, which restrict who can launch or spawn a helper. These can be used to ensure the helper only runs from the signed app bundle. — [Apple Dev Forums 799933](https://developer.apple.com/forums/thread/799933)
- Forum reports describe pitfalls with SMAppService sample code and with migrating from SMJobBless (helper not launching after migration). — [Forums 799910](https://developer.apple.com/forums/thread/799910); [Forums 843622](https://developer.apple.com/forums/thread/843622)

### Inferences
- Unprivileged macOS tier: `proc_listallpids` + `proc_pidinfo(PROC_PIDTASKINFO)` + `proc_pid_rusage` for the user's own processes. `host_statistics64` gives system-wide CPU/memory, and `sysctl kern.proc` gives the process list (other users' processes are visible in the list but with limited detail). `KERN_PROCARGS2` (argv/env) is restricted for other users' processes. This tier needs no entitlement and works under hardened runtime and notarization. **[background, unverified this session]**
- Elevated macOS tier: an `SMAppService.daemon` helper running as root, exposing a `MachServices` XPC endpoint. It sets a peer code-signing requirement that pins Team ID and the main app's bundle identifier. Apply launch constraints on the helper (macOS 13+) so it cannot be launched outside the app. Use Endpoint Security only if exec/fork/exit events are needed, and accept the cost: Apple's entitlement approval, FDA prompt, and a provisioning profile. Avoid task_for_pid entirely.
- kperf/kdebug (`kdebug_trace`, `ktrace` via `/usr/bin/ktrace` or private `ktrace.framework`) are private/SPI and need root. They are unsuitable for a notarized third-party product. **[background, unverified this session; no 2025–2026 primary source found]**
- On macOS, the XPC audit token (`xpc_connection_get_audit_token`, also SPI-ish) is the correct per-message identity and the PID is not. The code-signing requirement API wraps this. **[background]**

### Gaps
- No current Apple doc fetched confirming the exact restrictions on `proc_pidinfo` for root-owned processes on macOS 15/26 (whether some flavors return EPERM vs limited data).
- No 2025–2026 primary source on kperf/kdebug availability to third parties.
- Did not confirm whether an ES client can be packaged as an SMAppService daemon (the forum thread uses a manual launchd plist).

## Cross-cutting: IPC authentication, PID allowlisting, data minimization, rate limiting, signed helper, Tauri v2 capability isolation, threat model

### Takeaway
The helper is the main local-privilege-escalation surface. It must authenticate peers by kernel-provided credentials (SO_PEERCRED/pidfd on Linux, token/SID via impersonation on Windows, XPC code-signing requirement/audit token on macOS), never PID alone. It should expose a narrow, fixed command set (no "load arbitrary BPF", no arbitrary paths/PIDs beyond an allowlist), minimize data (no argv/env by default), and rate-limit. Inside the app, Tauri v2 capabilities should restrict which windows can call telemetry commands. They do not protect against Rust-side bugs.

### Cited Findings
- Tauri v2 capabilities scope permissions per window/webview by **label**. A window in multiple capabilities gets the merged permissions. A `platforms` array targets specific operating systems. `remote.urls` can expose commands to remote origins, but on Linux/Android Tauri cannot distinguish iframes from the window. Commands registered with `invoke_handler` are allowed for all windows by default unless restricted via `AppManifest::commands` in `build.rs`. — [Tauri v2: Capabilities](https://v2.tauri.app/security/capabilities/); reference: [Capability ACL reference](https://v2.tauri.app/reference/acl/capability/)
- What capabilities cover: they limit the impact of a frontend compromise, but they do **not** protect against malicious/insecure Rust code, overly lax scopes, incorrect scope checks in commands, WebView 0-days, or supply-chain attacks. Window creation should be exposed only to higher-privileged windows. — [Tauri v2: Capabilities](https://v2.tauri.app/security/capabilities/)
- PID-based peer authentication is spoofable on Windows named pipes. — [Project Zero](https://projectzero.google/2019/09/windows-exploitation-tricks-spoofing.html)
- On macOS, roll-your-own signature checks on a PID are subject to TOC/TOU, so the XPC peer requirement APIs should be used. — [Apple Dev Forums 773573](https://developer.apple.com/forums/thread/773573)
- A setcap'd or Linux helper with CAP_BPF/CAP_PERFMON can load kprobe programs. The Beyla docs treat CAP_SYS_ADMIN as a superset fallback. — [Grafana Beyla](https://grafana.com/docs/beyla/latest/requirements-and-security/security.md)

### Inferences
- **Linux IPC:**
  - Use a Unix domain socket in `/run/<app>/` with 0660 permissions and a dedicated group.
  - On accept, read `SO_PEERCRED` (uid/gid/pid, captured at connect() time and therefore race-safe for uid). Optionally use `SO_PEERPIDFD` (kernel 6.5+) to get a pidfd that is immune to PID reuse.
  - Enforce `peer_uid == session user`, or membership in the allowed group.
  - Do not trust `/proc/<pid>/exe` checks for authorization (racy).
  
  **[background, unverified this session: SO_PEERPIDFD version]**
- **Windows IPC:** use the named-pipe DACL (user SID + SYSTEM), `PIPE_REJECT_REMOTE_CLIENTS`, `FILE_FLAG_FIRST_PIPE_INSTANCE`, and a token SID/session check via impersonation (`SecurityIdentification` level is enough). Clients should open the pipe with `SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION` so a squatting server cannot fully impersonate them. **[background]**
- **macOS IPC:** use an XPC Mach service with `setCodeSigningRequirement` pinning Team ID + bundle ID, plus hardened runtime and library validation on both sides. The latter prevents dylib injection into the app, which would otherwise inherit its signature. **[background]**
- **Target scoping:** by default, report only processes owned by the requesting uid/SID. System-wide or other-user views need an explicit opt-in toggle stored in a root-owned config, not chosen by the client. Treat PIDs in requests as hints and re-validate ownership in the helper on each event, so PID reuse cannot be used to redirect monitoring.
- **Data minimization:** by default, emit pid, ppid, comm/exe basename, start time, and CPU/memory/IO counters. Do not capture argv, env, cwd, full paths of other users' files, or network payloads. Make argv capture opt-in, redact it (truncate, strip tokens matching secret patterns), and limit it to the user's own processes. Keep events in memory with a bounded ring buffer and do not persist them by default.
- **Rate limiting / DoS:** use bounded BPF ring buffers (`BPF_MAP_TYPE_RINGBUF`) with drop counters, per-client subscription limits, a max event rate per client, backpressure (drop rather than block the kernel path), and an idle timeout. The helper should also cap memory and CPU via `MemoryMax=`/`CPUQuota=` (systemd) or Job Objects (Windows).
- **Signed helper and update integrity:**
  - The helper binary and its BPF objects must be root/admin-owned and not user-writable. Embed the BPF bytecode in the signed binary rather than loading it from user-writable paths.
  - On macOS, SMAppService keeps the helper inside the app bundle, so bundle tampering matters. Rely on code signing, launch constraints, and SIP-protected `/Applications` (an inference from the theevilbit analysis).
  - On Windows, the service binary should live under `Program Files` with default ACLs, and the service path must be quoted (to avoid the unquoted-service-path LPE).
- **Threat model:** the main threats are:
  - A malicious same-user process (or a compromised webview) asking the helper for data it could not otherwise access, such as other users' argv/env or kernel memory via BPF.
  - Command injection or parameter abuse in the helper (arbitrary file paths, PIDs, BPF programs).
  - Pipe or socket squatting before the helper starts.
  - TOCTOU / PID reuse.
  - Persistence via a disabled-but-running daemon (macOS FB13206906).
  - Supply chain, via a compromised update replacing the helper.
  
  Mitigations: a fixed small command surface, kernel-credential peer auth, root-owned config for scope, a sandboxed service (systemd hardening / AppContainer-less service with restricted token / launchd), and a default-off telemetry tier with explicit per-OS consent UI.

### Gaps
- Could not find 2025–2026 Tauri-specific guidance on sidecar/helper privilege separation (the Tauri shell/sidecar plugin docs were not fetched).
- No published security audit of an eBPF-based desktop telemetry helper was found to cite as a precedent.
