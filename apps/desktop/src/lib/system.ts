// The running system as boxes: one card per container (or per tier when there is no
// compose file), what code lives in each, and where every example-run hop lands.
// Positions are computed here, not measured, so the camera and the packet can be
// animated without waiting for layout.

import type {
  AiStage,
  ExampleRun,
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
  frontend: "#5ec8ff",
  proxy: "#c9b6ff",
  api: "#7cf0a6",
  worker: "#ffd36b",
  agent: "#ff9ecf",
  modelServer: "#ff8a5c",
  vectorStore: "#b48cff",
  searchEngine: "#9ad0ff",
  database: "#6be3d6",
  cache: "#ff6b6b",
  broker: "#ffc46b",
  objectStore: "#d0d4e4",
  observability: "#a8e06b",
  other: "#dcdcec",
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
}

export interface SystemModel {
  boxes: Box[];
  byId: Map<string, Box>;
  links: ServiceLink[];
  width: number;
  height: number;
  synthetic: boolean;
  totalLimit: number;
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
  const map: [string, ServiceKind, string][] = [
    ["Frontend", "frontend", "web"],
    ["Gateway", "proxy", "gateway"],
    ["Backend", "api", "server"],
    ["Workers", "worker", "workers"],
    ["Storage", "database", "database"],
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

export function buildSystem(ix: Indexed, run?: ExampleRun): SystemModel | null {
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

  const boxes: Box[] = services.map((svc) => {
    const lane = LANE[svc.kind];
    const files = new Set(filesOf(svc));
    const code: Row[] = [];
    const add = (ref: string, text: string, badge?: string, color?: string) => {
      if (code.some((r) => r.ref === ref) || code.length >= 9) return;
      code.push({ key: `c:${ref}`, ref, text, badge, color, y: 0 });
    };
    if (svc.entryFile && files.has(svc.entryFile))
      add(svc.entryFile, pathOf(ix, svc.entryFile), "starts here", "#7cf0a6");
    for (const m of mem)
      if (files.has(m.fileId)) add(m.fileId, pathOf(ix, m.fileId), "memory", "#ff6b6b");
    for (const [f, n] of routesByFile)
      if (files.has(f)) add(f, pathOf(ix, f), `${n} route${n > 1 ? "s" : ""}`, "#c9b6ff");
    const aiFiles = new Map<string, number>();
    for (const st of ai?.stages ?? [])
      if (files.has(st.fileId)) aiFiles.set(st.fileId, (aiFiles.get(st.fileId) ?? 0) + 1);
    for (const [f, n] of [...aiFiles].sort((a, b) => b[1] - a[1]))
      add(f, pathOf(ix, f), `${n} AI step${n > 1 ? "s" : ""}`, "#ff9ecf");
    // Run hops landing here pull their files in too.
    for (const h of run?.hops ?? []) {
      const f = ix.fileOf(h.node) ?? (h.node.startsWith("file:") ? h.node : undefined);
      if (f && files.has(f)) add(f, pathOf(ix, f));
    }
    const pipeline: Row[] = [];
    const inRun = (run?.hops ?? []).filter(
      (h) => h.kind === "stage" && hopService(ix, h, synthetic) === svc.id,
    );
    if (inRun.length) {
      for (const h of inRun)
        pipeline.push({ key: `p:${h.node}`, ref: h.node, text: h.label, y: 0 });
    } else {
      const here = (ai?.stages ?? [])
        .filter((st) => files.has(st.fileId) && ONLINE_ORDER.includes(st.role))
        .sort(
          (a, b) => ONLINE_ORDER.indexOf(a.role) - ONLINE_ORDER.indexOf(b.role) || a.line - b.line,
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
      memory: mem.filter((m) => files.has(m.fileId)),
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
