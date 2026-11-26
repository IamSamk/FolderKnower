// "System" view: the running system as containers, the code inside each one, how
// memory is split between them, and an example run that follows one request from the
// screen through the proxy, the backend, the AI pipeline and the data stores and back.
// Every box, link and hop comes from the static analysis (compose files, Dockerfiles,
// resolved calls, routes); hops show the line that proves them.

import {
  ChevronLeft,
  ChevronRight,
  Cpu,
  Expand,
  MemoryStick,
  Pause,
  Play,
  RotateCcw,
  X,
} from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { ExampleRun, RunHop } from "../../gen";
import { fmtBytes } from "../../lib/insights";
import {
  type Box,
  buildSystem,
  CARD_W,
  type Depth,
  HEAD_H,
  hopAnchor,
  hopService,
  LANE_NAME,
  LIFE_NAME,
  linkLabel,
  linkPath,
  linkPorts,
  PAD,
  PLACE_COLOR,
  PLACE_NAME,
  PLAIN_NAME,
  PLAIN_WHAT,
  PREVIEW_H,
  plainHeadline,
  proofTarget,
  ROW_H,
  SECTION_H,
  SVC_LABEL,
  type SystemModel,
} from "../../lib/system";
import { useStore } from "../../store";
import { MemoryMap } from "./MemoryMap";

function readPref<T extends string>(key: string, ok: readonly T[], fallback: T): T {
  try {
    const v = localStorage.getItem(key) as T | null;
    return v && ok.includes(v) ? v : fallback;
  } catch {
    return fallback;
  }
}

function writePref(key: string, v: string) {
  try {
    localStorage.setItem(key, v);
  } catch {
    // private window or blocked storage: the choice just is not remembered
  }
}

const HOP_MS = 1900;
const VIA_TEXT: Record<RunHop["via"], string> = {
  start: "starts",
  handler: "event handler",
  call: "function call",
  http: "HTTP request",
  proxy: "forwarded by the proxy",
  import: "imported module",
  pipeline: "AI pipeline step",
  sql: "database query",
  network: "network call to a container",
  return: "response",
};

function useSize(ref: React.RefObject<HTMLDivElement | null>) {
  const [s, setS] = useState({ w: 900, h: 600 });
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setS({ w: el.clientWidth, h: el.clientHeight }));
    ro.observe(el);
    setS({ w: el.clientWidth, h: el.clientHeight });
    return () => ro.disconnect();
  }, [ref]);
  return s;
}

/** The screen the user sees, drawn from the UI elements found in the start file. */
function Preview({ run, step }: { run: ExampleRun; step: number }) {
  const ix = useStore((s) => s.ix);
  const start = ix?.ui.get(run.start);
  const els = [...(ix?.ui.values() ?? [])]
    .filter((u) => u.fileId === start?.fileId)
    .sort((a, b) => a.line - b.line)
    .slice(0, 4);
  const hop = run.hops[step];
  const typing = hop?.kind === "ui" && hop.via === "start";
  const [typed, setTyped] = useState(0);
  const text = run.sampleInput ?? "";
  useEffect(() => {
    if (!typing) {
      setTyped(step < 0 ? 0 : text.length);
      return;
    }
    setTyped(0);
    let i = 0;
    const t = setInterval(
      () => {
        i += 1;
        setTyped(i);
        if (i >= text.length) clearInterval(t);
      },
      Math.max(28, (HOP_MS * 0.7) / Math.max(1, text.length)),
    );
    return () => clearInterval(t);
  }, [typing, text, step]);
  const done = hop?.kind === "response";
  return (
    <div className="ak-sys-preview">
      <div className="ak-sys-chrome">
        <i />
        <i />
        <i />
        <span className="ak-mono">{ix?.label(start?.fileId ?? "") ?? "app"}</span>
      </div>
      <div className="ak-sys-screen">
        {els.map((u) =>
          u.tag === "input" || u.tag === "textarea" ? (
            <div key={u.id} className={`ak-sys-input ${typing ? "on" : ""}`}>
              {typed > 0 ? text.slice(0, typed) : <span className="ak-muted">{u.label}</span>}
              {typing && <b className="ak-caret" />}
            </div>
          ) : (
            <motion.div
              key={u.id}
              className={`ak-sys-button ${hop?.node === u.id && hop.kind === "ui" ? "on" : ""}`}
              animate={{ scale: hop?.node === u.id && hop.kind === "ui" ? [1, 0.9, 1] : 1 }}
            >
              {u.label ?? `<${u.tag}>`}
            </motion.div>
          ),
        )}
        <AnimatePresence>
          {done && (
            <motion.div
              className="ak-sys-answer"
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
            >
              answer appears here
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}

function BoxCard({
  b,
  active,
  activeRow,
  run,
  step,
  depth,
  onOpen,
  onFeature,
}: {
  b: Box;
  active: boolean;
  activeRow?: string;
  run?: ExampleRun;
  step: number;
  depth: Depth;
  onOpen: () => void;
  onFeature: (id: string) => void;
}) {
  const reveal = useStore((s) => s.reveal);
  const s = b.svc;
  return (
    <motion.div
      className={`ak-sys-box ${active ? "on" : ""} ${b.dim ? "dim" : ""}`}
      style={{ left: b.x, top: b.y, width: b.w, height: b.h, ["--c" as string]: b.color }}
      animate={{ scale: active ? 1.025 : 1, opacity: b.dim ? 0.28 : 1 }}
      transition={{ type: "spring", stiffness: 300, damping: 26 }}
    >
      {depth === "simple" ? (
        <button type="button" className="ak-sys-head" onClick={onOpen} style={{ height: HEAD_H }}>
          <b className="text-[16px]">{PLAIN_NAME[s.kind]}</b>
          <div className="ak-sys-plain">{PLAIN_WHAT[s.kind]}</div>
        </button>
      ) : (
        <button type="button" className="ak-sys-head" onClick={onOpen} style={{ height: HEAD_H }}>
          <div className="flex items-baseline gap-2">
            <b className="text-[15.5px]">{s.name}</b>
            <span className="ak-sys-kind">{SVC_LABEL[s.kind]}</span>
          </div>
          <div className="ak-mono ak-muted mt-0.5 truncate text-[11.5px]">
            {s.image ?? (b.synthetic ? "code tier" : "built here")}
            {s.ports.length > 0 && ` · :${s.ports.map((p) => p.split(":").pop()).join(", :")}`}
          </div>
          <div className="mt-1.5 flex flex-wrap gap-1">
            {s.memoryLimit ? (
              <span className="ak-chip">
                <MemoryStick size={11} /> {fmtBytes(s.memoryLimit)}
              </span>
            ) : !b.synthetic ? (
              <span className="ak-chip ak-chip-dim">no memory limit</span>
            ) : null}
            {s.gpu && (
              <span className="ak-chip ak-chip-gpu">
                <Cpu size={11} /> GPU
              </span>
            )}
            {s.codeFiles > 0 && (
              <span className="ak-chip">
                {s.codeFiles} file{s.codeFiles > 1 ? "s" : ""}
              </span>
            )}
          </div>
        </button>
      )}
      {b.preview && run && (
        <div style={{ height: PREVIEW_H }} className="ak-sys-preview-wrap">
          <Preview run={run} step={step} />
        </div>
      )}
      {b.code.length > 0 && (
        <div className="ak-sys-section" style={{ height: SECTION_H }}>
          code inside
        </div>
      )}
      {b.code.map((r) => (
        <button
          key={r.key}
          type="button"
          className={`ak-sys-row ${activeRow === r.key ? "on" : ""}`}
          style={{ height: ROW_H }}
          onClick={() => reveal(r.ref)}
          title="Open in the code view"
        >
          <span className="ak-mono truncate">{r.text}</span>
          {r.badge && (
            <span className="ak-sys-badge" style={{ ["--b" as string]: r.color }}>
              {r.badge}
            </span>
          )}
        </button>
      ))}
      {b.pipeline.length > 0 && (
        <div className="ak-sys-section" style={{ height: SECTION_H }}>
          {run ? "AI steps for this request" : b.dim ? "AI steps" : "AI steps here"}
        </div>
      )}
      {b.pipeline.map((r, i) => (
        <div
          key={r.key}
          className={`ak-sys-row ak-sys-step ${activeRow === r.key ? "on" : ""}`}
          style={{ height: ROW_H }}
        >
          <span className="ak-sys-n">{i + 1}</span>
          <span className="truncate">{r.text}</span>
        </div>
      ))}
      {depth === "simple" && b.features.length > 0 && (
        <>
          <div className="ak-sys-section" style={{ height: SECTION_H }}>
            helps with
          </div>
          <div className="ak-sys-chips">
            {b.features.map((f) => (
              <button key={f.id} type="button" className="ak-chip" onClick={() => onFeature(f.id)}>
                {f.name}
              </button>
            ))}
          </div>
        </>
      )}
      {depth === "deep" && b.stack.length > 0 && (
        <>
          <div className="ak-sys-section" style={{ height: SECTION_H }}>
            tech stack
          </div>
          <div className="ak-sys-chips">
            {b.stack.map((p) => (
              <span key={p} className="ak-chip ak-mono">
                {p}
              </span>
            ))}
          </div>
        </>
      )}
      {depth === "deep" && b.places.length > 0 && (
        <>
          <div className="ak-sys-section" style={{ height: SECTION_H }}>
            data kept here
          </div>
          {b.places.map((p) => (
            <div
              key={p.id}
              className="ak-sys-row"
              style={{ height: ROW_H }}
              title={`${p.explain}${p.bound ? ` (${p.bound})` : ""}`}
            >
              <span className="ak-sys-badge" style={{ ["--b" as string]: PLACE_COLOR[p.place] }}>
                {PLACE_NAME[p.place]}
              </span>
              <span className="truncate">{p.what}</span>
              <span className="ak-muted ml-auto shrink-0 text-[11px]">{LIFE_NAME[p.lifetime]}</span>
            </div>
          ))}
        </>
      )}
    </motion.div>
  );
}

function MemorySplit({ m }: { m: SystemModel }) {
  const limited = m.boxes.filter((b) => b.svc.memoryLimit);
  const unlimited = m.boxes.filter((b) => !b.svc.memoryLimit && !b.synthetic);
  if (!limited.length && !unlimited.length) return null;
  return (
    <div className="ak-sys-mem">
      <div className="ak-sys-mem-title">
        <MemoryStick size={14} /> Memory split{" "}
        <span className="ak-muted">
          {limited.length ? `${fmtBytes(m.totalLimit)} capped by compose limits` : "no limits set"}
        </span>
      </div>
      <div className="ak-sys-mem-bar">
        {limited.map((b) => (
          <span
            key={b.svc.id}
            style={{
              width: `${((b.svc.memoryLimit ?? 0) / m.totalLimit) * 100}%`,
              background: b.color,
            }}
            title={`${b.svc.name}: ${fmtBytes(b.svc.memoryLimit)}`}
          >
            {(b.svc.memoryLimit ?? 0) / m.totalLimit > 0.09 ? b.svc.name : ""}
          </span>
        ))}
      </div>
      <div className="ak-sys-mem-legend">
        {limited.map((b) => (
          <span key={b.svc.id}>
            <i style={{ background: b.color }} />
            {b.svc.name} {fmtBytes(b.svc.memoryLimit)}
            {b.svc.gpu ? " + GPU" : ""}
          </span>
        ))}
        {unlimited.length > 0 && (
          <span className="ak-muted">
            no limit (may use all free RAM): {unlimited.map((b) => b.svc.name).join(", ")}
          </span>
        )}
      </div>
    </div>
  );
}

function ServicePanel({ b, onClose }: { b: Box; onClose: () => void }) {
  const reveal = useStore((s) => s.reveal);
  const s = b.svc;
  return (
    <motion.aside
      className="ak-sys-panel ak-card"
      initial={{ opacity: 0, x: 24 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: 24 }}
    >
      <div className="flex items-start gap-2">
        <div>
          <div className="text-[17px] font-bold">{s.name}</div>
          <div className="ak-muted text-[12px]">{SVC_LABEL[s.kind]}</div>
        </div>
        <button type="button" className="ak-iconbtn ml-auto" onClick={onClose} aria-label="Close">
          <X size={15} />
        </button>
      </div>
      <p className="mt-2 text-[13px] leading-relaxed">{s.explain}</p>
      <dl className="ak-sys-dl">
        {s.image && (
          <>
            <dt>image</dt>
            <dd className="ak-mono">{s.image}</dd>
          </>
        )}
        {s.command && (
          <>
            <dt>runs</dt>
            <dd className="ak-mono">{s.command}</dd>
          </>
        )}
        {s.entryFile && (
          <>
            <dt>entry</dt>
            <dd>
              <button
                type="button"
                className="ak-link-btn"
                onClick={() => reveal(s.entryFile ?? "")}
              >
                {s.entryFile.replace("file:", "")}
              </button>
            </dd>
          </>
        )}
        {(s.memoryLimit || s.memoryReservation || s.cpuLimit) && (
          <>
            <dt>limits</dt>
            <dd>
              {s.memoryLimit ? `memory ${fmtBytes(s.memoryLimit)}` : ""}
              {s.memoryReservation ? `, reserved ${fmtBytes(s.memoryReservation)}` : ""}
              {s.cpuLimit ? `, ${s.cpuLimit} CPU` : ""}
            </dd>
          </>
        )}
        {s.volumes.length > 0 && (
          <>
            <dt>volumes</dt>
            <dd className="ak-mono">{s.volumes.join("\n")}</dd>
          </>
        )}
        {s.envKeys.length > 0 && (
          <>
            <dt>env</dt>
            <dd className="ak-mono">{s.envKeys.join(", ")}</dd>
          </>
        )}
        {s.declaredIn && (
          <>
            <dt>declared</dt>
            <dd className="ak-mono">{s.declaredIn}</dd>
          </>
        )}
      </dl>
      {b.memory.length > 0 && (
        <>
          <div className="ak-section-title mt-3">Memory handled in this code</div>
          {b.memory.map((m) => (
            <div key={m.id} className="mt-1.5 text-[12.5px]">
              <b>{m.name}</b>{" "}
              <button type="button" className="ak-link-btn" onClick={() => reveal(m.fileId)}>
                {m.fileId.replace("file:", "")}:{m.line}
              </button>
              <div className="ak-muted">{m.explain}</div>
            </div>
          ))}
        </>
      )}
    </motion.aside>
  );
}

const DEPTHS = ["simple", "standard", "deep"] as const;
const DEPTH_HINT: Record<Depth, string> = {
  simple: "Plain words: what each part is for, no code",
  standard: "Containers, the code inside them and the AI steps",
  deep: "Every file, the packages each part uses and the data it keeps",
};
const MAPS = ["services", "memory"] as const;
type MapKind = (typeof MAPS)[number];

export function SystemView() {
  const ix = useStore((s) => s.ix);
  const reveal = useStore((s) => s.reveal);
  const features = ix?.g.features ?? [];
  const [depth, setDepthRaw] = useState<Depth>(() => readPref("ak.sys.depth", DEPTHS, "standard"));
  const [map, setMapRaw] = useState<MapKind>(() => readPref("ak.sys.map", MAPS, "services"));
  const [featureId, setFeatureId] = useState<string>("");
  const feature = features.find((f) => f.id === featureId);
  const setDepth = (d: Depth) => {
    setDepthRaw(d);
    writePref("ak.sys.depth", d);
  };
  const setMap = (v: MapKind) => {
    setMapRaw(v);
    writePref("ak.sys.map", v);
  };
  const allRuns = ix?.g.exampleRuns ?? [];
  const runs = feature ? allRuns.filter((r) => feature.ui.includes(r.start)) : allRuns;
  const systemRun = useStore((s) => s.systemRun);
  const [runId, setRunId] = useState(systemRun ?? runs[0]?.id);
  const run = runs.find((r) => r.id === runId) ?? (feature ? runs[0] : undefined);
  const [step, setStep] = useState(-1);
  const [playing, setPlaying] = useState(false);
  const [open, setOpen] = useState<string | null>(null);
  const m = useMemo(
    () => (ix ? buildSystem(ix, run, { depth, feature }) : null),
    [ix, run, depth, feature],
  );
  const vp = useRef<HTMLDivElement>(null);
  const size = useSize(vp);
  const hop = run && step >= 0 ? run.hops[step] : undefined;
  const anchor = ix && m && hop ? hopAnchor(ix, m, hop) : undefined;

  useEffect(() => {
    if (!playing || !run) return;
    if (step >= run.hops.length - 1) {
      const t = setTimeout(() => setPlaying(false), HOP_MS);
      return () => clearTimeout(t);
    }
    const t = setTimeout(() => setStep((s) => s + 1), step < 0 ? 300 : HOP_MS);
    return () => clearTimeout(t);
  }, [playing, step, run]);

  // Asked to play a run from another view (UI map, Inspector): start it once.
  useEffect(() => {
    if (!systemRun) return;
    setRunId(systemRun);
    setFeatureId("");
    setMapRaw("services");
    setStep(-1);
    setPlaying(true);
    useStore.setState({ systemRun: null });
  }, [systemRun]);

  const play = useCallback(() => {
    if (!run) return;
    if (step >= run.hops.length - 1) setStep(-1);
    setPlaying(true);
  }, [run, step]);

  useEffect(() => {
    const k = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement)?.tagName === "INPUT") return;
      if (e.key === " ") {
        e.preventDefault();
        playing ? setPlaying(false) : play();
      }
      if (e.key === "ArrowRight" && run) setStep((s) => Math.min(run.hops.length - 1, s + 1));
      if (e.key === "ArrowLeft") setStep((s) => Math.max(-1, s - 1));
      if (e.key === "Escape") setOpen(null);
    };
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  }, [play, playing, run]);

  if (!ix || !m)
    return (
      <div className="ak-empty">
        <h2>No running system found</h2>
        <p>No compose file, Dockerfile or backend code was recognized.</p>
      </div>
    );

  // Camera: whole system when idle, follow the packet while a run plays.
  const fit = Math.min(1, (size.w - 40) / m.width, (size.h - 40) / m.height);
  const zoom = hop ? Math.max(fit, Math.min(1.15, fit * 1.6)) : fit;
  const cx = anchor ? anchor.x : m.width / 2;
  const cy = anchor ? anchor.y : m.height / 2;
  const tx = size.w / 2 - cx * zoom;
  const ty = size.h / 2 - cy * zoom;
  const activeSvc = hop ? hopService(ix, hop, m.synthetic) : undefined;
  const activeRow = anchor?.row?.key;
  const lanes = [...new Set(m.boxes.map((b) => b.lane))].sort();
  const prevAnchor = run && step > 0 ? hopAnchor(ix, m, run.hops[step - 1]) : undefined;
  const travelling = prevAnchor && anchor && prevAnchor.box !== anchor.box;
  const opened = open ? m.byId.get(open) : undefined;
  const ports = linkPorts(m);
  const prevSvc = run && step > 0 ? hopService(ix, run.hops[step - 1], m.synthetic) : undefined;

  return (
    <div className="ak-sys">
      <div className="ak-sys-top">
        <div className="min-w-0">
          <div className="ak-sys-headline">
            {depth === "simple" ? plainHeadline(ix, m.boxes.length) : ix.g.summary.headline}
          </div>
          {m.synthetic && (
            <div className="ak-muted text-[12px]">
              No docker-compose file found: boxes are code tiers. Add a compose file to see real
              containers, limits and links.
            </div>
          )}
        </div>
        {depth !== "simple" && <MemorySplit m={m} />}
      </div>
      <div className="ak-sys-controls">
        <div className="ak-seg" role="radiogroup" aria-label="Diagram">
          {MAPS.map((k) => (
            <button
              key={k}
              type="button"
              role="radio"
              aria-checked={map === k}
              className={map === k ? "on" : ""}
              onClick={() => setMap(k)}
            >
              {k === "services" ? "System" : "Memory"}
            </button>
          ))}
        </div>
        <div className="ak-seg" role="radiogroup" aria-label="Detail">
          {DEPTHS.map((d) => (
            <button
              key={d}
              type="button"
              role="radio"
              aria-checked={depth === d}
              className={depth === d ? "on" : ""}
              onClick={() => setDepth(d)}
              title={DEPTH_HINT[d]}
            >
              {d[0].toUpperCase() + d.slice(1)}
            </button>
          ))}
        </div>
        {features.length > 1 && (
          <select
            className="ak-input"
            value={featureId}
            aria-label="Feature"
            onChange={(e) => {
              setFeatureId(e.target.value);
              setStep(-1);
              setPlaying(false);
            }}
          >
            <option value="">All features</option>
            {features.map((f) => (
              <option key={f.id} value={f.id}>
                {f.name}
              </option>
            ))}
          </select>
        )}
        <span className="ak-sys-sep" />
        {map === "services" && runs.length > 1 && (
          <select
            className="ak-input"
            value={runId}
            onChange={(e) => {
              setRunId(e.target.value);
              setStep(-1);
              setPlaying(false);
            }}
          >
            {runs.map((r) => (
              <option key={r.id} value={r.id}>
                {r.title}
              </option>
            ))}
          </select>
        )}
        {map === "memory" ? null : run ? (
          <>
            <button
              type="button"
              className="ak-btn ak-sys-play"
              onClick={() => (playing ? setPlaying(false) : play())}
            >
              {playing ? <Pause size={14} /> : <Play size={14} />}
              {playing ? "Pause" : step >= 0 ? "Continue" : "Play example run"}
            </button>
            <button
              type="button"
              className="ak-iconbtn"
              onClick={() => setStep((s) => Math.max(-1, s - 1))}
              aria-label="Previous step"
            >
              <ChevronLeft size={15} />
            </button>
            <button
              type="button"
              className="ak-iconbtn"
              onClick={() => setStep((s) => Math.min(run.hops.length - 1, s + 1))}
              aria-label="Next step"
            >
              <ChevronRight size={15} />
            </button>
            <button
              type="button"
              className="ak-iconbtn"
              onClick={() => {
                setStep(-1);
                setPlaying(false);
              }}
              aria-label="Reset"
            >
              {step >= 0 ? <RotateCcw size={15} /> : <Expand size={15} />}
            </button>
          </>
        ) : (
          <span className="ak-muted text-[12px]">
            {feature
              ? "No screen of this feature was traced to a route, so it has no example run."
              : "No UI element was traced to a backend route, so there is no example run yet."}
          </span>
        )}
      </div>
      {feature && (
        <div className="ak-sys-feature">
          <b>{feature.name}</b>
          <span className="ak-muted">{feature.explain}</span>
          {depth !== "simple" &&
            feature.stack.slice(0, 6).map((p) => (
              <span key={p} className="ak-chip ak-mono">
                {p}
              </span>
            ))}
          <button type="button" className="ak-link-btn ml-auto" onClick={() => setFeatureId("")}>
            show everything
          </button>
        </div>
      )}
      {map === "memory" ? (
        <MemoryMap feature={feature} depth={depth} />
      ) : (
        <div className="ak-sys-vp" ref={vp}>
          <motion.div
            className="ak-sys-canvas"
            style={{ width: m.width, height: m.height, originX: 0, originY: 0 }}
            animate={{ x: tx, y: ty, scale: zoom }}
            transition={{ type: "spring", stiffness: 70, damping: 20, mass: 0.9 }}
          >
            {lanes.map((l) => {
              const first = m.boxes.find((b) => b.lane === l);
              return (
                <div
                  key={l}
                  className="ak-sys-lane"
                  style={{ left: first?.x ?? 0, top: PAD - 6, width: CARD_W[l] }}
                >
                  {LANE_NAME[l]}
                </div>
              );
            })}
            <svg className="ak-sys-links" width={m.width} height={m.height} aria-hidden="true">
              <defs>
                <marker
                  id="sysarrow"
                  viewBox="0 0 10 10"
                  refX="9"
                  refY="5"
                  markerWidth="7"
                  markerHeight="7"
                  orient="auto-start-reverse"
                >
                  <path d="M0,0 L10,5 L0,10 z" fill="currentColor" />
                </marker>
              </defs>
              {m.links.map((l) => {
                const a = m.byId.get(l.from) as Box;
                const b = m.byId.get(l.to) as Box;
                const on =
                  !!activeSvc &&
                  !!prevSvc &&
                  prevSvc !== activeSvc &&
                  ((prevSvc === l.from && activeSvc === l.to) ||
                    (prevSvc === l.to && activeSvc === l.from));
                return (
                  <g key={`${l.from}-${l.to}`} className={on ? "on" : a.dim || b.dim ? "dim" : ""}>
                    <path
                      id={`sl-${l.from}-${l.to}`}
                      d={linkPath(a, b, ports.get(l))}
                      markerEnd="url(#sysarrow)"
                    />
                    <text dy={-5}>
                      <textPath
                        href={`#sl-${l.from}-${l.to}`}
                        startOffset="50%"
                        textAnchor="middle"
                      >
                        {linkLabel(l, m)}
                      </textPath>
                    </text>
                  </g>
                );
              })}
            </svg>
            {m.boxes.map((b) => (
              <BoxCard
                key={b.svc.id}
                b={b}
                active={activeSvc === b.svc.id}
                activeRow={activeSvc === b.svc.id ? activeRow : undefined}
                run={run}
                step={step}
                depth={depth}
                onOpen={() => setOpen(b.svc.id)}
                onFeature={(id) => setFeatureId(id)}
              />
            ))}
            {anchor && (
              <motion.div
                className={`ak-sys-packet ${hop?.kind === "response" ? "back" : ""}`}
                initial={false}
                animate={{
                  x: anchor.x - 9,
                  y: anchor.y - 9,
                  scale: travelling ? [1, 1.6, 1] : 1,
                }}
                transition={{ duration: travelling ? 0.9 : 0.45, ease: [0.65, 0, 0.35, 1] }}
              />
            )}
          </motion.div>
          <AnimatePresence>
            {hop && run && (
              <motion.div
                key={step}
                className="ak-sys-narrate ak-card"
                initial={{ opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.25 }}
              >
                <div className="flex items-baseline gap-2">
                  <span className="ak-sys-stepno">
                    {step + 1}/{run.hops.length}
                  </span>
                  <b className="truncate text-[14.5px]">{hop.label}</b>
                  <span className="ak-chip ml-auto shrink-0">{VIA_TEXT[hop.via]}</span>
                </div>
                <p className="mt-1 text-[13px] leading-relaxed">{hop.explain}</p>
                <div className="mt-1.5 flex flex-wrap items-center gap-1">
                  <span className="ak-muted text-[11px]">proof</span>
                  {hop.proof.map((p) => {
                    const t = proofTarget(ix, p);
                    return t ? (
                      <button
                        key={p}
                        type="button"
                        className="ak-ev ak-mono ak-ev-link"
                        onClick={() => reveal(t)}
                      >
                        {p}
                      </button>
                    ) : (
                      <span key={p} className="ak-ev ak-mono">
                        {p}
                      </span>
                    );
                  })}
                  <span className="ak-sys-conf" title="How sure the analysis is about this hop">
                    <i style={{ width: `${hop.confidence * 100}%` }} />
                  </span>
                </div>
              </motion.div>
            )}
          </AnimatePresence>
          <AnimatePresence>
            {opened && <ServicePanel b={opened} onClose={() => setOpen(null)} />}
          </AnimatePresence>
        </div>
      )}
    </div>
  );
}
