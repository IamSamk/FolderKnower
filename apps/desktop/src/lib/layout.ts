// ELK layered layout. Deterministic for the same input order.
import ELK from "elkjs/lib/elk.bundled.js";

const elk = new ELK();

export interface LNode {
  id: string;
  width: number;
  height: number;
  /** Optional layer constraint: "FIRST" | "LAST" keeps callers left, callees right. */
  layer?: "FIRST" | "LAST";
}
export interface LEdge {
  id: string;
  source: string;
  target: string;
}

export async function layout(
  nodes: LNode[],
  edges: LEdge[],
  opts: { direction?: "RIGHT" | "DOWN"; nodeGap?: number; layerGap?: number } = {},
): Promise<Map<string, { x: number; y: number }>> {
  const ids = new Set(nodes.map((n) => n.id));
  const graph = {
    id: "root",
    layoutOptions: {
      "elk.algorithm": "layered",
      "elk.direction": opts.direction ?? "RIGHT",
      "elk.spacing.nodeNode": String(opts.nodeGap ?? 40),
      "elk.layered.spacing.nodeNodeBetweenLayers": String(opts.layerGap ?? 110),
      "elk.layered.nodePlacement.strategy": "NETWORK_SIMPLEX",
      "elk.layered.crossingMinimization.strategy": "LAYER_SWEEP",
      "elk.layered.considerModelOrder.strategy": "NODES_AND_EDGES",
      "elk.separateConnectedComponents": "true",
      "elk.spacing.componentComponent": "70",
      "elk.randomSeed": "1",
    },
    children: nodes.map((n) => ({
      id: n.id,
      width: n.width,
      height: n.height,
      layoutOptions: n.layer ? { "elk.layered.layering.layerConstraint": n.layer } : undefined,
    })),
    edges: edges
      .filter((e) => ids.has(e.source) && ids.has(e.target) && e.source !== e.target)
      .map((e) => ({ id: e.id, sources: [e.source], targets: [e.target] })),
  };
  const res = await elk.layout(graph);
  const out = new Map<string, { x: number; y: number }>();
  for (const c of res.children ?? []) out.set(c.id, { x: c.x ?? 0, y: c.y ?? 0 });
  return out;
}
