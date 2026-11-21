import {
  Background,
  BackgroundVariant,
  Controls,
  type Edge,
  MiniMap,
  type Node,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
} from "@xyflow/react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { TierLabel } from "../gen";
import { layout } from "../lib/layout";
import { TIER_COLOR } from "../lib/theme";
import { useStore } from "../store";
import { type Built, estimateHeight, estimateWidth, type FlowEdgeData } from "./build";
import { edgeTypes } from "./FlowEdge";
import { nodeTypes } from "./nodes";

interface Props {
  built: Built;
  /** Stable key for the current scene; a change triggers the enter animation + fit. */
  sceneKey: string;
  onOpen: (node: Node) => void;
  onNavigate: (targetId: string) => void;
}

function Inner({ built, sceneKey, onOpen, onNavigate }: Props) {
  const rf = useReactFlow();
  const [nodes, setNodes] = useState<Node[]>([]);
  const [edges, setEdges] = useState<Edge[]>([]);
  const selected = useStore((s) => s.selected);
  const select = useStore((s) => s.select);
  const travel = useStore((s) => s.travel);
  const hops = useStore((s) => s.hops);
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
      { direction: built.direction, layerGap: built.direction === "DOWN" ? 90 : 120 },
    ).then((pos) => {
      if (my !== seq.current) return;
      setNodes(ns.map((n) => ({ ...n, position: pos.get(n.id) ?? { x: 0, y: 0 } })));
      setEdges(built.edges);
      requestAnimationFrame(() => {
        void rf.fitView({ duration: 750, padding: 0.18, maxZoom: 1.15 });
      });
    });
  }, [built, rf]);

  // Highlight the neighbourhood of the selection (and the active trace).
  const view = useMemo(() => {
    const hopIds = new Set(hops.map((h) => h.id));
    const focus = selected && nodes.some((n) => n.id === selected) ? selected : null;
    // Only dim when the selection lives in this scene; a trace started elsewhere
    // just highlights the hops that happen to be visible.
    if (!focus) {
      if (hopIds.size === 0) return { nodes, edges };
      return {
        nodes: nodes.map((n) => (hopIds.has(n.id) ? { ...n, data: { ...n.data, hot: true } } : n)),
        edges,
      };
    }
    const keep = new Set<string>();
    if (focus) {
      keep.add(focus);
      for (const e of edges) {
        if (e.source === focus) keep.add(e.target);
        if (e.target === focus) keep.add(e.source);
      }
    }
    for (const id of hopIds) keep.add(id);
    if (keep.size <= 1 && !focus) return { nodes, edges };
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

  const edgeTo = (nodeId: string) =>
    edges.find((e) => e.target === nodeId) ?? edges.find((e) => e.source === nodeId);

  return (
    <ReactFlow
      key={sceneKey}
      nodes={view.nodes}
      edges={view.edges}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      nodesConnectable={false}
      nodesDraggable
      minZoom={0.08}
      maxZoom={2.5}
      proOptions={{ hideAttribution: true }}
      onlyRenderVisibleElements={false}
      onNodeClick={(_, n) => {
        if (n.type === "portal") {
          const e = edgeTo(n.id);
          const target = (n.data as { targetId: string }).targetId;
          if (e) travel(e.id, () => onNavigate(target));
          else onNavigate(target);
          return;
        }
        if (n.type === "fn" && (n.data as { ghost?: boolean }).ghost) {
          const e = edgeTo(n.id);
          if (e) travel(e.id, () => onNavigate(n.id));
          else onNavigate(n.id);
          return;
        }
        select(n.id);
      }}
      onNodeDoubleClick={(_, n) => onOpen(n)}
      onEdgeClick={(_, e) => {
        const d = e.data as FlowEdgeData | undefined;
        if (d?.to) travel(e.id, () => onNavigate(d.to as string));
      }}
      onPaneClick={() => select(null)}
      fitView
    >
      <Background variant={BackgroundVariant.Dots} gap={22} size={1.2} color="#2A3142" />
      <Controls showInteractive={false} position="bottom-right" />
      <MiniMap
        pannable
        zoomable
        position="bottom-left"
        maskColor="rgba(8,10,16,0.72)"
        nodeColor={(n) => {
          const t = (n.data as { tier?: TierLabel }).tier;
          return t ? TIER_COLOR[t] : n.type === "model" ? "#10B981" : "#475569";
        }}
        nodeStrokeWidth={0}
        style={{ background: "rgba(15,18,26,0.85)", width: 160, height: 104 }}
      />
    </ReactFlow>
  );
}

export function Canvas(props: Props) {
  return (
    <ReactFlowProvider>
      <Inner {...props} />
    </ReactFlowProvider>
  );
}
