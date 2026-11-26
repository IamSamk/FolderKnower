// What the local model is told about a selection, and the small set of moves it may make
// to look around before it explains. Every move reads the static graph or one source
// file: the model picks from a fixed list and can never invent a target.

import { useStore } from "../store";
import { readSource } from "./backend";
import type { Indexed } from "./graph";

export interface Node {
  id: string;
  kind: string;
  label: string;
  facts: Record<string, unknown>;
}

export interface Context {
  nodes: Map<string, Node>;
  edges: unknown[];
  /** Moves already made, so they are not offered twice. */
  done: Set<string>;
}

export type Tool = "source" | "callers" | "calls" | "samefile" | "subsystem";

export interface Move {
  /** What the model sees and picks. */
  key: string;
  tool: Tool;
  id: string;
}

const MAX_NODES = 24;
// The narrator has an 8k-token window: a few short excerpts fit next to the graph facts.
const MAX_SOURCES = 3;
const SOURCE_CHARS = 2200;

export interface Selection {
  id: string;
  kind: string;
  label: string;
  where: string;
}

/** What "Explain" is about, or null when the selection has nothing to explain. */
export function selection(ix: Indexed, id: string): Selection | null {
  const fn = ix.fns.get(id);
  if (fn) return { id, kind: "function", label: fn.fn.name, where: fn.file.path };
  const file = ix.files.get(id);
  if (file) return { id, kind: "file", label: ix.label(id), where: file.file.path };
  const cluster = ix.clusters.get(id);
  if (cluster)
    return { id, kind: "subsystem", label: cluster.cluster.name, where: `${cluster.tier} tier` };
  const model = ix.models.get(id);
  if (model) return { id, kind: "table", label: model.name, where: model.fileId.slice(5) };
  const route = ix.routes.get(id);
  if (route) return { id, kind: "route", label: ix.label(id), where: route.fileId.slice(5) };
  const ui = ix.ui.get(id);
  if (ui) return { id, kind: "UI element", label: ix.label(id), where: ui.fileId.slice(5) };
  return null;
}

function describe(ix: Indexed, id: string): Node | null {
  const sel = selection(ix, id);
  if (!sel) return null;
  const node = (facts: Record<string, unknown>): Node => ({
    id,
    kind: sel.kind,
    label: sel.label,
    facts,
  });
  const fe = ix.fns.get(id);
  if (fe)
    return node({
      file: fe.file.path,
      tier: fe.tier,
      subsystem: ix.clusters.get(fe.clusterId)?.cluster.name,
      signature: fe.fn.signature,
      bigO: fe.fn.complexity.timeBigO,
      calledBy: fe.fn.calledBy.length,
      concepts: fe.fn.ontologyTags.map((t) => t.concept),
      routes: (ix.routesByHandler.get(id) ?? []).map((r) => `${r.method} ${r.path}`),
    });
  const f = ix.files.get(id);
  if (f)
    return node({
      tier: f.tier,
      subsystem: f.cluster.name,
      role: f.file.role,
      functions: f.file.functions.slice(0, 30).map((x) => x.name),
    });
  const c = ix.clusters.get(id);
  if (c)
    return node({
      tier: c.tier,
      files: c.cluster.files.slice(0, 30).map((x) => x.path),
    });
  const m = ix.models.get(id);
  if (m)
    return node({ table: m.table, fields: m.fields.map((x) => x.name), framework: m.framework });
  const r = ix.routes.get(id);
  if (r) return node({ method: r.method, path: r.path, framework: r.framework });
  const u = ix.ui.get(id);
  return node({ event: u?.event, file: u?.fileId });
}

function add(ix: Indexed, ctx: Context, id: string): boolean {
  if (ctx.nodes.has(id)) return true;
  if (ctx.nodes.size >= MAX_NODES) return false;
  const n = describe(ix, id);
  if (n) ctx.nodes.set(id, n);
  return n !== null;
}

function addCalls(ix: Indexed, ctx: Context, id: string) {
  for (const c of ix.fns.get(id)?.fn.calls ?? []) {
    if (add(ix, ctx, c.target))
      ctx.edges.push({ from: id, to: c.target, kind: "call", provenance: c.provenance });
  }
}

function addCallers(ix: Indexed, ctx: Context, id: string) {
  for (const c of ix.fns.get(id)?.fn.calledBy ?? []) {
    if (add(ix, ctx, c)) ctx.edges.push({ from: c, to: id, kind: "call" });
  }
}

/** The selection and its direct neighbours: where every explanation starts. */
export function start(ix: Indexed, id: string): Context {
  const ctx: Context = { nodes: new Map(), edges: [], done: new Set() };
  add(ix, ctx, id);
  for (const h of useStore.getState().hops) add(ix, ctx, h.id);
  addCalls(ix, ctx, id);
  addCallers(ix, ctx, id);
  for (const e of [...(ix.crossOut.get(id) ?? []), ...(ix.crossIn.get(id) ?? [])]) {
    add(ix, ctx, e.source);
    add(ix, ctx, e.target);
    ctx.edges.push({ from: e.source, to: e.target, kind: e.protocol, evidence: e.evidence });
  }
  return ctx;
}

const WORDS: Record<Tool, string> = {
  source: "read the code of",
  callers: "who calls",
  calls: "what is called by",
  samefile: "what else is in the file of",
  subsystem: "what else is in the subsystem of",
};

/** The moves open from here: only ones that would show something new. */
export function moves(ix: Indexed, ctx: Context): Move[] {
  const out: Move[] = [];
  const offer = (tool: Tool, n: Node) => {
    const key = `${WORDS[tool]} ${n.label} [${out.length + 1}]`;
    if (!ctx.done.has(`${tool}:${n.id}`)) out.push({ key, tool, id: n.id });
  };
  const sources = [...ctx.done].filter((d) => d.startsWith("source:")).length;
  for (const n of ctx.nodes.values()) {
    if (out.length >= 24) break;
    const fe = ix.fns.get(n.id);
    if (sources < MAX_SOURCES && (fe || ix.files.has(n.id))) offer("source", n);
    if (!fe) continue;
    if (fe.fn.calledBy.some((c) => !ctx.nodes.has(c))) offer("callers", n);
    if (fe.fn.calls.some((c) => !ctx.nodes.has(c.target))) offer("calls", n);
    if (fe.file.functions.some((f) => !ctx.nodes.has(f.id))) offer("samefile", n);
    offer("subsystem", n);
  }
  return out;
}

/** Make one move and add what it shows to the context. */
export async function run(ix: Indexed, ctx: Context, move: Move): Promise<void> {
  ctx.done.add(`${move.tool}:${move.id}`);
  if (move.tool === "callers") addCallers(ix, ctx, move.id);
  if (move.tool === "calls") addCalls(ix, ctx, move.id);
  const fe = ix.fns.get(move.id);
  if (move.tool === "samefile" && fe) for (const f of fe.file.functions) add(ix, ctx, f.id);
  if (move.tool === "subsystem" && fe) {
    const c = ix.clusters.get(fe.clusterId);
    if (c) {
      add(ix, ctx, c.cluster.id);
      ctx.edges.push({ from: fe.file.id, to: c.cluster.id, kind: "part of" });
    }
  }
  if (move.tool === "source") {
    const root = useStore.getState().root;
    const path = fe?.file.path ?? ix.files.get(move.id)?.file.path;
    const node = ctx.nodes.get(move.id);
    if (!root || !path || !node) return;
    const lines = (await readSource(root, path).catch(() => "")).split("\n");
    const [from, to] = fe ? fe.fn.lineRange : [1, 60];
    const code = lines
      .slice(from - 1, to)
      .join("\n")
      .slice(0, SOURCE_CHARS);
    if (code) node.facts = { ...node.facts, source: code };
  }
}
