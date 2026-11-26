// The running system as boxes: one card per container (or per tier when there is no
// compose file), what code lives in each, and where every example-run hop lands.
// Positions are computed here, not measured, so the camera and the packet can be
// animated without waiting for layout.

import type {
  AiStage,
  ExampleRun,
  Feature,
  MemoryPlacement,
  MemoryTechnique,
  RunHop,
  Service,
  ServiceKind,
  ServiceLink,
} from "../gen";
import type { Indexed } from "./graph";

export const SVC_LABEL: Record<ServiceKind, string> = {
  frontend: "Frontend",
  proxy: "Proxy / gateway",
  api: "Backend API",
  worker: "Worker",
  agent: "Agent",
  modelServer: "Model server",
  vectorStore: "Vector store",
  searchEngine: "Search engine",
  database: "Database",
  cache: "Cache",
  broker: "Queue / broker",
  objectStore: "Object storage",
  observability: "Observability",
  other: "Service",
};

export const SVC_COLOR: Record<ServiceKind, string> = {
  frontend: "oklch(79.2% 0.125 234)",
  proxy: "oklch(81.8% 0.103 296)",
  api: "oklch(86.8% 0.149 154)",
  worker: "oklch(88.5% 0.132 87)",
  agent: "oklch(81.2% 0.129 349)",
  modelServer: "oklch(75.3% 0.155 42)",
  vectorStore: "oklch(72.5% 0.165 298)",
  searchEngine: "oklch(83.7% 0.087 245)",
  database: "oklch(84.3% 0.110 186)",
  cache: "oklch(71.2% 0.181 23)",
  broker: "oklch(85.6% 0.126 76)",
  objectStore: "oklch(87.2% 0.022 275)",
  observability: "oklch(84.3% 0.159 130)",
  other: "oklch(89.9% 0.022 286)",
};

/** Columns, left to right: what the user touches, the edge, the code, what it leans on. */
const LANE: Record<ServiceKind, number> = {
  frontend: 0,
  proxy: 1,
  api: 2,
  agent: 2,
  worker: 2,
  modelServer: 3,
  vectorStore: 3,
  searchEngine: 3,
  database: 3,
  cache: 3,
  broker: 3,
  objectStore: 3,
  observability: 3,
  other: 3,
};
export const LANE_NAME = ["In the browser", "At the edge", "Your code", "What it leans on"];
/** A desktop app has no browser or edge: its UI is the app window. */
export const DESKTOP_LANE_NAME = [
  "In the app window",
  "At the edge",
  "On the computer",
  "What it keeps",
];

/** Kind label for a box, in a desktop app's own words where they differ. */
export function svcLabel(ix: Indexed | null, s: Service): string {
  if (ix?.g.summary.kind === "desktop" && s.id === "tier:backend") return "Native code";
  if (ix?.g.summary.kind === "desktop" && s.id === "tier:frontend") return "Web view";
  if (ix?.g.summary.kind === "desktop" && s.id === "tier:storage") return "Files and SQLite";
  return SVC_LABEL[s.kind];
}

/** How much the diagram says: plain words, the usual boxes, or every file and package. */
export type Depth = "simple" | "standard" | "deep";

/** Non-technical names for the simple diagram. */
export const PLAIN_NAME: Record<ServiceKind, string> = {
  frontend: "Website",
  proxy: "Front door",
  api: "Server",
  worker: "Background worker",
  agent: "AI agent",
  modelServer: "AI model",
  vectorStore: "Knowledge index",
  searchEngine: "Search index",
  database: "Database",
  cache: "Short-term memory",
  broker: "Job queue",
  objectStore: "File storage",
  observability: "Monitoring",
  other: "Helper service",
};

/** Desktop apps have a window and a native side, not a website and a server. */
const DESKTOP_PLAIN: Record<string, [string, string]> = {
  "tier:frontend": [
    "App window",
    "The screens people see and click, drawn by a web view inside the app.",
  ],
  "tier:backend": [
    "Native side",
    "Code that runs on the computer itself: reads files, runs tools and models, and answers the window's commands.",
  ],
  "tier:storage": ["Saved data", "Files and small databases the app keeps on this computer."],
};

export function plainName(ix: Indexed, s: Service): string {
  return (ix.g.summary.kind === "desktop" && DESKTOP_PLAIN[s.id]?.[0]) || PLAIN_NAME[s.kind];
}

export function plainWhat(ix: Indexed, s: Service): string {
  return (ix.g.summary.kind === "desktop" && DESKTOP_PLAIN[s.id]?.[1]) || PLAIN_WHAT[s.kind];
}

export const PLAIN_WHAT: Record<ServiceKind, string> = {
  frontend: "The pages people see and click in their browser.",
  proxy: "Takes every request first and passes it to the right place.",
  api: "Runs the app's rules: checks input, does the work, sends the answer back.",
  worker: "Does slow jobs in the background so nobody waits.",
  agent: "Decides which tool to use next, step by step.",
  modelServer: "The AI model that reads text and writes the answer.",
  vectorStore: "Finds the passages that mean the same as a question.",
  searchEngine: "Finds documents by the words they contain.",
  database: "Keeps the app's records safe, even after a restart.",
  cache: "Remembers recent things for a short time so repeats are instant.",
  broker: "Holds jobs in line until a worker picks them up.",
  objectStore: "Keeps uploaded files and images.",
  observability: "Collects logs and numbers about how the system is doing.",
  other: "A supporting program the system needs.",
};

const PLAIN_VERB: Partial<Record<ServiceKind, string>> = {
  api: "asks",
  database: "saves & reads",
  cache: "remembers",
  vectorStore: "looks up",
  searchEngine: "searches",
  modelServer: "asks the AI",
  broker: "hands jobs to",
  objectStore: "stores files in",
  worker: "hands work to",
};

export const PLACE_NAME: Record<MemoryPlacement["place"], string> = {
  vram: "GPU memory",
  ram: "RAM",
  disk: "Disk",
  browser: "Browser",
  remote: "Provider",
};

export const PLACE_COLOR: Record<MemoryPlacement["place"], string> = {
  vram: "oklch(75.3% 0.155 42)",
  ram: "oklch(79.2% 0.125 234)",
  disk: "oklch(84.3% 0.110 186)",
  browser: "oklch(88.5% 0.132 87)",
  remote: "oklch(81.8% 0.103 296)",
};

export const LIFE_NAME: Record<MemoryPlacement["lifetime"], string> = {
  request: "per request",
  session: "per session",
  process: "until restart",
  persistent: "kept",
};

export interface BuildOpts {
  depth?: Depth;
  feature?: Feature;
}

export const CARD_W = [300, 230, 330, 250];
export const HEAD_H = 92;
export const ROW_H = 26;
export const SECTION_H = 24;
export const GAP_X = 110;
export const GAP_Y = 26;
export const PAD = 40;
export const PREVIEW_H = 150;

export interface Row {
  key: string;
  /** File id or stage id the row stands for. */
  ref: string;
  text: string;
  badge?: string;
  color?: string;
  y: number;
}

export interface Box {
  svc: Service;
  synthetic: boolean;
  lane: number;
  x: number;
  y: number;
  w: number;
  h: number;
  color: string;
  code: Row[];
  pipeline: Row[];
  preview: boolean;
  memory: MemoryTechnique[];
  /** Outside the selected feature: drawn faded. */
  dim: boolean;
  /** Packages used by the code in this box (deep), most used first. */
  stack: string[];
  /** Features this box serves. */
  features: Feature[];
  /** Where data sits in this box (deep). */
  places: MemoryPlacement[];
  /** Heights of the extra sections, for layout and rendering. */
  extraH: number;
}

export interface SystemModel {
  boxes: Box[];
  byId: Map<string, Box>;
  links: ServiceLink[];
  width: number;
  height: number;
  synthetic: boolean;
  totalLimit: number;
  depth: Depth;
}

/** Box ids a feature touches: its services, or the tiers of its files without compose. */
export function featureBoxes(ix: Indexed, f: Feature, synthetic: boolean): Set<string> {
  const out = new Set<string>(synthetic ? [] : f.services);
  const files = [...f.files, ...f.sharedFiles];
  const services = ix.g.insights?.deployment?.services ?? [];
  for (const id of files) {
    if (synthetic) {
      const t = ix.tierOf(id);
      if (t) out.add(`tier:${t.toLowerCase()}`);
    } else {
      const s = serviceOfFile(services, id.replace(/^file:/, ""));
      if (s) out.add(s.id);
    }
  }
  if (synthetic && f.tables.length) out.add("tier:storage");
  if (synthetic && f.ui.length) out.add("tier:frontend");
  return out;
}

/** Plain or technical label for a link. */
export function linkLabel(l: ServiceLink, m: SystemModel): string {
  if (m.depth === "simple") {
    const to = m.byId.get(l.to);
    if (l.via === "proxy") return "forwards to";
    return (to && PLAIN_VERB[to.svc.kind]) ?? "uses";
  }
  if (l.via === "proxy") return "proxy_pass";
  return m.depth === "deep" ? l.detail : l.detail.split(" -> ")[0];
}

export const STACK_LINE = 22;

/** Height of a wrapped row of chips (section title included), estimated from text width. */
function chipBlockH(labels: string[], boxW: number, mono = false): number {
  if (!labels.length) return 0;
  const avail = boxW - 24;
  const ch = mono ? 6.9 : 6.2;
  let lines = 1;
  let x = 0;
  for (const l of labels) {
    const w = Math.min(avail, l.length * ch + 20);
    if (x > 0 && x + w > avail) {
      lines += 1;
      x = 0;
    }
    x += w + 4;
  }
  return SECTION_H + lines * STACK_LINE + 6;
}

const ONLINE_ORDER: AiStage["role"][] = [
  "agent",
  "router",
  "tool",
  "retriever",
  "vectorStore",
  "keywordIndex",
  "fusion",
  "reranker",
  "prompt",
  "llm",
  "memory",
  "cache",
  "guardrail",
  "evaluator",
];

function pathOf(ix: Indexed, fileId: string) {
  return ix.files.get(fileId)?.file.path ?? fileId.replace(/^file:/, "");
}

/** Deepest build context that holds the file. */
export function serviceOfFile(services: Service[], path: string): Service | undefined {
  let best: Service | undefined;
  for (const s of services) {
    const c = s.buildContext;
    if (c === undefined) continue;
    if (c === "" || path.startsWith(`${c}/`)) {
      if (!best || (best.buildContext ?? "").length < c.length) best = s;
    }
  }
  return best;
}

/** Without compose: one box per code tier, so the story still has places to go. */
function syntheticServices(ix: Indexed): { services: Service[]; links: ServiceLink[] } {
  const desktop = ix.g.summary.kind === "desktop";
  const map: [string, ServiceKind, string][] = [
    ["Frontend", "frontend", desktop ? "window" : "web"],
    ["Gateway", "proxy", "gateway"],
    ["Backend", "api", desktop ? "native" : "server"],
    ["Workers", "worker", "workers"],
    ["Storage", "database", desktop ? "local data" : "database"],
  ];
  const services: Service[] = [];
  for (const [tier, kind, name] of map) {
    const t = ix.tiers.get(tier as never);
    const n = t ? t.clusters.reduce((a, c) => a + c.files.length, 0) : 0;
    if (!n) continue;
    services.push({
      id: `tier:${tier.toLowerCase()}`,
      name,
      kind,
      ports: [],
      volumes: [],
      envKeys: [],
      codeFiles: n,
      declaredIn: "",
      explain: `No compose file was found, so this box is the ${tier.toLowerCase()} code (${n} files) grouped by what it does.`,
    });
  }
  const links: ServiceLink[] = [];
  for (const e of ix.g.crossTierEdges) {
    const a = ix.tierOf(e.source);
    const b = ix.tierOf(e.target);
    if (!a || !b || a === b) continue;
    const from = `tier:${a.toLowerCase()}`;
    const to = `tier:${b.toLowerCase()}`;
    if (!services.some((s) => s.id === from) || !services.some((s) => s.id === to)) continue;
    if (e.protocol === "IPC") {
      const n = ix.g.crossTierEdges.filter(
        (x) => x.protocol === "IPC" && ix.tierOf(x.source) === a && ix.tierOf(x.target) === b,
      ).length;
      const detail = `invoke() -> ${n} native command${n === 1 ? "" : "s"}`;
      const have = links.find((l) => l.from === from && l.to === to);
      if (have) have.detail = detail;
      else links.push({ from, to, via: "env", detail });
      continue;
    }
    if (!links.some((l) => l.from === from && l.to === to))
      links.push({ from, to, via: "env", detail: e.evidence[0] ?? e.protocol });
  }
  return { services, links };
}

export function hopService(ix: Indexed, h: RunHop, synthetic: boolean): string | undefined {
  if (!synthetic) return h.service;
  const t = ix.tierOf(h.node) ?? ix.tierOf(h.node.replace(/^route:.*#/, "file:"));
  if (h.kind === "ui" || h.kind === "response") return "tier:frontend";
  if (h.kind === "data") return "tier:storage";
  if (h.kind === "stage") {
    const st = ix.g.insights?.ai?.stages.find((s) => s.id === h.node);
    const tt = st ? ix.tierOf(st.fileId) : undefined;
    return tt ? `tier:${tt.toLowerCase()}` : "tier:backend";
  }
  return t ? `tier:${t.toLowerCase()}` : undefined;
}

export function buildSystem(
  ix: Indexed,
  run?: ExampleRun,
  opts: BuildOpts = {},
): SystemModel | null {
  const depth = opts.depth ?? "standard";
  const feature = opts.feature;
  const features = ix.g.features ?? [];
  const placements = ix.g.memoryMap ?? [];
  const pretty = new Map<string, string>();
  for (const lib of ix.g.insights?.libraries ?? [])
    for (const ev of lib.evidence) pretty.set(ev.split(/[./]/)[0].replace(/-/g, "_"), lib.name);
  const dep = ix.g.insights?.deployment;
  const synthetic = !dep || dep.services.length === 0;
  const { services, links } = synthetic
    ? syntheticServices(ix)
    : { services: dep.services, links: dep.links };
  if (!services.length) return null;
  const ai = ix.g.insights?.ai;
  const mem = ix.g.insights?.memoryTechniques ?? [];
  const routesByFile = new Map<string, number>();
  for (const r of ix.g.routes) routesByFile.set(r.fileId, (routesByFile.get(r.fileId) ?? 0) + 1);

  const filesOf = (s: Service): string[] => {
    if (synthetic) {
      const tier = s.id.replace("tier:", "");
      return [...ix.files.values()]
        .filter((f) => f.tier.toLowerCase() === tier)
        .map((f) => f.file.id);
    }
    return [...ix.files.values()]
      .filter((f) => serviceOfFile(services, f.file.path)?.id === s.id)
      .map((f) => f.file.id);
  };

  const broken = new Set(
    (ix.g.health?.issues ?? []).filter((i) => i.severity === "error").map((i) => i.fileId),
  );
  const fFiles = feature ? new Set([...feature.files, ...feature.sharedFiles]) : undefined;
  const fStages = feature ? new Set(feature.aiStages) : undefined;
  const fBoxes = feature ? featureBoxes(ix, feature, synthetic) : undefined;
  const boxes: Box[] = services.map((svc) => {
    const lane = LANE[svc.kind];
    const allFiles = new Set(filesOf(svc));
    const files = fFiles ? new Set([...allFiles].filter((f) => fFiles.has(f))) : allFiles;
    const code: Row[] = [];
    const cap = depth === "deep" ? 16 : 9;
    const add = (ref: string, text: string, badge?: string, color?: string) => {
      if (depth === "simple" || code.some((r) => r.ref === ref) || code.length >= cap) return;
      code.push({ key: `c:${ref}`, ref, text, badge, color, y: 0 });
    };
    for (const f of broken)
      if (files.has(f)) add(f, pathOf(ix, f), "broken", "oklch(71.2% 0.181 23)");
    if (svc.entryFile && files.has(svc.entryFile))
      add(svc.entryFile, pathOf(ix, svc.entryFile), "starts here", "oklch(86.8% 0.149 154)");
    for (const m of mem)
      if (files.has(m.fileId))
        add(m.fileId, pathOf(ix, m.fileId), "memory", "oklch(71.2% 0.181 23)");
    for (const [f, n] of routesByFile)
      if (files.has(f))
        add(
          f,
          pathOf(ix, f),
          ix.g.routes.some((r) => r.fileId === f && r.method === "IPC")
            ? `${n} command${n > 1 ? "s" : ""}`
            : `${n} route${n > 1 ? "s" : ""}`,
          "oklch(81.8% 0.103 296)",
        );
    const aiFiles = new Map<string, number>();
    for (const st of ai?.stages ?? [])
      if (files.has(st.fileId)) aiFiles.set(st.fileId, (aiFiles.get(st.fileId) ?? 0) + 1);
    for (const [f, n] of [...aiFiles].sort((a, b) => b[1] - a[1]))
      add(f, pathOf(ix, f), `${n} AI step${n > 1 ? "s" : ""}`, "oklch(81.2% 0.129 349)");
    // Run hops landing here pull their files in too.
    for (const h of run?.hops ?? []) {
      const f = ix.fileOf(h.node) ?? (h.node.startsWith("file:") ? h.node : undefined);
      if (f && files.has(f)) add(f, pathOf(ix, f));
    }
    if (depth === "deep")
      for (const f of [...files].sort()) {
        const fe = ix.files.get(f);
        add(f, pathOf(ix, f), fe?.file.role ?? undefined, "oklch(83.7% 0.087 245)");
      }
    const pipeline: Row[] = [];
    const inRun = (run?.hops ?? []).filter(
      (h) => h.kind === "stage" && hopService(ix, h, synthetic) === svc.id,
    );
    if (depth === "simple") {
      // no rows: the simple diagram is boxes and plain sentences only
    } else if (inRun.length) {
      for (const h of inRun)
        pipeline.push({ key: `p:${h.node}`, ref: h.node, text: h.label, y: 0 });
    } else {
      const here = (ai?.stages ?? [])
        .filter(
          (st) =>
            files.has(st.fileId) && (fStages ? fStages.has(st.id) : ONLINE_ORDER.includes(st.role)),
        )
        .sort(
          (a, b) =>
            (fStages ? 0 : ONLINE_ORDER.indexOf(a.role) - ONLINE_ORDER.indexOf(b.role)) ||
            a.fileId.localeCompare(b.fileId) ||
            a.line - b.line,
        );
      for (const st of here.slice(0, 10))
        pipeline.push({ key: `p:${st.id}`, ref: st.id, text: st.name, y: 0 });
    }
    const preview = svc.kind === "frontend" && !!run;
    let y = HEAD_H + (preview ? PREVIEW_H : 0);
    if (code.length) {
      y += SECTION_H;
      for (const r of code) {
        r.y = y;
        y += ROW_H;
      }
    }
    if (pipeline.length) {
      y += SECTION_H;
      for (const r of pipeline) {
        r.y = y;
        y += ROW_H;
      }
    }
    const served = features.filter((f) => featureBoxes(ix, f, synthetic).has(svc.id));
    const counts = new Map<string, number>();
    if (depth === "deep")
      for (const f of files)
        for (const p of ix.files.get(f)?.file.packages ?? []) {
          const name = pretty.get(p.split(/[./]/)[0].replace(/-/g, "_")) ?? p.replace(/_/g, "-");
          counts.set(name, (counts.get(name) ?? 0) + 1);
        }
    const stack = [...counts]
      .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
      .map((x) => x[0])
      .slice(0, 10);
    const places =
      depth === "deep"
        ? placements.filter(
            (p) =>
              p.holder === svc.id &&
              (!feature ||
                fStages?.has(p.source) ||
                p.proof.some((x) => fFiles?.has(`file:${x.replace(/:\d+$/, "")}`))),
          )
        : [];
    const w = CARD_W[lane];
    let extraH = 0;
    if (depth === "simple")
      extraH += chipBlockH(
        served.map((f) => f.name),
        w,
      );
    if (depth === "deep")
      extraH +=
        chipBlockH(stack, w, true) + (places.length ? SECTION_H + places.length * ROW_H : 0);
    y += extraH;
    return {
      svc,
      synthetic,
      lane,
      x: 0,
      y: 0,
      w: CARD_W[lane],
      h: y + 14,
      color: SVC_COLOR[svc.kind],
      code,
      pipeline,
      preview,
      memory: mem.filter((m) => allFiles.has(m.fileId)),
      dim: !!fBoxes && !fBoxes.has(svc.id),
      stack,
      features: served,
      places,
      extraH,
    };
  });

  // Stack each lane, then centre the lanes vertically against the tallest one.
  const lanes = [0, 1, 2, 3].filter((l) => boxes.some((b) => b.lane === l));
  let x = PAD;
  let height = 0;
  const colH = new Map<number, number>();
  for (const l of lanes) {
    const inLane = boxes.filter((b) => b.lane === l);
    const h = inLane.reduce((a, b) => a + b.h, 0) + GAP_Y * (inLane.length - 1);
    colH.set(l, h);
    height = Math.max(height, h);
  }
  for (const l of lanes) {
    let y = PAD + 30 + (height - (colH.get(l) ?? 0)) / 2;
    for (const b of boxes.filter((b) => b.lane === l)) {
      b.x = x;
      b.y = y;
      y += b.h + GAP_Y;
    }
    x += CARD_W[l] + GAP_X;
  }
  const byId = new Map(boxes.map((b) => [b.svc.id, b]));
  return {
    boxes,
    byId,
    links: links.filter((l) => byId.has(l.from) && byId.has(l.to)),
    width: x - GAP_X + PAD,
    height: height + PAD * 2 + 30,
    synthetic,
    totalLimit: services.reduce((a, s) => a + (s.memoryLimit ?? 0), 0),
    depth,
  };
}

/** Where the packet sits for a hop: on its row inside the box, or on the box header. */
export function hopAnchor(
  ix: Indexed,
  m: SystemModel,
  h: RunHop,
): { x: number; y: number; box?: Box; row?: Row } {
  const sid = hopService(ix, h, m.synthetic);
  const box = sid ? m.byId.get(sid) : undefined;
  if (!box) return { x: m.width / 2, y: m.height / 2 };
  const file = ix.fileOf(h.node) ?? (h.node.startsWith("file:") ? h.node : undefined);
  const route = ix.routes.get(h.node);
  if (box.preview && (h.kind === "ui" || h.kind === "response"))
    return { x: box.x + box.w / 2, y: box.y + HEAD_H + PREVIEW_H / 2, box };
  const row =
    box.pipeline.find((r) => r.ref === h.node) ??
    box.code.find((r) => r.ref === (route?.fileId ?? file));
  if (row) return { x: box.x + box.w - 18, y: box.y + row.y + ROW_H / 2, box, row };
  return { x: box.x + box.w / 2, y: box.y + 34, box };
}

/** Where each link meets its two boxes, spread down the box edge so labels don't pile up. */
export function linkPorts(m: SystemModel): Map<ServiceLink, { ay: number; by: number }> {
  const out = new Map<ServiceLink, { ay: number; by: number }>();
  const slots = new Map<string, ServiceLink[]>();
  const side = (box: Box, other: Box) => `${box.svc.id}|${other.x > box.x ? "r" : "l"}`;
  for (const l of m.links) {
    const a = m.byId.get(l.from) as Box;
    const b = m.byId.get(l.to) as Box;
    for (const [x, y] of [
      [a, b],
      [b, a],
    ] as const) {
      const k = side(x, y);
      slots.set(k, [...(slots.get(k) ?? []), l]);
    }
  }
  const yAt = (box: Box, other: Box, l: ServiceLink) => {
    const list = (slots.get(side(box, other)) ?? []).slice().sort((p, q) => {
      const op = m.byId.get(p.from === box.svc.id ? p.to : p.from) as Box;
      const oq = m.byId.get(q.from === box.svc.id ? q.to : q.from) as Box;
      return op.y - oq.y;
    });
    const i = list.indexOf(l);
    const span = Math.min(box.h - 40, 28 * list.length);
    const top = box.y + Math.min(box.h / 2, 60) - span / 2 + 14;
    return list.length === 1 ? box.y + Math.min(box.h / 2, 60) : top + (span / list.length) * i;
  };
  for (const l of m.links) {
    const a = m.byId.get(l.from) as Box;
    const b = m.byId.get(l.to) as Box;
    out.set(l, { ay: yAt(a, b, l), by: yAt(b, a, l) });
  }
  return out;
}

/** Curve from the right edge of one box to the left edge of another (or around). */
export function linkPath(a: Box, b: Box, ports?: { ay: number; by: number }): string {
  const ay = ports?.ay ?? a.y + Math.min(a.h / 2, 60);
  const by = ports?.by ?? b.y + Math.min(b.h / 2, 60);
  if (a.lane === b.lane) {
    const x = a.x + a.w;
    const bend = x + 46;
    return `M${x},${ay} C${bend},${ay} ${bend},${by} ${x},${by}`;
  }
  const [l, r] = a.x < b.x ? [a, b] : [b, a];
  const ly = l === a ? ay : by;
  const ry = r === a ? ay : by;
  const x1 = l.x + l.w;
  const x2 = r.x;
  const mid = (x1 + x2) / 2;
  return `M${x1},${ly} C${mid},${ly} ${mid},${ry} ${x2},${ry}`;
}

/** `path:line` proof -> a graph id the app can reveal. */
export function proofTarget(ix: Indexed, p: string): string | undefined {
  const m = /^(.+?):(\d+)$/.exec(p);
  if (!m) return undefined;
  const fileId = `file:${m[1]}`;
  const fe = ix.files.get(fileId);
  if (!fe) return undefined;
  const line = Number(m[2]);
  const fn = fe.file.functions.find((f) => f.lineRange[0] <= line && line <= f.lineRange[1]);
  return fn?.id ?? fileId;
}

/** One plain sentence about the whole system, for the simple diagram. */
export function plainHeadline(ix: Indexed, parts: number): string {
  const kind = ix.g.summary.kind;
  const what = ix.g.insights?.ai
    ? "An AI app"
    : kind === "desktop"
      ? "A desktop app"
      : kind === "full-stack"
        ? "A website with its own server"
        : kind === "frontend"
          ? "A website"
          : kind === "backend"
            ? "A server program"
            : kind === "cli"
              ? "A command-line tool"
              : kind === "worker"
                ? "A background program"
                : kind === "library"
                  ? "A code library"
                  : "A program";
  const names = (ix.g.features ?? []).map((f) => f.name);
  const list =
    names.length > 1 ? `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}` : names[0];
  return `${what} made of ${parts} part${parts === 1 ? "" : "s"}${list ? `. It helps people with ${list}.` : "."}`;
}
