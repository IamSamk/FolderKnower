// Turns the current focus into React Flow nodes + edges (before layout).

import type { Edge, Node } from "@xyflow/react";
import type { DataModel, FunctionNode, SourceFileNode, SubsystemCluster, TierLabel } from "../gen";
import type { Indexed } from "../lib/graph";
import { PROTOCOL_COLOR, TIER_COLOR, TIER_ORDER } from "../lib/theme";
import type { Focus } from "../store";

export type FlowKind = "http" | "sql" | "call" | "import" | "relation" | "mirror";

export interface FlowEdgeData extends Record<string, unknown> {
  kind: FlowKind;
  color: string;
  label?: string;
  count?: number;
  /** Navigation target when the edge is clicked. */
  to?: string;
}

export interface Built {
  nodes: Node[];
  edges: Edge<FlowEdgeData>[];
  direction: "RIGHT" | "DOWN";
}

const KIND_COLOR: Record<FlowKind, string> = {
  http: PROTOCOL_COLOR["HTTP/REST"],
  sql: PROTOCOL_COLOR.SQL,
  call: "#A1A1AA",
  import: "#52525B",
  relation: "#34D399",
  mirror: "#A78BFA",
};

export const SIZE: Record<string, { w: number; h: number }> = {
  tier: { w: 300, h: 190 },
  cluster: { w: 290, h: 0 },
  file: { w: 260, h: 118 },
  fn: { w: 250, h: 104 },
  ghost: { w: 210, h: 64 },
  portal: { w: 210, h: 74 },
  model: { w: 250, h: 0 },
};

class EdgeBag {
  private m = new Map<string, Edge<FlowEdgeData>>();
  add(source: string, target: string, kind: FlowKind, opts: { label?: string; to?: string } = {}) {
    if (source === target) return;
    const id = `${kind}:${source}->${target}`;
    const e = this.m.get(id);
    if (e?.data) {
      e.data.count = (e.data.count ?? 1) + 1;
      return;
    }
    this.m.set(id, {
      id,
      source,
      target,
      type: "flow",
      data: { kind, color: KIND_COLOR[kind], label: opts.label, count: 1, to: opts.to },
    });
  }
  list(labeler?: (e: Edge<FlowEdgeData>) => string | undefined) {
    const out = [...this.m.values()];
    if (labeler) for (const e of out) if (e.data) e.data.label = labeler(e) ?? e.data.label;
    return out;
  }
}

const proto = (p: string): FlowKind => (p === "SQL" ? "sql" : "http");

function countLabel(e: Edge<FlowEdgeData>): string | undefined {
  const n = e.data?.count ?? 1;
  const k = e.data?.kind;
  if (k === "http") return `${n} API call${n > 1 ? "s" : ""}`;
  if (k === "sql") return `${n} quer${n > 1 ? "ies" : "y"}`;
  if (k === "import") return n > 1 ? `${n} imports` : undefined;
  if (k === "call") return n > 1 ? `${n} calls` : undefined;
  return e.data?.label;
}

export function buildFocus(ix: Indexed, focus: Focus): Built {
  switch (focus.level) {
    case "system":
      return system(ix);
    case "tier":
      return tier(ix, focus.tier);
    case "cluster":
      return cluster(ix, focus.clusterId);
    case "file":
      return file(ix, focus.fileId);
    case "function":
      return fn(ix, focus.fnId);
  }
}

function system(ix: Indexed): Built {
  const nodes: Node[] = [];
  for (const t of TIER_ORDER) {
    const tn = ix.tiers.get(t);
    if (!tn) continue;
    const files = tn.clusters.flatMap((c) => c.files);
    nodes.push({
      id: `tier:${t}`,
      type: "tier",
      position: { x: 0, y: 0 },
      data: {
        tier: t,
        files: files.length,
        fns: files.reduce((a, f) => a + f.functions.length, 0),
        clusters: tn.clusters.length,
        lines: files.reduce((a, f) => a + f.lines, 0),
        top: tn.clusters.slice(0, 4).map((c) => c.name),
        roles: [...new Set(files.map((f) => f.role).filter(Boolean))].slice(0, 5),
      },
    });
  }
  const bag = new EdgeBag();
  for (const e of ix.g.crossTierEdges) {
    const a = ix.tierOf(e.source);
    const b = ix.tierOf(e.target);
    if (a && b && a !== b)
      bag.add(`tier:${a}`, `tier:${b}`, proto(e.protocol), { to: `tier:${b}` });
  }
  for (const [, fe] of ix.files) {
    for (const imp of fe.file.imports) {
      const b = ix.files.get(imp.target)?.tier;
      if (b && b !== fe.tier) bag.add(`tier:${fe.tier}`, `tier:${b}`, "import");
    }
    for (const f of fe.file.functions)
      for (const c of f.calls) {
        const b = ix.tierOf(c.target);
        if (b && b !== fe.tier && c.confidence >= 0.5)
          bag.add(`tier:${fe.tier}`, `tier:${b}`, "call");
      }
  }
  // One line per tier pair: protocol edges (HTTP/SQL) win over calls, calls over imports.
  const all = bag.list(countLabel);
  const rank = (k?: FlowKind) => (k === "http" || k === "sql" ? 2 : k === "call" ? 1 : 0);
  const best = new Map<string, number>();
  for (const e of all) {
    const key = `${e.source}>${e.target}`;
    best.set(key, Math.max(best.get(key) ?? 0, rank(e.data?.kind)));
  }
  const edges = all.filter((e) => rank(e.data?.kind) === best.get(`${e.source}>${e.target}`));
  return { nodes, edges, direction: "DOWN" };
}

function portal(
  id: string,
  targetId: string,
  tier: TierLabel | undefined,
  label: string,
  sub: string,
): Node {
  return {
    id,
    type: "portal",
    position: { x: 0, y: 0 },
    data: { targetId, tier: tier ?? "Shared", label, sub },
  };
}

function tier(ix: Indexed, t: TierLabel): Built {
  const tn = ix.tiers.get(t);
  if (!tn) return { nodes: [], edges: [], direction: "RIGHT" };
  const nodes: Node[] = tn.clusters.map((c) => ({
    id: c.id,
    type: "cluster",
    position: { x: 0, y: 0 },
    data: { cluster: c, tier: t },
  }));
  const bag = new EdgeBag();
  const clusterOfFile = (fid: string) => ix.files.get(fid)?.cluster.id;
  const portals = new Map<string, { tier: TierLabel; n: number }>();
  for (const c of tn.clusters) {
    for (const f of c.files) {
      for (const imp of f.imports) {
        const tc = clusterOfFile(imp.target);
        const tt = ix.files.get(imp.target)?.tier;
        if (tc && tt === t) bag.add(c.id, tc, "import");
        else if (tt) {
          bag.add(c.id, `portal:tier:${tt}`, "import", { to: `tier:${tt}` });
          portals.set(tt, { tier: tt, n: (portals.get(tt)?.n ?? 0) + 1 });
        }
      }
      for (const fnode of f.functions) {
        for (const e of ix.crossOut.get(fnode.id) ?? []) {
          const tt = ix.tierOf(e.target);
          if (!tt || tt === t) continue;
          bag.add(c.id, `portal:tier:${tt}`, proto(e.protocol), { to: `tier:${tt}` });
          portals.set(tt, { tier: tt, n: (portals.get(tt)?.n ?? 0) + 1 });
        }
        for (const e of ix.crossIn.get(fnode.id) ?? []) {
          const st = ix.tierOf(e.source);
          if (!st || st === t) continue;
          bag.add(`portal:tier:${st}`, c.id, proto(e.protocol), { to: `tier:${st}` });
          portals.set(st, { tier: st, n: (portals.get(st)?.n ?? 0) + 1 });
        }
        for (const call of fnode.calls) {
          const tc = clusterOfFile(ix.fileOf(call.target) ?? "");
          if (tc && tc !== c.id && ix.tierOf(call.target) === t && call.confidence >= 0.5)
            bag.add(c.id, tc, "call");
        }
      }
    }
  }
  for (const [pt, p] of portals) {
    nodes.push(
      portal(
        `portal:tier:${pt}`,
        `tier:${pt}`,
        p.tier,
        pt,
        `${p.n} link${p.n > 1 ? "s" : ""} · jump`,
      ),
    );
  }
  return { nodes, edges: bag.list(countLabel), direction: "RIGHT" };
}

function fileNode(f: SourceFileNode, t: TierLabel, c: SubsystemCluster, ghost = false): Node {
  return {
    id: f.id,
    type: "file",
    position: { x: 0, y: 0 },
    data: { file: f, tier: t, cluster: c, ghost },
  };
}

function cluster(ix: Indexed, clusterId: string): Built {
  const ce = ix.clusters.get(clusterId);
  if (!ce) return { nodes: [], edges: [], direction: "RIGHT" };
  const inside = new Set(ce.cluster.files.map((f) => f.id));
  const nodes: Node[] = ce.cluster.files.map((f) => fileNode(f, ce.tier, ce.cluster));
  const bag = new EdgeBag();
  const outside = new Map<string, number>();
  const note = (id: string) => outside.set(id, (outside.get(id) ?? 0) + 1);
  for (const f of ce.cluster.files) {
    for (const imp of f.imports) {
      bag.add(f.id, imp.target, "import");
      if (!inside.has(imp.target)) note(imp.target);
    }
    for (const fnode of f.functions) {
      for (const call of fnode.calls) {
        const tf = ix.fileOf(call.target);
        if (!tf || tf === f.id || call.confidence < 0.5) continue;
        bag.add(f.id, tf, "call");
        if (!inside.has(tf)) note(tf);
      }
      for (const e of ix.crossOut.get(fnode.id) ?? []) {
        const tf = ix.fileOf(e.target);
        if (tf) {
          bag.add(f.id, tf, proto(e.protocol), { to: e.target });
          note(tf);
        }
      }
      for (const e of ix.crossIn.get(fnode.id) ?? []) {
        const sf = ix.fileOf(e.source);
        if (sf) {
          bag.add(sf, f.id, proto(e.protocol), { to: e.source });
          note(sf);
        }
      }
    }
  }
  // Files elsewhere that point into this cluster.
  for (const [, fe] of ix.files) {
    if (inside.has(fe.file.id)) continue;
    for (const imp of fe.file.imports)
      if (inside.has(imp.target)) {
        bag.add(fe.file.id, imp.target, "import");
        note(fe.file.id);
      }
  }
  const top = [...outside.entries()]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .slice(0, 14);
  const keep = new Set([...inside, ...top.map(([id]) => id)]);
  for (const [id] of top) {
    const fe = ix.files.get(id);
    if (fe)
      nodes.push(
        portal(
          id,
          id,
          fe.tier,
          fe.file.path.split("/").pop() ?? id,
          `${fe.tier} · ${fe.cluster.name}`,
        ),
      );
  }
  const edges = bag.list(countLabel).filter((e) => keep.has(e.source) && keep.has(e.target));
  // Drop plain import edges that duplicate a call or protocol edge between the same files.
  const strong = new Set(
    edges.filter((e) => e.data?.kind !== "import").map((e) => `${e.source}>${e.target}`),
  );
  return {
    nodes,
    edges: edges.filter((e) => e.data?.kind !== "import" || !strong.has(`${e.source}>${e.target}`)),
    direction: "RIGHT",
  };
}

function fnNode(
  f: FunctionNode,
  fe: { file: SourceFileNode; tier: TierLabel },
  ghost: boolean,
  center = false,
): Node {
  return {
    id: f.id,
    type: "fn",
    position: { x: 0, y: 0 },
    data: { fn: f, file: fe.file, tier: fe.tier, ghost, center },
  };
}

function modelPortal(m: DataModel): Node {
  return portal(
    m.id,
    m.id,
    "Storage",
    m.name,
    `${m.framework} table${m.table ? ` · ${m.table}` : ""}`,
  );
}

function file(ix: Indexed, fileId: string): Built {
  const fe = ix.files.get(fileId);
  if (!fe) return { nodes: [], edges: [], direction: "RIGHT" };
  const own = new Set(fe.file.functions.map((f) => f.id));
  const nodes: Node[] = fe.file.functions.map((f) => fnNode(f, fe, false));
  const bag = new EdgeBag();
  const extra = new Map<string, Node>();
  const addExternal = (id: string, layer: "FIRST" | "LAST") => {
    if (own.has(id) || extra.has(id) || extra.size > 26) return;
    const x = ix.fns.get(id);
    if (x) {
      const n = fnNode(x.fn, x, true);
      n.data = { ...n.data, layer };
      extra.set(id, n);
      return;
    }
    const m = ix.models.get(id);
    if (m) extra.set(id, modelPortal(m));
  };
  for (const f of fe.file.functions) {
    for (const c of f.calls) {
      if (c.confidence < 0.3) continue;
      addExternal(c.target, "LAST");
      bag.add(f.id, c.target, "call", { to: c.target });
    }
    for (const e of ix.crossOut.get(f.id) ?? []) {
      addExternal(e.target, "LAST");
      bag.add(f.id, e.target, proto(e.protocol), { to: e.target, label: e.evidence[0] });
    }
    for (const e of ix.crossIn.get(f.id) ?? []) {
      addExternal(e.source, "FIRST");
      bag.add(e.source, f.id, proto(e.protocol), { to: e.source, label: e.evidence[0] });
    }
    for (const caller of f.calledBy) {
      if (own.has(caller)) continue;
      addExternal(caller, "FIRST");
      bag.add(caller, f.id, "call", { to: caller });
    }
  }
  nodes.push(...extra.values());
  const ids = new Set(nodes.map((n) => n.id));
  return {
    nodes,
    edges: bag
      .list((e) => (e.data?.kind === "http" ? e.data.label?.replace(/^http\./, "") : countLabel(e)))
      .filter((e) => ids.has(e.source) && ids.has(e.target)),
    direction: "RIGHT",
  };
}

function fn(ix: Indexed, fnId: string): Built {
  const x = ix.fns.get(fnId);
  if (!x) return { nodes: [], edges: [], direction: "RIGHT" };
  const nodes: Node[] = [fnNode(x.fn, x, false, true)];
  const bag = new EdgeBag();
  const seen = new Set([fnId]);
  const add = (id: string, layer: "FIRST" | "LAST") => {
    if (seen.has(id)) return;
    seen.add(id);
    const y = ix.fns.get(id);
    if (y) {
      const n = fnNode(y.fn, y, true);
      n.data = { ...n.data, layer };
      nodes.push(n);
    } else {
      const m = ix.models.get(id);
      if (m) nodes.push(modelPortal(m));
    }
  };
  for (const c of x.fn.calls) {
    add(c.target, "LAST");
    bag.add(fnId, c.target, "call", {
      to: c.target,
      label: c.confidence < 0.6 ? `${c.provenance} ${Math.round(c.confidence * 100)}%` : undefined,
    });
  }
  for (const cb of x.fn.calledBy) {
    add(cb, "FIRST");
    bag.add(cb, fnId, "call", { to: cb });
  }
  for (const e of ix.crossOut.get(fnId) ?? []) {
    add(e.target, "LAST");
    bag.add(fnId, e.target, proto(e.protocol), {
      to: e.target,
      label: e.evidence[0]?.replace(/^http\./, ""),
    });
  }
  for (const e of ix.crossIn.get(fnId) ?? []) {
    add(e.source, "FIRST");
    bag.add(e.source, fnId, proto(e.protocol), {
      to: e.source,
      label: e.evidence[0]?.replace(/^http\./, ""),
    });
  }
  const ids = new Set(nodes.map((n) => n.id));
  return {
    nodes,
    edges: bag.list().filter((e) => ids.has(e.source) && ids.has(e.target)),
    direction: "RIGHT",
  };
}

export function buildEr(ix: Indexed): Built {
  const nodes: Node[] = ix.g.dataModels.map((m) => ({
    id: m.id,
    type: "model",
    position: { x: 0, y: 0 },
    data: { model: m },
  }));
  const bag = new EdgeBag();
  const card: Record<string, string> = {
    many_to_one: "N : 1",
    one_to_many: "1 : N",
    one_to_one: "1 : 1",
    many_to_many: "N : M",
  };
  for (const m of ix.g.dataModels) {
    for (const r of m.relations) {
      // Draw each FK once, from the owning side.
      if (
        r.kind === "one_to_many" &&
        ix.models
          .get(r.target)
          ?.relations.some((x) => x.target === m.id && x.kind === "many_to_one")
      )
        continue;
      bag.add(m.id, r.target, "relation", { label: `${card[r.kind]} · ${r.via}` });
    }
    if (m.kind === "schema")
      for (const t of m.mirrors) bag.add(m.id, t, "mirror", { label: "mirrors" });
  }
  return { nodes, edges: bag.list(), direction: "RIGHT" };
}

export function tierColor(t: TierLabel | undefined): string {
  return t ? TIER_COLOR[t] : "#64748B";
}

export function estimateHeight(n: Node): number {
  if (n.type === "cluster") {
    const c = (n.data as { cluster: SubsystemCluster }).cluster;
    return 86 + Math.min(c.files.length, 8) * 26 + (c.files.length > 8 ? 22 : 0);
  }
  if (n.type === "model") {
    const m = (n.data as { model: DataModel }).model;
    return 58 + Math.min(m.fields.length, 14) * 25 + (m.fields.length > 14 ? 22 : 0);
  }
  if (n.type === "fn" && (n.data as { ghost?: boolean }).ghost) return SIZE.ghost.h;
  return SIZE[n.type ?? "file"]?.h ?? 100;
}

export function estimateWidth(n: Node): number {
  if (n.type === "fn" && (n.data as { ghost?: boolean }).ghost) return SIZE.ghost.w;
  if (n.type === "fn" && (n.data as { center?: boolean }).center) return 320;
  return SIZE[n.type ?? "file"]?.w ?? 240;
}
