// Live: what your project is doing right now, explained. Its own process tree, a breakdown of one process's memory into the
// kinds a student meets in an OS course, and the local model servers with their KV
// cache. Sampling is unprivileged and stays on this computer.

import { Activity, Server } from "lucide-react";
import { motion } from "motion/react";
import { useEffect, useMemo, useRef, useState } from "react";
import { fmtBytes } from "../../lib/insights";
import { runPorts } from "../../lib/run";
import {
  type ProcessDetail,
  processDetail,
  type RuntimeStatus,
  runtimes,
  simulated,
  systemSnapshot,
} from "../../lib/telemetry";
import { KvCalculator } from "../insights/KvCache";
import { RunPanel } from "./RunPanel";

const HISTORY = 40;

export function Spark({
  v,
  max,
  color = "var(--accent)",
}: {
  v: number[];
  max?: number;
  color?: string;
}) {
  if (v.length < 2) return <svg className="ak-spark" aria-hidden />;
  // Without a fixed scale, stretch the sample's own range so small changes show.
  const lo = max == null ? Math.min(...v) : 0;
  const hi = max ?? Math.max(...v);
  const span = Math.max(hi - lo, 1);
  const pts = v
    .map((x, i) => `${(i / (v.length - 1)) * 100},${29 - ((x - lo) / span) * 27}`)
    .join(" ");
  return (
    <svg className="ak-spark" viewBox="0 0 100 30" preserveAspectRatio="none" aria-hidden>
      <polyline
        points={pts}
        fill="none"
        stroke={color}
        strokeWidth={2}
        vectorEffect="non-scaling-stroke"
      />
    </svg>
  );
}

function Meter({
  parts,
  total,
}: {
  parts: { v: number; c: string; label: string }[];
  total: number;
}) {
  return (
    <div className="ak-meter">
      {parts.map((p) => (
        <motion.span
          key={p.label}
          title={`${p.label}: ${fmtBytes(p.v)}`}
          style={{ background: p.c }}
          animate={{ width: `${Math.min(100, (p.v / Math.max(total, 1)) * 100)}%` }}
          transition={{ type: "spring", stiffness: 200, damping: 30 }}
        />
      ))}
    </div>
  );
}

const BREAKDOWN: {
  k: keyof NonNullable<ProcessDetail["breakdown"]>;
  label: string;
  c: string;
  explain: string;
}[] = [
  {
    k: "anonymous",
    label: "Heap and private",
    c: "#8FB0FF",
    explain:
      "Memory the program asked for itself: Python objects, arrays, tensors. Only this process has it.",
  },
  {
    k: "fileBacked",
    label: "Mapped files",
    c: "#7FE0B4",
    explain:
      "Files mapped into memory: shared libraries and, for model servers, the model weights (mmap). The OS can drop and re-read these.",
  },
  {
    k: "shmem",
    label: "Shared memory",
    c: "#FFD25E",
    explain: "Pages several processes see at once, used to pass data between them without copying.",
  },
  {
    k: "swap",
    label: "Swapped out",
    c: "#FF9C8F",
    explain: "Pages pushed to disk. Touching them again costs a disk read.",
  },
];

function DetailCard({ d }: { d: ProcessDetail }) {
  const b = d.breakdown;
  const threads = useMemo(() => {
    const m = new Map<string, number>();
    for (const t of d.threadNames) m.set(t, (m.get(t) ?? 0) + 1);
    return [...m.entries()].sort((a, c) => c[1] - a[1]);
  }, [d.threadNames]);
  return (
    <section className="ak-card">
      <div className="ak-live-title">
        <Activity size={15} /> {d.row.name}{" "}
        <span className="ak-muted ak-mono">pid {d.row.pid}</span>
      </div>
      <div className="ak-mono ak-muted truncate text-[11.5px]" title={d.row.cmd}>
        {d.row.cmd}
      </div>
      <div className="ak-live-stats">
        <div>
          <b>{fmtBytes(d.row.memory)}</b>
          <span>resident (RSS)</span>
        </div>
        <div>
          <b>{fmtBytes(d.row.virtualMemory)}</b>
          <span>virtual size</span>
        </div>
        <div>
          <b>{d.row.cpu.toFixed(0)}%</b>
          <span>of one core</span>
        </div>
        <div>
          <b>{d.row.threads ?? "?"}</b>
          <span>threads</span>
        </div>
        <div>
          <b>{fmtBytes(d.treeMemory)}</b>
          <span>with {d.children.length} children</span>
        </div>
        {b && (
          <div>
            <b>{fmtBytes(b.pss)}</b>
            <span>fair share (PSS)</span>
          </div>
        )}
      </div>
      <p className="ak-muted text-[12px]">
        Resident is what sits in RAM now. Virtual size counts every address the process reserved,
        most of it never touched, which is why it looks huge.
      </p>
      {b && (
        <>
          <Meter
            total={Math.max(b.rss + b.swap, 1)}
            parts={BREAKDOWN.map((x) => ({ v: b[x.k], c: x.c, label: x.label }))}
          />
          <ul className="ak-breakdown">
            {BREAKDOWN.map((x) => (
              <li key={x.k}>
                <i style={{ background: x.c }} />
                <b>{x.label}</b> <span className="ak-mono">{fmtBytes(b[x.k])}</span>
                <div className="ak-muted text-[12px]">{x.explain}</div>
              </li>
            ))}
          </ul>
          <div className="ak-muted text-[12px]">
            Peak resident so far {fmtBytes(b.peakRss)}. Main thread stack {fmtBytes(b.stack)}: every
            function call pushes a frame here and pops it on return.
          </div>
        </>
      )}
      {d.children.length > 0 && (
        <>
          <div className="ak-section-title mt-3">Child processes</div>
          {d.children.slice(0, 8).map((c) => (
            <div key={c.pid} className="ak-live-child">
              <span className="ak-mono">{c.name}</span>
              <span className="ak-muted ml-auto">{fmtBytes(c.memory)}</span>
            </div>
          ))}
        </>
      )}
      {threads.length > 0 && (
        <>
          <div className="ak-section-title mt-3">Threads by name</div>
          <div className="flex flex-wrap gap-1">
            {threads.slice(0, 16).map(([n, c]) => (
              <span key={n} className="ak-chip ak-mono">
                {n} {c > 1 ? `×${c}` : ""}
              </span>
            ))}
          </div>
        </>
      )}
    </section>
  );
}

function RuntimeCard({ r }: { r: RuntimeStatus }) {
  return (
    <section className="ak-card">
      <div className="ak-live-title">
        <Server size={15} /> {r.server}
        <span className="ak-muted ak-mono ml-auto text-[11px]">{r.endpoint}</span>
      </div>
      {r.models.map((m) => (
        <div key={m.name} className="ak-live-model">
          <div className="flex items-baseline gap-2">
            <b className="ak-mono">{m.name}</b>
            {m.parameterSize && <span className="ak-chip">{m.parameterSize}</span>}
            {m.quantization && <span className="ak-chip">{m.quantization}</span>}
          </div>
          <div className="ak-muted text-[12px]">
            {fmtBytes(m.sizeBytes)} loaded, {fmtBytes(m.vramBytes)} of it on the GPU
            {m.contextLength ? `, context window ${m.contextLength.toLocaleString()} tokens` : ""}
          </div>
          <Meter
            total={m.sizeBytes}
            parts={[
              { v: m.vramBytes, c: "#BFA6FF", label: "GPU memory" },
              { v: m.sizeBytes - m.vramBytes, c: "#C9CDDC", label: "system RAM" },
            ]}
          />
        </div>
      ))}
      {r.kvCacheUsage != null && (
        <div className="mt-2">
          <div className="flex text-[12.5px]">
            <b>KV cache</b>
            <span className="ml-auto ak-mono">{(r.kvCacheUsage * 100).toFixed(0)}% full</span>
          </div>
          <Meter
            total={1}
            parts={[
              {
                v: r.kvCacheUsage,
                c: r.kvCacheUsage > 0.85 ? "var(--danger)" : "#67E8F9",
                label: "KV cache",
              },
            ]}
          />
          <div className="ak-muted text-[12px]">
            Keys and values of every token the running requests have seen. Near 100% the server
            pauses or evicts requests.
          </div>
        </div>
      )}
      <div className="ak-live-stats">
        {r.prefixCacheHitRate != null && (
          <div>
            <b>{(r.prefixCacheHitRate * 100).toFixed(0)}%</b>
            <span>prefix cache hits</span>
          </div>
        )}
        {r.requestsRunning != null && (
          <div>
            <b>{r.requestsRunning}</b>
            <span>running</span>
          </div>
        )}
        {r.requestsWaiting != null && (
          <div>
            <b>{r.requestsWaiting}</b>
            <span>waiting</span>
          </div>
        )}
        {r.promptTokensTotal != null && (
          <div>
            <b>{r.promptTokensTotal.toLocaleString()}</b>
            <span>prompt tokens</span>
          </div>
        )}
        {r.generatedTokensTotal != null && (
          <div>
            <b>{r.generatedTokensTotal.toLocaleString()}</b>
            <span>generated</span>
          </div>
        )}
        {r.busySlots != null && (
          <div>
            <b>
              {r.busySlots}/{r.slotContexts.length}
            </b>
            <span>slots busy</span>
          </div>
        )}
      </div>
      {r.notes.map((n) => (
        <div key={n} className="ak-muted text-[12px]">
          {n}
        </div>
      ))}
    </section>
  );
}

export function LiveView() {
  const [target, setTarget] = useState<number | null>(null);
  const [sel, setSel] = useState<number | null>(null);
  const [detail, setDetail] = useState<ProcessDetail | null>(null);
  const [selDetail, setSelDetail] = useState<ProcessDetail | null>(null);
  const [rts, setRts] = useState<RuntimeStatus[]>([]);
  const [ramBytes, setRam] = useState<number | undefined>();
  const hist = useRef(new Map<number, number[]>());
  const [, force] = useState(0);
  const sim = simulated();

  useEffect(() => {
    void systemSnapshot()
      .then((s) => setRam(s.totalMemory))
      .catch(() => undefined);
  }, []);

  // Everything below follows one process tree: the project's own. Nothing machine-wide.
  useEffect(() => {
    if (target == null) {
      setDetail(null);
      setSelDetail(null);
      setRts([]);
      return;
    }
    let alive = true;
    let n = 0;
    const tick = async () => {
      try {
        const d = await processDetail(target);
        if (!alive) return;
        setDetail(d);
        if (d)
          for (const r of [d.row, ...d.children])
            hist.current.set(r.pid, [...(hist.current.get(r.pid) ?? []), r.memory].slice(-HISTORY));
        if (n++ % 3 === 0) {
          const [all, ports] = await Promise.all([runtimes().catch(() => []), runPorts()]);
          const mine = new Set([target, ...(d?.children.map((c) => c.pid) ?? [])]);
          const myPorts = new Set(ports.filter((p) => mine.has(p.pid)).map((p) => p.port));
          if (alive)
            setRts(
              sim
                ? all
                : all.filter((r) => myPorts.has(Number(/:(\d+)/.exec(r.endpoint)?.[1] ?? 0))),
            );
        }
        force((x) => x + 1);
      } catch {
        /* the next tick retries */
      }
    };
    void tick();
    const t = setInterval(() => void tick(), 1500);
    return () => {
      alive = false;
      clearInterval(t);
    };
  }, [target, sim]);

  const shownPid = sel ?? target;
  useEffect(() => {
    if (shownPid == null || shownPid === target) {
      setSelDetail(null);
      return;
    }
    let alive = true;
    void processDetail(shownPid).then((d) => alive && setSelDetail(d));
    return () => {
      alive = false;
    };
  }, [shownPid, target]);

  const rows = detail ? [detail.row, ...detail.children] : [];
  const shown = selDetail ?? detail;
  return (
    <div className="ak-live">
      {sim && (
        <div className="ak-live-sim">
          Simulated numbers: this is the browser demo. The desktop app measures your own project
          while it runs, and nothing leaves your computer.
        </div>
      )}
      <div className="ak-live-grid">
        <div className="space-y-4">
          <RunPanel
            onInspect={(p) => {
              setTarget(p);
              setSel(null);
            }}
          />
          {detail && (
            <section className="ak-card">
              <div className="ak-live-title">
                Your project's processes
                <span className="ak-muted ml-auto text-[12px]">
                  {rows.length} processes, {fmtBytes(detail.treeMemory)} together
                </span>
              </div>
              <div className="ak-proc">
                {rows.map((r) => (
                  <button
                    key={r.pid}
                    type="button"
                    className={shownPid === r.pid ? "on" : ""}
                    onClick={() => setSel(r.pid)}
                  >
                    <span className="ak-mono truncate">{r.name}</span>
                    <Spark v={hist.current.get(r.pid) ?? []} />
                    <span className="ak-mono">{fmtBytes(r.memory)}</span>
                    <span className="ak-mono ak-muted">{r.cpu.toFixed(0)}%</span>
                    <span className="ak-mono ak-muted">{r.threads ?? ""}</span>
                  </button>
                ))}
              </div>
              <div className="ak-muted mt-1 text-[11px]">
                columns: name, memory trend, resident memory, CPU, threads
              </div>
            </section>
          )}
        </div>
        <div className="space-y-4">
          {shown && <DetailCard d={shown} />}
          {rts.map((r) => (
            <RuntimeCard key={r.endpoint} r={r} />
          ))}
          {!detail && (
            <section className="ak-card ak-muted text-[12.5px]">
              Run your project or attach to it by port. Only that project's processes, containers
              and model servers appear here, never the rest of this computer.
            </section>
          )}
          <section className="ak-card">
            <div className="ak-live-title">Will a model fit on this computer?</div>
            <KvCalculator ramBytes={ramBytes} />
          </section>
        </div>
      </div>
    </div>
  );
}
