// Workflow diagrams: boxes in phases with arrows, like a hand-drawn architecture sheet,
// built only from what the analysis found. Every box carries the `path:line` that proves
// it; a box that has no proof is not created. Three kinds:
//   request  one feature's example run, grouped by the container it runs in
//   ai       the AI pipeline: index once, then answer (parallel searches, fusion, rerank)
//   context  what is stored, what goes into each model call and how it is bounded

import type { AiStage, ExampleRun, Feature, MemoryPlacement } from "../gen";
import type { Indexed } from "./graph";
import { LIFE_NAME, PLACE_NAME } from "./system";

export type WKind = "step" | "store" | "model" | "decision" | "join" | "job" | "limit";

export interface WNode {
  id: string;
  title: string;
  /** One line under the title. */
  sub?: string;
  /** Plain explanation (Standard and Deep). */
  explain?: string;
  /** Extra rows (Deep): settings, items of a parallel list. */
  items?: string[];
  /** Right-hand note, e.g. how often a slot changes. */
  tag?: string;
  kind: WKind;
  proof: string[];
}

export interface WRow {
  /** A dashed frame around parallel nodes, with its title. */
  group?: string;
  nodes: WNode[];
}

export interface WPhase {
  id: string;
  title: string;
  rows: WRow[];
}

export interface WEdge {
  from: string;
  to: string;
  label?: string;
  dashed?: boolean;
  loop?: boolean;
}

export interface Workflow {
  id: string;
  kind: WorkflowKind;
  title: string;
  phases: WPhase[];
  edges: WEdge[];
}

export type WorkflowKind = "request" | "ai" | "context";

export const WORKFLOW_KINDS: { id: WorkflowKind; label: string; hint: string }[] = [
  { id: "request", label: "Request", hint: "One action, step by step, container by container" },
  { id: "ai", label: "AI pipeline", hint: "How the AI parts are wired: index once, then answer" },
  {
    id: "context",
    label: "Memory and context",
    hint: "What is stored and what each model call sees",
  },
];

const proofOf = (ix: Indexed, s: AiStage): string[] => {
  const p = ix.files.get(s.fileId)?.file.path ?? s.fileId.replace(/^file:/, "");
  return [`${p}:${s.line}`];
};

/** Up to `n` settings as "k=8" pairs, model first. */
function settings(s: AiStage, n = 3): string[] {
  const order = ["model", "model_name", "k", "top_k", "top_n", "chunk_size", "chunk_overlap"];
  const keys = Object.keys(s.params).sort((a, b) => {
    const ia = order.indexOf(a);
    const ib = order.indexOf(b);
    return (ia < 0 ? 99 : ia) - (ib < 0 ? 99 : ib) || a.localeCompare(b);
  });
  return keys.slice(0, n).map((k) => `${k}=${s.params[k]}`);
}

function stageNode(ix: Indexed, s: AiStage, kind: WKind, title?: string): WNode {
  const set = settings(s);
  return {
    id: s.id,
    title: title ?? s.name,
    sub: set.length ? set.join(" · ") : s.library,
    explain: s.explain,
    items: [s.library, ...Object.entries(s.params).map(([k, v]) => `${k} = ${v}`)],
    kind,
    proof: proofOf(ix, s),
  };
}

const by = (stages: AiStage[], ...roles: AiStage["role"][]) =>
  stages.filter((s) => roles.includes(s.role));

/* ------------------------------------------------------------------ AI pipeline */

export function aiWorkflow(ix: Indexed, feature?: Feature, run?: ExampleRun): Workflow | null {
  const ai = ix.g.insights?.ai;
  if (!ai) return null;
  let stages = ai.stages;
  if (feature) {
    const mine = new Set(feature.aiStages);
    const sub = stages.filter((s) => mine.has(s.id));
    if (sub.length) stages = sub;
  }
  const edges: WEdge[] = [];
  const link = (a?: WNode, b?: WNode, label?: string, extra?: Partial<WEdge>) => {
    if (a && b) edges.push({ from: a.id, to: b.id, label, ...extra });
  };
  const phases: WPhase[] = [];

  // ---- offline
  const loader = by(stages, "loader")[0];
  const splitter = by(stages, "splitter")[0];
  const embedder = by(stages, "embedder")[0];
  const vstore = by(stages, "vectorStore")[0];
  const kw = by(stages, "keywordIndex");
  if (loader || splitter || vstore) {
    const row1: WNode[] = [];
    const nl = loader && stageNode(ix, loader, "step", `Load: ${loader.name}`);
    const ns = splitter && stageNode(ix, splitter, "step", `Chunk: ${splitter.name}`);
    const ne = embedder && stageNode(ix, embedder, "model", `Embed: ${embedder.name}`);
    const nv = vstore && stageNode(ix, vstore, "store", `Store vectors: ${vstore.name}`);
    for (const n of [nl, ns, ne, nv]) if (n) row1.push(n);
    link(nl, ns, "documents");
    link(ns ?? nl, ne, "chunks");
    link(ne ?? ns ?? nl, nv, "vectors");
    const rows: WRow[] = [{ nodes: row1 }];
    const kwIdx = kw.map((k) => stageNode(ix, k, "store", `Keyword index: ${k.name}`));
    if (kwIdx.length) {
      rows.push({ nodes: kwIdx });
      link(ns ?? nl ?? ne, kwIdx[0], "text");
    }
    phases.push({ id: "offline", title: "1. Offline: ingest and index", rows });
  }

  // ---- online
  const online: WRow[] = [];
  const start: WNode = {
    id: "q",
    title: run?.sampleInput ? `Question: "${run.sampleInput}"` : "A question arrives",
    sub: run ? run.title : undefined,
    kind: "step",
    proof: run?.hops[0]?.proof ?? [],
  };
  const planner = by(stages, "agent", "router")[0];
  const nplan = planner && stageNode(ix, planner, "decision", `Decide: ${planner.name}`);
  online.push({ nodes: nplan ? [start, nplan] : [start] });
  link(start, nplan, "question");

  const lists: WNode[] = [];
  const retr = by(stages, "retriever");
  const dense = retr[0];
  if (dense || (embedder && vstore)) {
    const items = [
      embedder ? `Embed the question with ${embedder.params.model ?? embedder.name}` : "",
      vstore ? `Nearest chunks in ${vstore.name}` : "",
      dense ? `${dense.name}${dense.params.k ? ` (top ${dense.params.k})` : ""}` : "",
    ].filter(Boolean);
    const src = dense ?? vstore ?? embedder;
    if (src)
      lists.push({
        id: "list:dense",
        title: "Meaning (dense)",
        sub: vstore ? `reads: ${vstore.name}` : undefined,
        items,
        explain: "Finds chunks whose meaning is close to the question.",
        kind: "step",
        proof: proofOf(ix, src),
      });
  }
  for (const k of kw)
    lists.push({
      id: `list:${k.id}`,
      title: `Words (${k.name})`,
      sub: k.library,
      items: [k.explain],
      explain: "Finds chunks that contain the question's words; good for names and codes.",
      kind: "step",
      proof: proofOf(ix, k),
    });
  for (const r of retr.slice(dense ? 1 : 0))
    lists.push({
      id: `list:${r.id}`,
      title: r.name,
      sub: settings(r).join(" · ") || r.library,
      explain: r.explain,
      kind: "step",
      proof: proofOf(ix, r),
    });
  if (lists.length)
    online.push({
      group: `Search: ${lists.length} ranked list${lists.length > 1 ? "s" : ""} run together`,
      nodes: lists,
    });
  const searchHead = lists[0];
  if (searchHead) for (const l of lists) link(nplan ?? start, l, undefined);

  const fusion = by(stages, "fusion")[0];
  const rerank = by(stages, "reranker")[0];
  const prompt = by(stages, "prompt")[0];
  const llm = by(stages, "llm")[0];
  const tool = by(stages, "tool")[0];
  const nf = fusion && stageNode(ix, fusion, "join", `Fuse: ${fusion.name}`);
  const nr = rerank && stageNode(ix, rerank, "model", `Rerank: ${rerank.name}`);
  const np = prompt && stageNode(ix, prompt, "step", `Prompt: ${prompt.name}`);
  const nl2 = llm && stageNode(ix, llm, "model", `Answer: ${llm.name}`);
  const nt = tool && stageNode(ix, tool, "step", `Tool: ${tool.name}`);
  for (const n of [nf, nr, np, nl2]) if (n) online.push({ nodes: [n] });
  if (nt) online.push({ nodes: [nt] });
  // chain
  let prev: WNode | undefined;
  const afterSearch = [nf, nr, np, nl2].filter(Boolean) as WNode[];
  for (const n of afterSearch) {
    if (!prev) for (const l of lists) link(l, n);
    else link(prev, n);
    prev = n;
  }
  if (!prev && !lists.length) link(nplan ?? start, nl2 ?? np);
  if (nplan && nl2) link(nl2, nplan, "search again", { loop: true, dashed: true });
  if (nt && nplan) link(nplan, nt, "call tool", { dashed: true });
  const reply: WNode = {
    id: "answer",
    title: "Answer",
    sub: run?.hops.at(-1)?.explain,
    kind: "step",
    proof: run?.hops.at(-1)?.proof ?? [],
  };
  online.push({ nodes: [reply] });
  link(prev ?? nplan ?? start, reply);
  phases.push({
    id: "online",
    title: phases.length ? "2. Online: answering" : "Answering",
    rows: online,
  });
  return {
    id: `ai${feature ? `:${feature.id}` : ""}`,
    kind: "ai",
    title: ai.headline,
    phases,
    edges,
  };
}

/* ------------------------------------------------------------------ request flow */

export function requestWorkflow(ix: Indexed, run: ExampleRun | undefined): Workflow | null {
  if (!run || run.hops.length === 0) return null;
  const phases: WPhase[] = [];
  const edges: WEdge[] = [];
  const services = new Map((ix.g.insights?.deployment?.services ?? []).map((s) => [s.id, s]));
  let cur: WPhase | undefined;
  let curKey = "";
  let prev: WNode | undefined;
  run.hops.forEach((h, i) => {
    const key = h.service ?? (h.kind === "ui" || h.kind === "response" ? "screen" : "code");
    if (!cur || key !== curKey) {
      curKey = key;
      const svc = h.service ? services.get(h.service) : undefined;
      cur = {
        id: `${key}:${phases.length}`,
        title: svc ? svc.name : key === "screen" ? "Screen" : "Code",
        rows: [],
      };
      phases.push(cur);
    }
    const n: WNode = {
      id: `hop${i}`,
      title: h.label,
      explain: h.explain,
      sub: h.via,
      kind: h.kind === "data" ? "store" : h.kind === "stage" ? "model" : "step",
      proof: h.proof,
    };
    cur.rows.push({ nodes: [n] });
    if (prev) edges.push({ from: prev.id, to: n.id, dashed: h.kind === "response" });
    prev = n;
  });
  return { id: `req:${run.id}`, kind: "request", title: run.title, phases, edges };
}

/* ------------------------------------------------------------------ memory and context */

const SLOT_ROLES: [AiStage["role"], string, string][] = [
  ["prompt", "Instructions and tools", "fixed text in the code"],
  ["memory", "Conversation memory", "each turn"],
  ["retriever", "Retrieved evidence", "every question"],
  ["tool", "Tool results", "every call"],
];

export function contextWorkflow(ix: Indexed): Workflow | null {
  const mem = ix.g.memoryMap ?? [];
  const ai = ix.g.insights?.ai;
  if (!mem.length && !ai) return null;
  const phases: WPhase[] = [];
  const edges: WEdge[] = [];

  // what is stored
  const stored: WNode[] = mem.slice(0, 12).map((p) => memNode(p));
  if (stored.length)
    phases.push({
      id: "stored",
      title: "1. What is kept, and where",
      rows: chunk(stored, 4).map((nodes) => ({ nodes })),
    });

  // each model call
  const stages = ai?.stages ?? [];
  const slots: WNode[] = [];
  for (const [role, title, change] of SLOT_ROLES) {
    const s = by(stages, role)[0];
    if (!s) continue;
    slots.push({
      ...stageNode(ix, s, "step", title),
      sub: `${s.name}${settings(s, 2).length ? ` · ${settings(s, 2).join(" · ")}` : ""}`,
      tag: change,
    });
  }
  const llm = by(stages, "llm")[0];
  const ctxKey =
    llm && ["num_ctx", "n_ctx", "max_model_len", "max_tokens"].find((k) => llm.params[k]);
  const limit: WNode | undefined = llm
    ? {
        id: "limit",
        title: ctxKey ? `Context budget: ${llm.params[ctxKey]} tokens` : "Model call",
        sub: llm.params.model ?? llm.name,
        explain:
          "Everything above must fit in the model's context window. What does not fit is cut, summarized or left out.",
        kind: "limit",
        proof: proofOf(ix, llm),
      }
    : undefined;
  if (slots.length || limit) {
    const rows: WRow[] = [];
    if (slots.length) rows.push({ group: "Inside every model call, top to bottom", nodes: slots });
    if (limit) rows.push({ nodes: [limit] });
    phases.push({ id: "call", title: "2. What one model call sees", rows });
    for (const sl of slots) if (limit) edges.push({ from: sl.id, to: limit.id });
  }

  // caches
  const caches: WNode[] = [
    ...by(stages, "cache").map((c) => stageNode(ix, c, "job", c.name)),
    ...(ix.g.insights?.memoryTechniques ?? []).map((m) => ({
      id: m.id,
      title: m.name,
      sub: m.setting,
      explain: m.explain,
      kind: "job" as WKind,
      proof: [`${ix.files.get(m.fileId)?.file.path ?? m.fileId.replace(/^file:/, "")}:${m.line}`],
    })),
  ];
  const used = new Set(caches.flatMap((c) => c.proof));
  // a placement already described by a technique at the same line is the same fact
  const kv = mem.filter((p) => /kv|cache/i.test(p.what) && !p.proof.some((x) => used.has(x)));
  for (const p of kv) caches.push(memNode(p, "job"));
  const seen = new Set<string>();
  const uniq = caches.filter((c) => (seen.has(c.id) ? false : (seen.add(c.id), true)));
  if (uniq.length) {
    phases.push({
      id: "caches",
      title: "3. Reuse: caches and the KV cache",
      rows: chunk(uniq.slice(0, 8), 4).map((nodes) => ({ nodes })),
    });
    if (limit)
      for (const c of uniq.slice(0, 8)) edges.push({ from: limit.id, to: c.id, dashed: true });
  }
  const rules = (ix.g.rules ?? []).slice(0, 8);
  if (rules.length) {
    phases.push({
      id: "rules",
      title: "4. Limits written in the code",
      rows: chunk(
        rules.map((r) => ({
          id: `rule:${r.proof}`,
          title: `${r.name} = ${r.value}`,
          sub: r.meaning,
          kind: "limit" as WKind,
          proof: [r.proof],
        })),
        4,
      ).map((nodes) => ({ nodes })),
    });
  }
  return { id: "context", kind: "context", title: "Memory and context", phases, edges };
}

function memNode(p: MemoryPlacement, kind: WKind = "store"): WNode {
  return {
    id: `mem:${p.id}`,
    title: p.what,
    sub: `${PLACE_NAME[p.place]} · ${LIFE_NAME[p.lifetime]}${p.bound ? ` · ${p.bound}` : ""}`,
    explain: p.explain,
    items: [`held by ${p.holderName}`],
    kind,
    proof: p.proof,
  };
}

function chunk<T>(a: T[], n: number): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < a.length; i += n) out.push(a.slice(i, i + n));
  return out;
}

/** Which kinds can be drawn for this project. */
export function availableKinds(ix: Indexed, run?: ExampleRun): WorkflowKind[] {
  const out: WorkflowKind[] = [];
  if (run) out.push("request");
  if (ix.g.insights?.ai) out.push("ai");
  if ((ix.g.memoryMap?.length ?? 0) > 0 || ix.g.insights?.ai) out.push("context");
  return out;
}
