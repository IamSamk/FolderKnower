// Run-your-project bridge. Only the desktop app can start or attach to a process; in the
// browser demo these calls return nothing and the panel says so.

import { checkIsTauri } from "./backend";

export interface RunOption {
  id: string;
  label: string;
  command: string;
  cwd: string;
  why: string;
}
export interface PortRow {
  port: number;
  pid: number;
  name: string;
}
export interface RunStatus {
  running: boolean;
  pid: number | null;
  command: string;
  cwd: string;
  exitCode: number | null;
  log: string[];
}
export interface ContainerRow {
  name: string;
  cpu: string;
  memUsage: string;
  memPct: string;
  netIo: string;
  blockIo: string;
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export const canRun = () => checkIsTauri();
export const runDetect = (root: string) =>
  canRun() ? call<RunOption[]>("run_detect", { root }) : Promise.resolve([]);
export const runStart = (command: string, cwd: string) =>
  call<RunStatus>("run_start", { command, cwd });
export const runStatus = () =>
  canRun() ? call<RunStatus | null>("run_status") : Promise.resolve(null);
export const runStop = () => call<RunStatus | null>("run_stop");
export const runPorts = () => (canRun() ? call<PortRow[]>("run_ports") : Promise.resolve([]));
export const runContainers = () =>
  canRun() ? call<ContainerRow[]>("run_containers") : Promise.resolve([]);

/** "123.4MiB / 7.7GiB" -> bytes of the first number. */
export function parseDockerBytes(s: string): number {
  const m = /([\d.]+)\s*([KMGT]?i?B)/i.exec(s);
  if (!m) return 0;
  const u = m[2].toLowerCase();
  const k = u.startsWith("k")
    ? 1
    : u.startsWith("m")
      ? 2
      : u.startsWith("g")
        ? 3
        : u.startsWith("t")
          ? 4
          : 0;
  const base = u.includes("i") ? 1024 : 1000;
  return Number(m[1]) * base ** k;
}
