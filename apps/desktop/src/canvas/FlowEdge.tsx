import { BaseEdge, EdgeLabelRenderer, type EdgeProps, getBezierPath } from "@xyflow/react";
import { useStore } from "../store";
import type { FlowEdgeData } from "./build";

export function FlowEdge(props: EdgeProps) {
  const { id, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition, selected } =
    props;
  const data = props.data as FlowEdgeData | undefined;
  const travelling = useStore((s) => s.travelling === id);
  const [path, lx, ly] = getBezierPath({
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    curvature: 0.32,
  });
  const kind = data?.kind ?? "call";
  const color = data?.color ?? "#71717A";
  const dim = (data as { dim?: boolean } | undefined)?.dim;
  const strong = kind === "http" || kind === "sql";
  const width = Math.min(1.4 + Math.log2((data?.count ?? 1) + 1) * 0.9, 5) + (strong ? 0.6 : 0);
  return (
    <>
      <BaseEdge
        id={id}
        path={path}
        className={`ak-edge ak-edge-${kind} ${selected ? "ak-edge-selected" : ""}`}
        style={{
          stroke: color,
          strokeWidth: width,
          opacity: dim ? 0.12 : kind === "import" ? 0.45 : 0.85,
        }}
        interactionWidth={18}
      />
      {strong && !dim && (
        <path
          d={path}
          className="ak-edge-flow"
          style={{ stroke: color, strokeWidth: Math.max(width - 0.6, 1.2) }}
          fill="none"
        />
      )}
      {travelling && (
        <g>
          <circle r={9} fill={color} opacity={0.25}>
            <animateMotion
              dur="0.7s"
              repeatCount="1"
              path={path}
              fill="freeze"
              keyPoints="0;1"
              keyTimes="0;1"
              calcMode="spline"
              keySplines="0.4 0 0.2 1"
            />
          </circle>
          <circle r={4.5} fill="#fff">
            <animateMotion
              dur="0.7s"
              repeatCount="1"
              path={path}
              fill="freeze"
              keyPoints="0;1"
              keyTimes="0;1"
              calcMode="spline"
              keySplines="0.4 0 0.2 1"
            />
          </circle>
        </g>
      )}
      {data?.label && !dim && (
        <EdgeLabelRenderer>
          <div
            className="ak-edge-label nodrag nopan"
            style={{
              transform: `translate(-50%, -50%) translate(${lx}px, ${ly}px)`,
              borderColor: color,
              color,
            }}
          >
            {data.label}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  );
}

export const edgeTypes = { flow: FlowEdge };
