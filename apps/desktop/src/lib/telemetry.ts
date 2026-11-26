// Live telemetry bridge. In the desktop app this calls the unprivileged Rust sampler;
// in a plain browser (demo / web) it simulates a believable machine so the Live view
// can be explored anywhere. Simulated data is always labelled as such in the UI.

import { checkIsTauri } from "./backend";

export interface SystemSnapshot {
  totalMemory: number;
  usedMemory: number;
  availableMemory: number;
  totalSwap: number;
  usedSwap: number;
  cpus: number[];
  cpuBrand: string;
  physicalCores: number | null;
  os: string;
  processCount: number;
}

export interface ProcessRow {
  pid: number;
  parent: number | null;
  name: string;
  cmd: string;
  memory: number;
  virtualMemory: number;
  cpu: number;
  threads: number | null;
  diskRead: number;
  diskWritten: number;
  runTime: number;
  status: string;
}

export interface MemoryBreakdown {
  rss: number;
  pss: number;
  anonymous: number;
  fileBacked: number;
  shmem: number;
  swap: number;
  stack: number;
  peakRss: number;
  locked: number;
}

export interface ProcessDetail {
  row: ProcessRow;
  children: ProcessRow[];
  treeMemory: number;
  breakdown?: MemoryBreakdown;
  threadNames: string[];
}

export interface LoadedModel {
  name: string;
  sizeBytes: number;
  vramBytes: number;
  contextLength: number | null;
  quantization: string | null;
  parameterSize: string | null;
  expiresAt: string | null;
}

export interface RuntimeStatus {
  server: string;
  endpoint: string;
  models: LoadedModel[];
  kvCacheUsage: number | null;
  prefixCacheHitRate: number | null;
  requestsRunning: number | null;
  requestsWaiting: number | null;
  promptTokensTotal: number | null;
  generatedTokensTotal: number | null;
  slotContexts: number[];
  busySlots: number | null;
  notes: string[];
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

export const simulated = (): boolean => !checkIsTauri();

// ---------------------------------------------------------------- simulator

const GB = 1024 ** 3;
const MB = 1024 ** 2;
let tick = 0;
const wave = (base: number, amp: number, period: number, phase = 0) =>
  base + amp * Math.sin((tick + phase) / period);

const SIM: Omit<ProcessRow, "memory" | "cpu" | "diskRead" | "diskWritten">[] = [
  {
    pid: 4120,
    parent: 1,
    name: "python3",
    cmd: "python -m app.agent --book book.pdf",
    virtualMemory: 9.1 * GB,
    threads: 23,
    runTime: 812,
    status: "Run",
  },
  {
    pid: 4133,
    parent: 4120,
    name: "ollama_llama_server",
    cmd: "ollama runner --model qwen3:4b --ctx-size 8192",
    virtualMemory: 12.4 * GB,
    threads: 18,
    runTime: 790,
    status: "Run",
  },
  {
    pid: 2210,
    parent: 1,
    name: "chrome",
    cmd: "chrome --type=renderer",
    virtualMemory: 6.2 * GB,
    threads: 31,
    runTime: 5400,
    status: "Sleep",
  },
  {
    pid: 3301,
    parent: 1,
    name: "code",
    cmd: "code .",
    virtualMemory: 4.1 * GB,
    threads: 40,
    runTime: 7200,
    status: "Sleep",
  },
  {
    pid: 5002,
    parent: 1,
    name: "fknow",
    cmd: "fknow",
    virtualMemory: 2.2 * GB,
    threads: 14,
    runTime: 60,
    status: "Run",
  },
  {
    pid: 801,
    parent: 1,
    name: "postgres",
    cmd: "postgres -D /var/lib/postgresql",
    virtualMemory: 0.9 * GB,
    threads: 1,
    runTime: 86000,
    status: "Sleep",
  },
];

function simRows(): ProcessRow[] {
  const mem = [
    wave(1.6 * GB, 0.35 * GB, 6),
    wave(3.3 * GB, 0.05 * GB, 20),
    1.1 * GB,
    0.9 * GB,
    wave(240 * MB, 20 * MB, 9),
    180 * MB,
  ];
  const cpu = [
    wave(55, 40, 3),
    wave(160, 140, 2.2, 1),
    wave(8, 6, 5),
    wave(4, 3, 7),
    wave(12, 8, 4),
    0.4,
  ];
  return SIM.map((p, i) => ({
    ...p,
    memory: Math.max(0, mem[i]),
    cpu: Math.max(0, cpu[i]),
    diskRead: i === 0 ? Math.max(0, wave(4 * MB, 4 * MB, 3)) : 0,
    diskWritten: 0,
  }));
}

// ---------------------------------------------------------------- API

export async function systemSnapshot(): Promise<SystemSnapshot> {
  if (!checkIsTauri()) {
    tick++;
    const used = simRows().reduce((a, r) => a + r.memory, 0) + 3.2 * GB;
    return {
      totalMemory: 16 * GB,
      usedMemory: used,
      availableMemory: 16 * GB - used,
      totalSwap: 4 * GB,
      usedSwap: 0.3 * GB,
      cpus: Array.from({ length: 8 }, (_, i) =>
        Math.max(1, Math.min(100, wave(35, 30, 2 + i * 0.4, i * 3))),
      ),
      cpuBrand: "Simulated 8-core CPU",
      physicalCores: 4,
      os: "Demo machine (simulated)",
      processCount: 312,
    };
  }
  return invoke<SystemSnapshot>("telemetry_system");
}

export async function topProcesses(sort: "memory" | "cpu", limit = 40): Promise<ProcessRow[]> {
  if (!checkIsTauri()) {
    const r = simRows();
    return r.sort((a, b) => (sort === "cpu" ? b.cpu - a.cpu : b.memory - a.memory)).slice(0, limit);
  }
  return invoke<ProcessRow[]>("telemetry_processes", { limit, sort });
}

export async function processDetail(pid: number): Promise<ProcessDetail | null> {
  if (!checkIsTauri()) {
    const rows = simRows();
    const row = rows.find((r) => r.pid === pid);
    if (!row) return null;
    const children = rows.filter((r) => r.parent === pid);
    const isPy = row.name === "python3";
    const isLlm = row.name.includes("llama") || row.name.includes("ollama");
    return {
      row,
      children,
      treeMemory: row.memory + children.reduce((a, c) => a + c.memory, 0),
      breakdown: {
        rss: row.memory,
        pss: row.memory * 0.92,
        anonymous: row.memory * (isLlm ? 0.18 : 0.78),
        fileBacked: row.memory * (isLlm ? 0.8 : 0.2),
        shmem: row.memory * 0.02,
        swap: 0,
        stack: 132 * 1024,
        peakRss: row.memory * 1.15,
        locked: 0,
      },
      threadNames: isPy
        ? ["python3", ...Array(8).fill("ThreadPoolExecutor"), "asyncio_0", "tokenizers"]
        : isLlm
          ? ["ollama_llama_se", ...Array(8).fill("llama-worker"), "cuda-EvtHandlr"]
          : [row.name],
    };
  }
  return invoke<ProcessDetail | null>("telemetry_process", { pid });
}

export async function runtimes(): Promise<RuntimeStatus[]> {
  if (!checkIsTauri()) {
    return [
      {
        server: "ollama",
        endpoint: "http://127.0.0.1:11434 (simulated)",
        models: [
          {
            name: "qwen3:4b",
            sizeBytes: 3.3 * GB,
            vramBytes: 2.6 * GB,
            contextLength: 8192,
            quantization: "Q4_K_M",
            parameterSize: "4.0B",
            expiresAt: null,
          },
        ],
        kvCacheUsage: Math.max(0.05, Math.min(0.95, wave(0.45, 0.3, 5))),
        prefixCacheHitRate: 0.62,
        requestsRunning: Math.round(wave(1, 1, 3)),
        requestsWaiting: 0,
        promptTokensTotal: 18000 + tick * 220,
        generatedTokensTotal: 6000 + tick * 48,
        slotContexts: [],
        busySlots: null,
        notes: [],
      },
    ];
  }
  return invoke<RuntimeStatus[]>("telemetry_runtimes");
}
