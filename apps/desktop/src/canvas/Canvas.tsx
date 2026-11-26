import {
  Background,
  BackgroundVariant,
  type Edge,
  MiniMap,
  type Node,
  Panel,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  useViewport,
} from "@xyflow/react";
import { Maximize2, Minus, Plus } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { TierLabel } from "../gen";
import { layout } from "../lib/layout";
import { TIER_COLOR } from "../lib/theme";
import { useTheme } from "../lib/themeMode";
import { useStore } from "../store";
import { type Built, estimateHeight, estimateWidth, type FlowEdgeData } from "./build";
import { edgeTypes } from "./FlowEdge";
import { nodeTypes } from "./nodes";

export interface NavOrigin {
  x: number;
  y: number;
}

interface Props {
  built: Built;
  /** Open (zoom into) a node. `el` is the node's DOM element, for the lens. */
  onOpen: (node: Node, el: Element | null) => void;
  /** Go to another id. `origin` is where on screen the journey ended. */
  onNavigate: (targetId: string, origin?: NavOrigin) => void;
}

const ease = (t: number) => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2);

function ZoomBar() {
  const rf = useReactFlow();
  const { zoom } = useViewport();
  return (
    <Panel position="bottom-right" className="ak-zoombar">
      <button
        type="button"
        aria-label="Zoom out"
        onClick={() => void rf.zoomOut({ duration: 300 })}
      >
        <Minus size={15} strokeWidth={2.4} />
      </button>
      <span className="ak-zoomval">{Math.round(zoom * 100)}%</span>
      <button type="button" aria-label="Zoom in" onClick={() => void rf.zoomIn({ duration: 300 })}>
        <Plus size={15} strokeWidth={2.4} />
      </button>
      <button
        type="button"
        aria-label="Fit to screen"
        onClick={() => void rf.fitView({ duration: 600, padding: 0.18 })}
      >
        <Maximize2 size={14} strokeWidth={2.4} />
      </button>
    </Panel>
  );
}

function Inner({ built, onOpen, onNavigate }: Props) {
  const rf = useReactFlow();
  const [nodes, setNodes] = useState<Node[]>([]);
  const [edges, setEdges] = useState<Edge[]>([]);
  const selected = useStore((s) => s.selected);
  const select = useStore((s) => s.select);
  const travel = useStore((s) => s.travel);
  const hops = useStore((s) => s.hops);
  const dark = useTheme((s) => s.resolved === "dark");
  const box = useRef<HTMLDivElement>(null);
  const seq = useRef(0);

  useEffect(() => {
    const my = ++seq.current;
    const ns = built.nodes.map((n, i) => ({
      ...n,
      data: { ...n.data, _i: i, dir: built.direction },
      width: estimateWidth(n),
      height: estimateHeight(n),
    }));
    void layout(
      ns.map((n) => ({
        id: n.id,
        width: n.width ?? 240,
        height: n.height ?? 100,
        layer: (n.data as { layer?: "FIRST" | "LAST" }).layer,
      })),
      built.edges.map((e) => ({ id: e.id, source: e.source, target: e.target })),
      { direction: built.direction, layerGap: built.direction === "DOWN" ? 96 : 130 },
    ).then((pos) => {
      if (my !== seq.current) return;
      setNodes(ns.map((n) => ({ ...n, position: pos.get(n.id) ?? { x: 0, y: 0 } })));
      setEdges(built.edges);
      requestAnimationFrame(() => {
        void rf.fitView({ duration: 0, padding: 0.2, maxZoom: 1.1 }).then(() => {
          // A big diagram fits at a size nobody can read: open on the busiest node and
          // its neighbours instead, and leave the full picture one click away (Fit).
          if (rf.getZoom() >= 0.4) return;
          const deg = new Map<string, number>();
          for (const e of built.edges) {
            deg.set(e.source, (deg.get(e.source) ?? 0) + 1);
            deg.set(e.target, (deg.get(e.target) ?? 0) + 1);
          }
          const hub = [...deg.entries()].sort((a, b) => b[1] - a[1])[0]?.[0] ?? ns[0]?.id;
          if (!hub) return;
          const near = new Set([hub]);
          for (const e of built.edges) {
            if (e.source === hub) near.add(e.target);
            if (e.target === hub) near.add(e.source);
          }
          void rf.fitView({
            nodes: [...near].map((id) => ({ id })),
            duration: 0,
            padding: 0.25,
            minZoom: 0.45,
            maxZoom: 0.9,
          });
        });
      });
    });
  }, [built, rf]);

  const view = useMemo(() => {
    const hopIds = new Set(hops.map((h) => h.id));
    const focus = selected && nodes.some((n) => n.id === selected) ? selected : null;
    if (!focus) {
      if (hopIds.size === 0) return { nodes, edges };
      return {
        nodes: nodes.map((n) => (hopIds.has(n.id) ? { ...n, data: { ...n.data, hot: true } } : n)),
        edges,
      };
    }
    const keep = new Set<string>([focus]);
    for (const e of edges) {
      if (e.source === focus) keep.add(e.target);
      if (e.target === focus) keep.add(e.source);
    }
    for (const id of hopIds) keep.add(id);
    return {
      nodes: nodes.map((n) => ({
        ...n,
        data: { ...n.data, dim: !keep.has(n.id), hot: hopIds.has(n.id) },
      })),
      edges: edges.map((e) => ({
        ...e,
        data: { ...e.data, dim: !(keep.has(e.source) && keep.has(e.target)) },
      })),
    };
  }, [nodes, edges, selected, hops]);

  /** Fly the camera along an edge (following the travelling dot), then navigate. */
  const journey = (edge: Edge, target: string, reverse: boolean) => {
    const path = box.current?.querySelector<SVGPathElement>(`path[id="${CSS.escape(edge.id)}"]`);
    const zoom = Math.max(rf.getZoom(), 0.85);
    const end = () => {
      const r = box.current?.getBoundingClientRect();
      onNavigate(target, r ? { x: r.width / 2, y: r.height / 2 } : undefined);
    };
    if (!path) {
      travel(edge.id, end);
      return;
    }
    const len = path.getTotalLength();
    const t0 = performance.now();
    const dur = 720;
    travel(edge.id, end);
    const step = (now: number) => {
      const t = Math.min((now - t0) / dur, 1);
      const k = ease(t);
      const p = path.getPointAtLength(len * (reverse ? 1 - k : k));
      void rf.setCenter(p.x, p.y, { zoom: zoom + 0.25 * Math.sin(Math.PI * t) });
      if (t < 1) requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  };

  const edgeTo = (nodeId: string) =>
    edges.find((e) => e.target === nodeId) ?? edges.find((e) => e.source === nodeId);

  return (
    <div ref={box} className="h-full w-full">
      <ReactFlow
        nodes={view.nodes}
        edges={view.edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        nodesConnectable={false}
        nodesDraggable
        minZoom={0.08}
        maxZoom={2.5}
        zoomOnDoubleClick={false}
        proOptions={{ hideAttribution: true }}
        onNodeClick={(_, n) => {
          const isPortal =
            n.type === "portal" || (n.type === "fn" && (n.data as { ghost?: boolean }).ghost);
          if (isPortal) {
            const target = n.type === "portal" ? (n.data as { targetId: string }).targetId : n.id;
            const e = edgeTo(n.id);
            if (e) journey(e, target, e.target !== n.id);
            else onNavigate(target);
            return;
          }
          select(n.id);
        }}
        onNodeDoubleClick={(ev, n) =>
          onOpen(n, (ev.target as Element).closest(".react-flow__node"))
        }
        onEdgeClick={(_, e) => {
          const d = e.data as FlowEdgeData | undefined;
          if (d?.to) journey(e, d.to, d.to === e.source);
        }}
        onPaneClick={() => select(null)}
      >
        <Background
          variant={BackgroundVariant.Dots}
          gap={24}
          size={1.6}
          color={dark ? "#3a3760" : "#c3c6d9"}
        />
        <ZoomBar />
        <MiniMap
          pannable
          zoomable
          position="bottom-left"
          className="ak-minimap"
          bgColor={dark ? "#26244a" : "#ffffff"}
          maskColor={dark ? "rgba(20,19,38,0.7)" : "rgba(238,240,247,0.7)"}
          nodeColor={(n) => {
            const t = (n.data as { tier?: TierLabel }).tier;
            return t ? TIER_COLOR[t] : n.type === "model" ? TIER_COLOR.Storage : "#b9bccc";
          }}
          nodeStrokeWidth={0}
        />
      </ReactFlow>
    </div>
  );
}

export function Canvas(props: Props) {
  return (
    <ReactFlowProvider>
      <Inner {...props} />
    </ReactFlowProvider>
  );
}
