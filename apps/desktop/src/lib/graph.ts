// In-memory indexes over architecture.json plus the "inspect element" trace.

import type {
  CrossTierEdge,
  DataModel,
  FunctionNode,
  RouteNode,
  SourceFileNode,
  SubsystemCluster,
  SystemArchitectureGraph,
  TierLabel,
  TierNode,
  UiElement,
} from "../gen";

export interface FileEntry {
  file: SourceFileNode;
  tier: TierLabel;
  cluster: SubsystemCluster;
}
export interface FnEntry {
  fn: FunctionNode;
  file: SourceFileNode;
  tier: TierLabel;
  clusterId: string;
}

export interface Indexed {
  g: SystemArchitectureGraph;
  tiers: Map<TierLabel, TierNode>;
  clusters: Map<string, { cluster: SubsystemCluster; tier: TierLabel }>;
  files: Map<string, FileEntry>;
  fns: Map<string, FnEntry>;
  models: Map<string, DataModel>;
  routes: Map<string, RouteNode>;
  routesByHandler: Map<string, RouteNode[]>;
  ui: Map<string, UiElement>;
  uiByComponent: Map<string, UiElement[]>;
  crossOut: Map<string, CrossTierEdge[]>;
  crossIn: Map<string, CrossTierEdge[]>;
  /** Tier for any node id (fn/file/model/route). */
  tierOf: (id: string) => TierLabel | undefined;
  /** File id that contains any node id. */
  fileOf: (id: string) => string | undefined;
  label: (id: string) => string;
}

const push = <K, V>(m: Map<K, V[]>, k: K, v: V) => {
  const a = m.get(k);
  if (a) a.push(v);
  else m.set(k, [v]);
};

export function index(g: SystemArchitectureGraph): Indexed {
  const tiers = new Map<TierLabel, TierNode>();
  const clusters = new Map<string, { cluster: SubsystemCluster; tier: TierLabel }>();
  const files = new Map<string, FileEntry>();
  const fns = new Map<string, FnEntry>();
  for (const t of g.tiers) {
    tiers.set(t.label, t);
    for (const c of t.clusters) {
      clusters.set(c.id, { cluster: c, tier: t.label });
      for (const f of c.files) {
        files.set(f.id, { file: f, tier: t.label, cluster: c });
        for (const fn of f.functions)
          fns.set(fn.id, { fn, file: f, tier: t.label, clusterId: c.id });
      }
    }
  }
  const models = new Map(g.dataModels.map((m) => [m.id, m]));
  const routes = new Map(g.routes.map((r) => [r.id, r]));
  const routesByHandler = new Map<string, RouteNode[]>();
  for (const r of g.routes) if (r.handler) push(routesByHandler, r.handler, r);
  const ui = new Map(g.uiElements.map((u) => [u.id, u]));
  const uiByComponent = new Map<string, UiElement[]>();
  for (const u of g.uiElements) push(uiByComponent, u.component ?? u.fileId, u);
  const crossOut = new Map<string, CrossTierEdge[]>();
  const crossIn = new Map<string, CrossTierEdge[]>();
  for (const e of g.crossTierEdges) {
    push(crossOut, e.source, e);
    push(crossIn, e.target, e);
  }
  const fileOf = (id: string): string | undefined => {
    if (files.has(id)) return id;
    const fe = fns.get(id);
    if (fe) return fe.file.id;
    const m = models.get(id);
    if (m) return m.fileId;
    const r = routes.get(id);
    if (r) return r.fileId;
    const u = ui.get(id);
    if (u) return u.fileId;
    return undefined;
  };
  const tierOf = (id: string): TierLabel | undefined => {
    const f = fileOf(id);
    if (f && files.has(f)) return files.get(f)?.tier;
    if (models.has(id)) return "Storage";
    return undefined;
  };
  const label = (id: string): string => {
    const fe = fns.get(id);
    if (fe) return fe.fn.name;
    const f = files.get(id);
    if (f) return f.file.path.split("/").pop() ?? f.file.path;
    const m = models.get(id);
    if (m) return m.name;
    const r = routes.get(id);
    if (r) return `${r.method} ${r.path}`;
    const u = ui.get(id);
    if (u) return `<${u.tag}>${u.label ? ` ${u.label}` : ""}`;
    if (id.startsWith("fn:")) return id.split("#").pop() ?? id;
    if (id.startsWith("file:")) return id.slice(5).split("/").pop() ?? id;
    return id;
  };
  return {
    g,
    tiers,
    clusters,
    files,
    fns,
    models,
    routes,
    routesByHandler,
    ui,
    uiByComponent,
    crossOut,
    crossIn,
    tierOf,
    fileOf,
    label,
  };
}

export type HopKind = "ui" | "fn" | "route" | "model";
export interface Hop {
  id: string;
  kind: HopKind;
  label: string;
  tier?: TierLabel;
  /** How we got here from the previous hop. */
  via?: "handler" | "call" | "http" | "sql";
  detail?: string;
  fileId?: string;
  line?: number;
}

/**
 * Follow an element (or function) through the system the way a request would travel:
 * UI element -> handler -> calls -> HTTP -> route handler -> calls -> SQL -> table.
 * Breadth-first over static edges; prefers the path that reaches the deepest tier.
 */
export function trace(ix: Indexed, startId: string): Hop[] {
  const hops: Hop[] = [];
  let fnStart: string | undefined = startId;
  const u = ix.ui.get(startId);
  if (u) {
    hops.push({
      id: u.id,
      kind: "ui",
      label: `<${u.tag}>${u.label ? ` “${u.label}”` : ""}`,
      tier: ix.tierOf(u.fileId),
      fileId: u.fileId,
      line: u.line,
      detail: u.event ?? undefined,
    });
    fnStart = u.handler ?? undefined;
    if (!fnStart) return hops;
  }
  if (!fnStart) return hops;
  type Step = { id: string; via?: Hop["via"]; detail?: string };
  const parent = new Map<string, Step & { from?: string }>();
  parent.set(fnStart, {
    id: fnStart,
    via: u ? "handler" : undefined,
    detail: u?.event ?? undefined,
  });
  const queue = [fnStart];
  // A handler that only flips local state (e.g. `mutation.mutate()`) still belongs to a
  // component whose callbacks reach the backend: continue from the component too.
  if (u?.component && u.component !== fnStart && !parent.has(u.component)) {
    parent.set(u.component, { id: u.component, via: "call", from: fnStart, detail: "component" });
    queue.push(u.component);
  }
  const score = (id: string) => {
    if (ix.models.has(id)) return 100;
    const t = ix.tierOf(id);
    return t === "Storage"
      ? 50
      : t === "Backend"
        ? 40
        : t === "Workers"
          ? 35
          : t === "Gateway"
            ? 30
            : 0;
  };
  let best = fnStart;
  let bestDepth = 0;
  const depth = new Map<string, number>([[fnStart, 0]]);
  if (u?.component) depth.set(u.component, 1);
  while (queue.length && parent.size < 400) {
    const id = queue.shift() as string;
    const d = depth.get(id) ?? 0;
    if (score(id) + d > score(best) + bestDepth) {
      best = id;
      bestDepth = d;
    }
    if (d >= 8) continue;
    const next: Step[] = [];
    for (const e of ix.crossOut.get(id) ?? []) {
      next.push({
        id: e.target,
        via: e.protocol === "SQL" ? "sql" : "http",
        detail: e.evidence[0],
      });
    }
    const fe = ix.fns.get(id);
    for (const c of fe?.fn.calls ?? [])
      if (c.confidence >= 0.5) next.push({ id: c.target, via: "call", detail: c.provenance });
    for (const s of next) {
      if (parent.has(s.id)) continue;
      parent.set(s.id, { ...s, from: id });
      depth.set(s.id, d + 1);
      queue.push(s.id);
    }
  }
  const chain: (Step & { from?: string })[] = [];
  let cur: string | undefined = best;
  while (cur) {
    const p = parent.get(cur);
    if (!p) break;
    chain.unshift(p);
    cur = p.from;
  }
  for (const s of chain) {
    const fe = ix.fns.get(s.id);
    const m = ix.models.get(s.id);
    const r = ix.routes.get(s.id);
    if (s.via === "http" && fe) {
      const route = ix.routesByHandler.get(s.id)?.[0];
      if (route)
        hops.push({
          id: route.id,
          kind: "route",
          label: `${route.method} ${route.path}`,
          tier: ix.tierOf(route.fileId),
          via: "http",
          fileId: route.fileId,
          line: route.line,
          detail: route.framework,
        });
    }
    if (fe) {
      hops.push({
        id: s.id,
        kind: "fn",
        label: `${fe.fn.name}()`,
        tier: fe.tier,
        via: s.via === "http" ? "handler" : s.via,
        fileId: fe.file.id,
        line: fe.fn.lineRange[0],
        detail: fe.file.path,
      });
    } else if (m) {
      hops.push({
        id: m.id,
        kind: "model",
        label: m.table ? `${m.name} (${m.table})` : m.name,
        tier: "Storage",
        via: "sql",
        fileId: m.fileId,
        line: m.line,
        detail: m.framework,
      });
    } else if (r) {
      hops.push({
        id: r.id,
        kind: "route",
        label: `${r.method} ${r.path}`,
        via: s.via,
        fileId: r.fileId,
        line: r.line,
      });
    }
  }
  return hops;
}

export function shortPath(p: string, keep = 2): string {
  const parts = p.split("/");
  return parts.length <= keep ? p : `…/${parts.slice(-keep).join("/")}`;
}
