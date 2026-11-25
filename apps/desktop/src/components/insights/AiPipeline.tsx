// The recognized AI system drawn as a story in lanes: ingest once, retrieve per
// question, generate and decide, and what is inside the model. Every card is a real
// call in the code; "Play a request" walks a question through the stages in order.

import { ArrowRight, Pause, Play, X } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useMemo, useState } from "react";
import type { AiArchitecture, AiRole, AiStage } from "../../gen";
import {
  LANE_TITLE,
  PARAM_LABEL,
  ROLE_COLOR,
  ROLE_GLYPH,
  ROLE_LABEL,
  ROLE_LANE,
} from "../../lib/insights";
import { useStore } from "../../store";

const CARD_W = 236;
const CARD_H = 86;
const COL_GAP = 84;
const ROW_GAP = 22;
const TOP = 46;
const ROLE_ORDER = Object.keys(ROLE_LANE) as AiRole[];

interface Placed {
  s: AiStage;
  x: number;
  y: number;
}

function layout(ai: AiArchitecture) {
  const lanes = [0, 1, 2, 3].filter((l) => ai.stages.some((s) => ROLE_LANE[s.role] === l));
  const placed = new Map<string, Placed>();
  let height = 0;
  lanes.forEach((lane, col) => {
    const inLane = ai.stages
      .filter((s) => ROLE_LANE[s.role] === lane)
      .sort((a, b) => ROLE_ORDER.indexOf(a.role) - ROLE_ORDER.indexOf(b.role) || a.line - b.line);
    inLane.forEach((s, row) => {
      const y = TOP + row * (CARD_H + ROW_GAP);
      placed.set(s.id, { s, x: col * (CARD_W + COL_GAP), y });
      height = Math.max(height, y + CARD_H);
    });
  });
  return { lanes, placed, width: lanes.length * (CARD_W + COL_GAP) - COL_GAP, height: height + 24 };
}

/** Path between two cards: right side to left side across lanes, bottom to top inside one. */
function edgePath(a: Placed, b: Placed, loop: boolean): string {
  if (a.x === b.x) {
    const down = b.y > a.y;
    const x1 = a.x + CARD_W - 28;
    const y1 = down ? a.y + CARD_H : a.y;
    const y2 = down ? b.y : b.y + CARD_H;
    if (Math.abs(b.y - a.y) <= CARD_H + ROW_GAP) return `M${x1},${y1} L${x1},${y2}`;
    const bend = a.x + CARD_W + 26;
    return `M${a.x + CARD_W},${a.y + CARD_H / 2} C${bend},${a.y + CARD_H / 2} ${bend},${b.y + CARD_H / 2} ${b.x + CARD_W},${b.y + CARD_H / 2}`;
  }
  const fwd = b.x > a.x;
  const x1 = fwd ? a.x + CARD_W : a.x;
  const x2 = fwd ? b.x : b.x + CARD_W;
  const y1 = a.y + CARD_H / 2;
  const y2 = b.y + CARD_H / 2;
  const dx = Math.max(40, Math.abs(x2 - x1) / 2);
  if (loop) {
    const lift = Math.min(y1, y2) - 34;
    return `M${x1},${y1} C${x1 + (fwd ? dx : -dx)},${lift} ${x2 - (fwd ? dx : -dx)},${lift} ${x2},${y2}`;
  }
  return `M${x1},${y1} C${x1 + (fwd ? dx : -dx)},${y1} ${x2 - (fwd ? dx : -dx)},${y2} ${x2},${y2}`;
}

/** Order in which a question meets the stages: edges first, loops last, lanes as tie-break. */
function tour(ai: AiArchitecture): string[] {
  const indeg = new Map(ai.stages.map((s) => [s.id, 0]));
  const out = new Map<string, string[]>();
  for (const e of ai.edges) {
    if (e.loopBack) continue;
    indeg.set(e.to, (indeg.get(e.to) ?? 0) + 1);
    out.set(e.from, [...(out.get(e.from) ?? []), e.to]);
  }
  const linked = new Set(ai.edges.flatMap((e) => [e.from, e.to]));
  const rank = (id: string) => {
    const s = ai.stages.find((x) => x.id === id);
    return s ? ROLE_LANE[s.role] * 100 + ROLE_ORDER.indexOf(s.role) : 999;
  };
  const left = new Set(ai.stages.filter((s) => linked.has(s.id)).map((s) => s.id));
  const order: string[] = [];
  while (left.size) {
    const ready = [...left].filter((id) => (indeg.get(id) ?? 0) <= 0);
    // A cycle without a loop-back flag: break it at the earliest stage in the story.
    const pick = (ready.length ? ready : [...left]).sort((a, b) => rank(a) - rank(b))[0];
    left.delete(pick);
    order.push(pick);
    for (const t of out.get(pick) ?? []) indeg.set(t, (indeg.get(t) ?? 1) - 1);
  }
  return order;
}

function paramText(k: string, v: string) {
  return `${PARAM_LABEL[k] ?? k.replace(/_/g, " ")} ${v.length > 26 ? `${v.slice(0, 25)}…` : v}`;
}

function StageCard({
  p,
  on,
  dim,
  onClick,
}: {
  p: Placed;
  on: boolean;
  dim: boolean;
  onClick: () => void;
}) {
  const params = Object.entries(p.s.params)
    .filter(([k]) => k !== "search_kwargs")
    .slice(0, 2);
  return (
    <motion.button
      type="button"
      className={`ak-ai-card ${on ? "on" : ""}`}
      style={
        {
          left: p.x,
          top: p.y,
          width: CARD_W,
          height: CARD_H,
          "--c": ROLE_COLOR[p.s.role],
        } as React.CSSProperties
      }
      animate={{ opacity: dim ? 0.38 : 1, scale: on ? 1.04 : 1 }}
      transition={{ type: "spring", stiffness: 420, damping: 30 }}
      onClick={onClick}
    >
      <span className="ak-ai-glyph">{ROLE_GLYPH[p.s.role]}</span>
      <span className="min-w-0 flex-1 text-left">
        <span className="ak-ai-role">{ROLE_LABEL[p.s.role]}</span>
        <span className="ak-ai-name ak-mono">{p.s.name}</span>
        <span className="ak-ai-params">
          {params.length ? (
            params.map(([k, v]) => (
              <span key={k} className="ak-ai-param">
                {paramText(k, v)}
              </span>
            ))
          ) : (
            <span className="ak-muted">{p.s.library}</span>
          )}
        </span>
      </span>
    </motion.button>
  );
}

function StageDetail({ s, onClose }: { s: AiStage; onClose: () => void }) {
  const reveal = useStore((st) => st.reveal);
  const entries = Object.entries(s.params);
  return (
    <motion.aside
      className="ak-ai-detail"
      initial={{ opacity: 0, x: 24 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: 24 }}
      transition={{ type: "spring", stiffness: 380, damping: 34 }}
      style={{ "--c": ROLE_COLOR[s.role] } as React.CSSProperties}
    >
      <div className="flex items-start gap-2">
        <span className="ak-ai-glyph lg">{ROLE_GLYPH[s.role]}</span>
        <div className="min-w-0 flex-1">
          <div className="ak-ai-role">{ROLE_LABEL[s.role]}</div>
          <div className="ak-mono truncate text-[15px] font-bold">{s.name}</div>
          <div className="ak-muted text-[12px]">{s.library}</div>
        </div>
        <button type="button" className="ak-iconbtn" onClick={onClose} title="Close">
          <X size={15} />
        </button>
      </div>
      <p className="mt-3 text-[13.5px] leading-relaxed">{s.explain}</p>
      {entries.length > 0 && (
        <>
          <div className="ak-section-title mt-4">Settings found in the code</div>
          <table className="ak-kv">
            <tbody>
              {entries.map(([k, v]) => (
                <tr key={k}>
                  <th>{PARAM_LABEL[k] ?? k.replace(/_/g, " ")}</th>
                  <td className="ak-mono">{v}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
      <button
        type="button"
        className="ak-btn mt-4 w-full"
        onClick={() => reveal(s.functionId ?? s.fileId)}
      >
        Show me the code, line {s.line} <ArrowRight size={14} />
      </button>
      <div className="ak-muted ak-mono mt-2 text-[10.5px]">rule {s.ruleId}</div>
    </motion.aside>
  );
}

/** Literal numbers a student asks about first, pulled from every stage. */
function KeyNumbers({ ai }: { ai: AiArchitecture }) {
  const get = (role: AiRole, ...keys: string[]) => {
    for (const s of ai.stages.filter((x) => x.role === role))
      for (const k of keys) if (s.params[k]) return s.params[k];
    return undefined;
  };
  const size = Number(get("splitter", "chunk_size", "chunkSize"));
  const overlap = Number(get("splitter", "chunk_overlap", "chunkOverlap") ?? 0);
  const [pages, setPages] = useState(300);
  const chars = pages * 1800;
  const chunks =
    size > overlap ? Math.max(1, Math.ceil((chars - overlap) / (size - overlap))) : null;
  const items: [string, string | undefined][] = [
    ["chunk size", Number.isFinite(size) ? `${size} chars` : undefined],
    ["overlap", overlap ? `${overlap} chars` : undefined],
    ["embedding model", get("embedder", "model_name", "model")],
    [
      "top-k per retriever",
      get("retriever", "k", "top_k", "similarity_top_k") ?? get("keywordIndex", "k"),
    ],
    ["fusion weights", get("fusion", "weights")],
    ["reranker", get("reranker", "model", "model_name")],
    ["keep top-n", get("reranker", "top_n")],
    ["language model", get("llm", "model", "model_name")],
    ["context window", get("llm", "num_ctx", "n_ctx", "max_model_len")],
  ];
  const shown = items.filter(([, v]) => v);
  if (!shown.length) return null;
  return (
    <div className="ak-ai-numbers">
      {shown.map(([k, v]) => (
        <div key={k} className="ak-ai-num">
          <span className="ak-muted text-[11px]">{k}</span>
          <span className="ak-mono text-[12.5px] font-semibold">{v}</span>
        </div>
      ))}
      {chunks && (
        <div className="ak-ai-num wide">
          <span className="ak-muted text-[11px]">
            how many chunks? for a{" "}
            <input
              className="ak-ai-pages"
              type="number"
              min={1}
              max={20000}
              value={pages}
              onChange={(e) => setPages(Math.max(1, Number(e.target.value) || 1))}
            />{" "}
            page document (~1,800 characters a page)
          </span>
          <span className="ak-mono text-[12.5px] font-semibold">
            ≈ {chunks.toLocaleString()} chunks, each one embedded once
          </span>
        </div>
      )}
    </div>
  );
}

export function AiPipeline({ ai }: { ai: AiArchitecture }) {
  const { lanes, placed, width, height } = useMemo(() => layout(ai), [ai]);
  const steps = useMemo(() => tour(ai), [ai]);
  const [sel, setSel] = useState<string | null>(null);
  const [step, setStep] = useState(-1);
  const playing = step >= 0;
  useEffect(() => {
    if (step < 0) return;
    const t = setTimeout(() => setStep((i) => (i + 1 < steps.length ? i + 1 : -1)), 1500);
    return () => clearTimeout(t);
  }, [step, steps.length]);
  const active = playing ? steps[step] : null;
  const prev = playing && step > 0 ? steps[step - 1] : null;
  const activeEdge = ai.edges.find((e) => e.from === prev && e.to === active);
  const selected = sel ? ai.stages.find((s) => s.id === sel) : undefined;
  const activeStage = active ? placed.get(active)?.s : undefined;
  return (
    <div className="ak-ai">
      <div className="ak-ai-head">
        <div>
          <div className="ak-headline">{ai.headline}</div>
          <div className="mt-1 flex flex-wrap gap-1.5">
            {ai.frameworks.map((f) => (
              <span key={f} className="ak-chip">
                {f}
              </span>
            ))}
          </div>
        </div>
        <button
          type="button"
          className="ak-btn primary ml-auto shrink-0 whitespace-nowrap"
          onClick={() => setStep(playing ? -1 : 0)}
        >
          {playing ? <Pause size={15} /> : <Play size={15} />}
          {playing ? "Stop" : "Play a request"}
        </button>
      </div>
      <KeyNumbers ai={ai} />
      <div className="ak-ai-scroll">
        <div className="ak-ai-canvas" style={{ width, height }}>
          {lanes.map((l, col) => (
            <div
              key={l}
              className="ak-ai-lane"
              style={{ left: col * (CARD_W + COL_GAP) - 14, width: CARD_W + 28, height }}
            >
              <span>{LANE_TITLE[l]}</span>
            </div>
          ))}
          <svg className="ak-ai-edges" width={width} height={height} aria-hidden>
            <defs>
              <marker
                id="ak-ai-arrow"
                viewBox="0 0 10 10"
                refX="9"
                refY="5"
                markerWidth="7"
                markerHeight="7"
                orient="auto-start-reverse"
              >
                <path d="M0,0 L10,5 L0,10 z" fill="var(--ink)" />
              </marker>
            </defs>
            {ai.edges.map((e) => {
              const a = placed.get(e.from);
              const b = placed.get(e.to);
              if (!a || !b) return null;
              const d = edgePath(a, b, Boolean(e.loopBack));
              const on = activeEdge === e;
              return (
                <g
                  key={`${e.from}>${e.to}`}
                  className={`ak-ai-edge ${e.loopBack ? "loop" : ""} ${on ? "on" : ""}`}
                >
                  <path d={d} markerEnd="url(#ak-ai-arrow)" />
                </g>
              );
            })}
          </svg>
          {activeEdge &&
            (() => {
              const a = placed.get(activeEdge.from);
              const b = placed.get(activeEdge.to);
              if (!a || !b) return null;
              const d = edgePath(a, b, Boolean(activeEdge.loopBack));
              return (
                <motion.div
                  key={`${activeEdge.from}>${activeEdge.to}`}
                  className="ak-ai-packet"
                  style={{ offsetPath: `path("${d}")` }}
                  initial={{ offsetDistance: "0%" }}
                  animate={{ offsetDistance: "100%" }}
                  transition={{ duration: 0.9, ease: [0.65, 0, 0.35, 1] }}
                />
              );
            })()}
          {ai.edges.map((e) => {
            const a = placed.get(e.from);
            const b = placed.get(e.to);
            if (!a || !b || a.x === b.x || !e.label) return null;
            const x = (Math.min(a.x, b.x) + CARD_W + Math.max(a.x, b.x)) / 2;
            const y = (a.y + b.y) / 2 + CARD_H / 2 - (e.loopBack ? 40 : 0);
            return (
              <span key={`l${e.from}>${e.to}`} className="ak-ai-elabel" style={{ left: x, top: y }}>
                {e.label}
              </span>
            );
          })}
          {[...placed.values()].map((p) => (
            <StageCard
              key={p.s.id}
              p={p}
              on={active === p.s.id || sel === p.s.id}
              dim={playing && active !== p.s.id && !steps.slice(0, step).includes(p.s.id)}
              onClick={() => setSel(p.s.id)}
            />
          ))}
        </div>
      </div>
      <AnimatePresence>
        {activeStage && (
          <motion.div
            key={activeStage.id}
            className="ak-ai-caption"
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -8 }}
          >
            <span className="ak-dsa-step">
              {step + 1}/{steps.length}
            </span>
            <b>{ROLE_LABEL[activeStage.role]}</b>
            {activeEdge?.label && <span className="ak-muted"> receives {activeEdge.label}. </span>}
            <span> {activeStage.explain}</span>
          </motion.div>
        )}
      </AnimatePresence>
      <AnimatePresence>
        {selected && <StageDetail key={selected.id} s={selected} onClose={() => setSel(null)} />}
      </AnimatePresence>
    </div>
  );
}
