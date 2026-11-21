import { create } from "zustand";
import type { TierLabel } from "./gen";
import {
  type AnalyzeResult,
  analyze,
  type Progress,
  type SlmStatus,
  slmStatus,
} from "./lib/backend";
import { type Hop, type Indexed, index, trace } from "./lib/graph";

export type View = "architecture" | "er" | "ui";
export type Focus =
  | { level: "system" }
  | { level: "tier"; tier: TierLabel }
  | { level: "cluster"; clusterId: string }
  | { level: "file"; fileId: string }
  | { level: "function"; fnId: string };

export interface LogLine {
  id: number;
  stage: Progress["stage"];
  text: string;
}

interface State {
  phase: "welcome" | "analyzing" | "ready" | "error";
  root: string | null;
  result: AnalyzeResult | null;
  ix: Indexed | null;
  log: LogLine[];
  error: string | null;
  overrides: Record<string, TierLabel>;
  view: View;
  stack: Focus[];
  selected: string | null;
  hovered: string | null;
  inspect: boolean;
  hops: Hop[];
  travelling: string | null;
  slm: SlmStatus | null;
  model: string | null;
  paletteOpen: boolean;

  start: (path: string) => Promise<void>;
  answer: (subject: string, tier: TierLabel) => Promise<void>;
  reset: () => void;
  setView: (v: View) => void;
  drill: (f: Focus) => void;
  back: (toIndex: number) => void;
  select: (id: string | null) => void;
  hover: (id: string | null) => void;
  setInspect: (b: boolean) => void;
  reveal: (id: string) => void;
  travel: (edgeId: string, then: () => void) => void;
  refreshSlm: () => Promise<void>;
  setModel: (m: string) => void;
  setPalette: (b: boolean) => void;
}

let logId = 0;

export const useStore = create<State>((set, get) => ({
  phase: "welcome",
  root: null,
  result: null,
  ix: null,
  log: [],
  error: null,
  overrides: {},
  view: "architecture",
  stack: [{ level: "system" }],
  selected: null,
  hovered: null,
  inspect: false,
  hops: [],
  travelling: null,
  slm: null,
  model: null,
  paletteOpen: false,

  async start(path) {
    set({ phase: "analyzing", root: path, log: [], error: null });
    try {
      const result = await analyze(path, get().overrides, (p) =>
        set((s) => ({
          log: [...s.log, { id: logId++, stage: p.stage, text: p.message }].slice(-60),
        })),
      );
      set({
        result,
        ix: index(result.graph),
        phase: "ready",
        stack: [{ level: "system" }],
        selected: null,
        hops: [],
        view: "architecture",
      });
      void get().refreshSlm();
    } catch (e) {
      set({ phase: "error", error: String(e) });
    }
  },

  async answer(subject, tier) {
    const overrides = { ...get().overrides, [subject]: tier };
    set({ overrides });
    const root = get().root;
    if (root) {
      const keepView = get().view;
      await get().start(root);
      set({ view: keepView });
    }
  },

  reset: () =>
    set({ phase: "welcome", result: null, ix: null, root: null, overrides: {}, log: [] }),
  setView: (view) => set({ view, hops: view === "ui" ? get().hops : get().hops }),
  drill: (f) => set((s) => ({ stack: [...s.stack, f] })),
  back: (toIndex) => set((s) => ({ stack: s.stack.slice(0, toIndex + 1) })),
  select(id) {
    const ix = get().ix;
    set({ selected: id, hops: id && ix ? trace(ix, id) : [] });
  },
  hover: (id) => set({ hovered: id }),
  setInspect: (inspect) => set({ inspect }),

  reveal(id) {
    const ix = get().ix;
    if (!ix) return;
    if (ix.models.has(id)) {
      set({ view: "er" });
      get().select(id);
      return;
    }
    if (ix.ui.has(id)) {
      set({ view: "ui" });
      get().select(id);
      return;
    }
    const fileId = ix.fileOf(id);
    const fe = fileId ? ix.files.get(fileId) : undefined;
    if (!fe) return;
    const stack: Focus[] = [
      { level: "system" },
      { level: "tier", tier: fe.tier },
      { level: "cluster", clusterId: fe.cluster.id },
    ];
    if (ix.fns.has(id) || ix.routes.has(id)) stack.push({ level: "file", fileId: fe.file.id });
    set({ view: "architecture", stack });
    get().select(id);
  },

  travel(edgeId, then) {
    set({ travelling: edgeId });
    setTimeout(() => {
      set({ travelling: null });
      then();
    }, 720);
  },

  async refreshSlm() {
    const s = await slmStatus();
    set({ slm: s, model: get().model ?? s.models[0] ?? null });
  },
  setModel: (model) => set({ model }),
  setPalette: (paletteOpen) => set({ paletteOpen }),
}));
