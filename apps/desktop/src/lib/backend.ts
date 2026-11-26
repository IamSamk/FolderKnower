// Bridge to the Rust side. In a plain browser (vite dev without Tauri) the app runs
// in demo mode on the bundled fixture so the UI can be developed and tried anywhere.

import brokenDemoGraph from "../../../../fixtures/expected/broken-shop.json";
import demoGraph from "../../../../fixtures/expected/fullstack-todo.json";
import aiDemoGraph from "../../../../fixtures/expected/rag-agent.json";
import type { SystemArchitectureGraph, TierLabel } from "../gen";

export function checkIsTauri(): boolean {
  return (
    typeof window !== "undefined" &&
    (Boolean((window as unknown as { isTauri?: boolean }).isTauri) ||
      "__TAURI_INTERNALS__" in window ||
      "__TAURI__" in window)
  );
}

export const isTauri = checkIsTauri();

export interface Progress {
  stage: "scan" | "parse" | "link" | "classify" | "assemble" | "done";
  message: string;
  done: number;
  total: number;
}

export interface RunStats {
  files: number;
  codeFiles: number;
  parsed: number;
  cached: number;
  millis: number;
}

export interface AnalyzeResult {
  graph: SystemArchitectureGraph;
  stats: RunStats;
  root: string;
}

export interface Concept {
  id: string;
  layer: "os" | "network" | "cloud" | "data";
  concept: string;
  explain: string;
}

/** FolderKnower reading its own code: the main example. */
export const DEMO_SELF_ROOT = "demo:folderknower";
export const DEMO_ROOT = "demo:fullstack-todo";
/** A small RAG + agent project: shows the AI pipeline, DSA and memory views. */
export const DEMO_AI_ROOT = "demo:rag-agent";
/** A deliberately broken shop: shows the Problems drawer. */
export const DEMO_BROKEN_ROOT = "demo:broken-shop";
export const isDemo = (root: string | null) =>
  root === DEMO_SELF_ROOT ||
  root === DEMO_ROOT ||
  root === DEMO_AI_ROOT ||
  root === DEMO_BROKEN_ROOT;
const DEMO_GRAPHS: Record<string, () => Promise<unknown>> = {
  // About 3 MB, so it loads only when opened. Regenerate with `pnpm example:self`.
  [DEMO_SELF_ROOT]: () => import("../examples/folderknower.json").then((m) => m.default),
  [DEMO_ROOT]: async () => demoGraph,
  [DEMO_AI_ROOT]: async () => aiDemoGraph,
  [DEMO_BROKEN_ROOT]: async () => brokenDemoGraph,
};

/** This repository's own sources for the self example, each loaded when it is opened. */
const selfSources = {
  ...import.meta.glob("../../../../crates/*/src/**/*.rs", { query: "?raw", import: "default" }),
  ...import.meta.glob("../../../../apps/desktop/src-tauri/src/**/*.rs", {
    query: "?raw",
    import: "default",
  }),
  ...import.meta.glob("../../../../apps/desktop/src/**/*.{ts,tsx}", {
    query: "?raw",
    import: "default",
  }),
  ...import.meta.glob("../../../../apps/web/src/**/*.{ts,tsx}", {
    query: "?raw",
    import: "default",
  }),
} as Record<string, () => Promise<string>>;

const demoSources = {
  ...import.meta.glob("../../../../fixtures/repos/fullstack-todo/**/*", {
    query: "?raw",
    import: "default",
    eager: true,
  }),
  ...import.meta.glob("../../../../fixtures/repos/broken-shop/**/*", {
    query: "?raw",
    import: "default",
    eager: true,
  }),
  ...import.meta.glob("../../../../fixtures/repos/rag-agent/**/*", {
    query: "?raw",
    import: "default",
    eager: true,
  }),
} as Record<string, string>;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function analyze(
  path: string,
  overrides: Record<string, TierLabel>,
  onProgress: (p: Progress) => void,
): Promise<AnalyzeResult> {
  if (isDemo(path) || !checkIsTauri()) {
    const root = isDemo(path) ? path : DEMO_SELF_ROOT;
    const g = structuredClone(await DEMO_GRAPHS[root]()) as unknown as SystemArchitectureGraph;
    const steps: Progress[] = [
      { stage: "scan", message: "Walking the repository", done: 0, total: 0 },
      {
        stage: "scan",
        message: `Found ${g.summary.stats.files + g.nonCodeNodes.length} files`,
        done: 1,
        total: 1,
      },
      {
        stage: "parse",
        message: `Parsing ${g.summary.stats.files} source files with tree-sitter`,
        done: 1,
        total: 1,
      },
      {
        stage: "link",
        message: `Linked ${g.routes.length} routes, ${g.crossTierEdges.length} cross-tier edges`,
        done: 1,
        total: 1,
      },
      { stage: "classify", message: "Deciding what each file does", done: 1, total: 1 },
      { stage: "classify", message: g.summary.headline, done: 1, total: 1 },
      { stage: "done", message: "Done", done: 1, total: 1 },
    ];
    for (const s of steps) {
      onProgress(s);
      await sleep(380);
    }
    for (const t of g.tiers) {
      for (const c of t.clusters)
        for (const f of c.files) if (overrides[f.path]) f.tierConfidence = 1;
    }
    return {
      graph: g,
      stats: {
        files: g.summary.stats.files + g.nonCodeNodes.length,
        codeFiles: g.summary.stats.files,
        parsed: g.summary.stats.files,
        cached: 0,
        millis: 75,
      },
      root,
    };
  }
  const { listen } = await import("@tauri-apps/api/event");
  const un = await listen<Progress>("analyze-progress", (e) => onProgress(e.payload));
  try {
    return await invoke<AnalyzeResult>("analyze", { path, overrides });
  } finally {
    un();
  }
}

export async function readSource(root: string, path: string): Promise<string> {
  if (isDemo(root) || !checkIsTauri()) {
    if (root === DEMO_SELF_ROOT || !isDemo(root)) {
      const load = selfSources[`../../../../${path}`];
      return load ? load() : `// source not bundled in the example: ${path}`;
    }
    const repo = root.replace("demo:", "");
    const key = Object.keys(demoSources).find((k) => k.endsWith(`/${repo}/${path}`));
    return key ? demoSources[key] : `// source not bundled in demo: ${path}`;
  }
  return invoke<string>("read_source", { root, path });
}

import ontology from "./ontology.json";

/** The Systems Ontology catalogue (generated by `folderknower ontology`). */
export const CONCEPTS: Concept[] = ontology as Concept[];
export const CONCEPT_BY_ID: Map<string, Concept> = new Map(CONCEPTS.map((c) => [c.id, c]));

export async function pickFolder(): Promise<string | null> {
  if (!checkIsTauri()) return DEMO_SELF_ROOT;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const res = await open({ directory: true, multiple: false, title: "Choose a repository" });
  return typeof res === "string" ? res : null;
}

export async function onFolderDrop(
  cb: (path: string) => void,
  onHover: (h: boolean) => void,
): Promise<() => void> {
  if (!checkIsTauri()) return () => {};
  const { getCurrentWebview } = await import("@tauri-apps/api/webview");
  return getCurrentWebview().onDragDropEvent((e) => {
    const p = e.payload;
    if (p.type === "enter" || p.type === "over") onHover(true);
    else if (p.type === "leave") onHover(false);
    else if (p.type === "drop") {
      onHover(false);
      if (p.paths[0]) cb(p.paths[0]);
    }
  });
}

export async function exportJson(graph: SystemArchitectureGraph): Promise<string | null> {
  const json = `${JSON.stringify(graph, null, 2)}\n`;
  if (!checkIsTauri()) {
    const url = URL.createObjectURL(new Blob([json], { type: "application/json" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = "architecture.json";
    a.click();
    URL.revokeObjectURL(url);
    return "architecture.json";
  }
  const { save } = await import("@tauri-apps/plugin-dialog");
  const path = await save({
    defaultPath: "architecture.json",
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!path) return null;
  await invoke("export_json", { path, graph });
  return path;
}

/** Copy a Markdown digest (size, tiers, Mermaid diagrams, routes, health) to the clipboard. */
export async function copyDigest(graph: SystemArchitectureGraph): Promise<boolean> {
  if (!checkIsTauri()) return false;
  const md = await invoke<string>("export_markdown", { graph });
  await navigator.clipboard.writeText(md);
  return true;
}

// ---- in-app updater

export interface UpdateInfo {
  version: string;
  currentVersion: string;
  body: string | null;
  date: string | null;
}

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
  done: boolean;
}

/**
 * Ask the Rust side whether a newer signed release is on GitHub.
 * Returns `null` when already up to date, or in demo/browser mode.
 */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  if (!checkIsTauri()) return null;
  return invoke<UpdateInfo | null>("check_update");
}

/** Download, verify and install the update, streaming progress, then relaunch. */
export async function installUpdate(onProgress?: (p: UpdateProgress) => void): Promise<void> {
  if (!checkIsTauri()) return;
  const { listen } = await import("@tauri-apps/api/event");
  const un = onProgress
    ? await listen<UpdateProgress>("update://progress", (e) => onProgress(e.payload))
    : null;
  try {
    await invoke("install_update");
  } finally {
    un?.();
  }
}

// ---- optional local model

export interface SlmStatus {
  available: boolean;
  server: string | null;
  baseUrl: string | null;
  models: string[];
  hint: string;
}

export async function slmStatus(baseUrl?: string): Promise<SlmStatus> {
  if (!checkIsTauri())
    return {
      available: false,
      server: null,
      baseUrl: null,
      models: [],
      hint: "The local-model layer runs in the desktop app (Ollama or llama.cpp on this machine).",
    };
  return invoke<SlmStatus>("slm_status", { baseUrl: baseUrl ?? null });
}

export interface Sentence {
  text: string;
  cites: string[];
}

export async function slmExplain(req: {
  baseUrl: string;
  model: string;
  question: string;
  nodes: { id: string; kind: string; label: string; facts: unknown }[];
  edges: unknown[];
}): Promise<{ sentences: Sentence[]; dropped: number; model: string }> {
  return invoke("slm_explain", { req });
}

/** One step of looking around: the model picks a move from the list, or null to stop. */
export async function slmChoose(req: {
  baseUrl: string;
  model: string;
  question: string;
  seen: string[];
  moves: string[];
}): Promise<string | null> {
  return invoke<string | null>("slm_choose", { req });
}

export async function slmClassify(req: {
  baseUrl: string;
  model: string;
  questionId: string;
  subject: string;
  options: string[];
  files: unknown[];
}): Promise<{ questionId: string; tier: TierLabel; reason: string; model: string }> {
  return invoke("slm_classify", { req });
}

// ---- built-in local AI (llama.cpp runtime + Laya decider + Qwen narrator)

export interface AiComponent {
  id: "runtime" | "decider" | "narrator";
  name: string;
  purpose: string;
  license: string;
  licenseUrl: string;
  sourceUrl: string;
  size: number;
  installed: boolean;
  supported: boolean;
  /** A file the user pointed to that is no longer there. */
  missingPath?: string;
  /** The user's own file, used instead of a download. */
  linked?: { path: string; check: "exact" | "compatible"; modelName?: string };
}

export interface AiStatus {
  components: AiComponent[];
  deciderReady: boolean;
  narratorReady: boolean;
  narratorBaseUrl: string | null;
  /** Where downloads are saved. */
  installDir: string;
}

/** A model or llama-server already on this computer. */
export interface AiFound {
  id: AiComponent["id"];
  path: string;
  size: number;
  likelyExact: boolean;
  label: string;
}

export interface AiProgress {
  id: string;
  done: number;
  total: number;
  phase: "download" | "verified" | "extract" | "check";
}

export async function aiStatus(): Promise<AiStatus | null> {
  if (!checkIsTauri()) return null;
  return invoke<AiStatus>("ai_status");
}

export async function aiInstall(
  id: string,
  onProgress: (p: AiProgress) => void,
): Promise<AiStatus> {
  const { listen } = await import("@tauri-apps/api/event");
  const un = await listen<AiProgress>("ai-progress", (e) => onProgress(e.payload));
  try {
    return await invoke<AiStatus>("ai_install", { id });
  } finally {
    un();
  }
}

/** Use a file the user already has; the app checks it before saving the link. */
export async function aiLink(
  id: string,
  path: string,
  onProgress: (p: AiProgress) => void,
): Promise<AiStatus> {
  const { listen } = await import("@tauri-apps/api/event");
  const un = await listen<AiProgress>("ai-progress", (e) => onProgress(e.payload));
  try {
    return await invoke<AiStatus>("ai_link", { id, path });
  } finally {
    un();
  }
}

/** Look in the usual model folders, or only in `folder` when given. */
export async function aiFind(folder?: string): Promise<AiFound[]> {
  return invoke<AiFound[]>("ai_find", { folder: folder ?? null });
}

export interface AiRequirements {
  rows: {
    what: string;
    minimum: string;
    recommended: string;
    yours: string;
    verdict: "good" | "ok" | "low";
  }[];
  suggestion: string;
}

export async function aiRequirements(): Promise<AiRequirements> {
  return invoke<AiRequirements>("ai_requirements");
}

/** Ask for a file: a .gguf model, or the llama-server program. */
export async function pickModelFile(id: string): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const res = await open({
    directory: false,
    multiple: false,
    title: id === "runtime" ? "Choose llama-server" : "Choose a .gguf model file",
    filters: id === "runtime" ? undefined : [{ name: "GGUF model", extensions: ["gguf"] }],
  });
  return typeof res === "string" ? res : null;
}

export async function aiRemove(id: string): Promise<AiStatus> {
  return invoke<AiStatus>("ai_remove", { id });
}

export async function aiStartNarrator(): Promise<string> {
  return invoke<string>("ai_start_narrator");
}

export interface Decision {
  choice: string;
  probabilities: Record<string, number>;
  confidence: number;
}

/** Ask the Laya decision model to pick one option. */
export async function aiDecide(
  state: string,
  instructions: string,
  criteria: Record<string, string>,
): Promise<Decision> {
  const r = await invoke<{ answers: { q: Decision } }>("ai_decide", {
    stateText: state,
    questions: { q: { type: "choice", instructions, criteria } },
  });
  return r.answers.q;
}
