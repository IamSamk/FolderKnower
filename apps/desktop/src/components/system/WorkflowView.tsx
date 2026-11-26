// Draws a Workflow (lib/workflow.ts): phases as framed groups, boxes with a title and one
// line, arrows between them. Arrows are measured from the real boxes so they follow the
// layout at any width. Simple shows titles; Standard adds the plain sentence; Deep adds
// settings and the proof line, which opens the code.

import { useLayoutEffect, useMemo, useRef, useState } from "react";
import type { Depth } from "../../lib/system";
import { proofTarget } from "../../lib/system";
import type { WEdge, WNode, Workflow } from "../../lib/workflow";
import { useStore } from "../../store";

interface Pt {
  x: number;
  y: number;
}

const rel = (r: DOMRect, o: DOMRect) => ({
  l: r.left - o.left,
  r: r.right - o.left,
  t: r.top - o.top,
  b: r.bottom - o.top,
  cx: (r.left + r.right) / 2 - o.left,
  cy: (r.top + r.bottom) / 2 - o.top,
});
type R = ReturnType<typeof rel>;

const hits = (r: R, x1: number, y1: number, x2: number, y2: number) =>
  x2 >= r.l - 2 && x1 <= r.r + 2 && y2 >= r.t - 2 && y1 <= r.b + 2;

/** An elbow between two boxes that goes round the side when a straight run would cross others. */
function route(A: R, B: R, others: R[], lane: number, loop?: boolean): string {
  const side = (from: R, to: R, toTop: boolean) => {
    const x = Math.max(from.r, to.r, ...others.map((o) => o.r)) + 16 + lane * 12;
    return toTop
      ? `M${from.r},${from.cy} H${x} V${to.t - 12} H${to.cx} V${to.t}`
      : `M${from.r},${from.cy} H${x} V${to.cy} H${to.r}`;
  };
  if (loop) return side(A, B, false);
  if (Math.abs(A.cy - B.cy) < 14)
    return B.l >= A.r ? `M${A.r},${A.cy} H${B.l}` : `M${A.l},${A.cy} H${B.r}`;
  if (B.t >= A.b - 2) {
    const my = (A.b + B.t) / 2;
    const blocked = others.some(
      (o) =>
        hits(o, A.cx, A.b, A.cx, my) ||
        hits(o, Math.min(A.cx, B.cx), my, Math.max(A.cx, B.cx), my) ||
        hits(o, B.cx, my, B.cx, B.t),
    );
    return blocked ? side(A, B, true) : `M${A.cx},${A.b} V${my} H${B.cx} V${B.t}`;
  }
  return side(A, B, false);
}

const KIND_CLASS: Record<WNode["kind"], string> = {
  step: "k-step",
  store: "k-store",
  model: "k-model",
  decision: "k-decision",
  join: "k-join",
  job: "k-job",
  limit: "k-limit",
};

/** First sentence, at most 110 characters, for the Standard view. */
function firstSentence(t: string): string {
  let out = "";
  for (const part of t.split(/(?<=[.!?])\s+/)) {
    out = out ? `${out} ${part}` : part;
    if (out.length >= 60) break;
  }
  return out.length > 140 ? `${out.slice(0, 137)}...` : out;
}

const LEGEND: [WNode["kind"], string][] = [
  ["step", "step in the code"],
  ["store", "data store or cache"],
  ["model", "AI model"],
  ["decision", "choice or branch"],
  ["join", "merge of parallel paths"],
  ["job", "background job"],
  ["limit", "limit or budget"],
];

export function WorkflowView({ wf, depth }: { wf: Workflow; depth: Depth }) {
  const ix = useStore((s) => s.ix);
  const reveal = useStore((s) => s.reveal);
  const box = useRef<HTMLDivElement>(null);
  const refs = useRef(new Map<string, HTMLElement>());
  const [paths, setPaths] = useState<{ d: string; e: WEdge }[]>([]);
  const [size, setSize] = useState({ w: 0, h: 0 });

  const edges = useMemo(() => wf.edges, [wf]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: measure after every layout change
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const draw = () => {
      const o = el.getBoundingClientRect();
      const out: { d: string; e: WEdge }[] = [];
      const all = new Map(
        [...refs.current].map(([id, n]) => [id, rel(n.getBoundingClientRect(), o)]),
      );
      let lane = 0;
      for (const e of edges) {
        const a = all.get(e.from);
        const b = all.get(e.to);
        if (!a || !b) continue;
        const others = [...all].filter(([id]) => id !== e.from && id !== e.to).map(([, r]) => r);
        out.push({ d: route(a, b, others, lane % 4, e.loop), e });
        if (e.loop || b.t < a.b) lane++;
      }
      setPaths(out);
      setSize({ w: el.offsetWidth, h: el.offsetHeight });
    };
    draw();
    const ro = new ResizeObserver(draw);
    ro.observe(el);
    return () => ro.disconnect();
  }, [edges, depth, wf]);

  const open = (p: string) => {
    const t = ix ? proofTarget(ix, p) : undefined;
    if (t) reveal(t);
  };

  const node = (n: WNode) => (
    <div
      key={n.id}
      ref={(r) => {
        if (r) refs.current.set(n.id, r);
        else refs.current.delete(n.id);
      }}
      className={`ak-wf-node ${KIND_CLASS[n.kind]}`}
    >
      <div className="ak-wf-title">{n.title}</div>
      {n.sub && <div className="ak-wf-sub">{n.sub}</div>}
      {depth !== "simple" && n.explain && (
        <p className="ak-wf-explain">{depth === "deep" ? n.explain : firstSentence(n.explain)}</p>
      )}
      {depth === "deep" && n.items && n.items.length > 0 && (
        <ul className="ak-wf-items">
          {n.items.slice(0, 8).map((it) => (
            <li key={it}>{it}</li>
          ))}
        </ul>
      )}
      {n.tag && <div className="ak-wf-tag">{n.tag}</div>}
      {depth !== "simple" &&
        n.proof.slice(0, 2).map((p) => (
          <button
            key={p}
            type="button"
            className="ak-ev ak-mono ak-ev-link"
            onClick={() => open(p)}
          >
            {p}
          </button>
        ))}
    </div>
  );

  return (
    <div className="ak-wf" ref={box}>
      {wf.phases.map((ph) => (
        <section key={ph.id} className="ak-wf-phase">
          <h3 className="ak-wf-phase-title">{ph.title}</h3>
          {ph.rows.map((r, i) => (
            <div
              // biome-ignore lint/suspicious/noArrayIndexKey: rows are positional
              key={i}
              className={`ak-wf-row ${r.group ? "grouped" : ""}`}
            >
              {r.group && <div className="ak-wf-group">{r.group}</div>}
              <div className="ak-wf-row-nodes">{r.nodes.map(node)}</div>
            </div>
          ))}
        </section>
      ))}
      <div className="ak-wf-legend">
        <b>Key</b>
        {LEGEND.map(([k, t]) => (
          <span key={k}>
            <i className={`ak-wf-node ${KIND_CLASS[k]}`} /> {t}
          </span>
        ))}
        <span>
          solid arrow: runs next · dashed: optional or async · every box links to its code
        </span>
      </div>
      <svg className="ak-wf-arrows" width={size.w} height={size.h} aria-hidden="true">
        <defs>
          <marker
            id="wfarrow"
            viewBox="0 0 10 10"
            refX="9"
            refY="5"
            markerWidth="7"
            markerHeight="7"
            orient="auto"
          >
            <path d="M0,0 L10,5 L0,10 z" fill="currentColor" />
          </marker>
        </defs>
        {paths.map(({ d, e }) => (
          <g key={`${e.from}>${e.to}`} className={e.dashed ? "dashed" : ""}>
            <path d={d} markerEnd="url(#wfarrow)" />
          </g>
        ))}
      </svg>
    </div>
  );
}
