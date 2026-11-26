// Run your project: pick a detected start command, confirm the exact command and folder,
// then watch the real process tree and docker containers. Every number here is measured
// on this computer while the project runs. The prediction from the code lives in the
// System view; this panel is the measurement.

import { Play, Plug, Square } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { fmtBytes } from "../../lib/insights";
import {
  type ContainerRow,
  canRun,
  type PortRow,
  parseDockerBytes,
  type RunOption,
  type RunStatus,
  runContainers,
  runDetect,
  runPorts,
  runStart,
  runStatus,
  runStop,
} from "../../lib/run";
import { processDetail } from "../../lib/telemetry";
import { useStore } from "../../store";
import { Spark } from "./LiveView";

const HIST = 60;
const SYSTEMISH = /^(system|svchost|chrome|msedge|code|explorer|firefox|slack|spotify)/i;

export function RunPanel({ onInspect }: { onInspect: (pid: number) => void }) {
  const root = useStore((s) => s.root);
  const [opts, setOpts] = useState<RunOption[]>([]);
  const [pick, setPick] = useState<RunOption | null>(null);
  const [st, setSt] = useState<RunStatus | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [tree, setTree] = useState<{ mem: number; n: number } | null>(null);
  const [ports, setPorts] = useState<PortRow[]>([]);
  const [boxes, setBoxes] = useState<ContainerRow[]>([]);
  const [attach, setAttach] = useState(false);
  const memHist = useRef<number[]>([]);
  const boxHist = useRef(new Map<string, number[]>());
  const [, force] = useState(0);
  const logEl = useRef<HTMLPreElement>(null);

  useEffect(() => {
    if (root && canRun()) void runDetect(root).then(setOpts);
    void runStatus().then(setSt);
  }, [root]);

  const live = st?.running ?? false;
  const runPid = live ? (st?.pid ?? null) : null;
  useEffect(() => {
    if (runPid != null) onInspect(runPid);
  }, [runPid]);
  useEffect(() => {
    if (!live && !attach) return;
    let alive = true;
    const tick = async () => {
      const [s, p, c] = await Promise.all([
        runStatus(),
        runPorts(),
        runContainers().catch(() => []),
      ]);
      if (!alive) return;
      setSt(s);
      setPorts(p);
      setBoxes(c);
      for (const b of c)
        boxHist.current.set(
          b.name,
          [...(boxHist.current.get(b.name) ?? []), parseDockerBytes(b.memUsage)].slice(-HIST),
        );
      if (s?.pid && s.running) {
        const d = await processDetail(s.pid).catch(() => null);
        if (d && alive) {
          setTree({ mem: d.treeMemory, n: d.children.length + 1 });
          memHist.current = [...memHist.current, d.treeMemory].slice(-HIST);
        }
      }
      force((x) => x + 1);
    };
    void tick();
    const t = setInterval(() => void tick(), 2000);
    return () => {
      alive = false;
      clearInterval(t);
    };
  }, [live, attach]);

  useEffect(() => {
    logEl.current?.scrollTo({ top: logEl.current.scrollHeight });
  }, [st?.log.length]);

  const start = async () => {
    if (!pick) return;
    setErr(null);
    memHist.current = [];
    try {
      setSt(await runStart(pick.command, pick.cwd));
      setPick(null);
    } catch (e) {
      setErr(String(e));
    }
  };

  if (!canRun())
    return (
      <section className="ak-card ak-muted text-[12.5px]">
        <div className="ak-live-title">Run your project</div>
        Launching and measuring your project needs the desktop app. This browser demo can only show
        simulated numbers.
      </section>
    );

  const mine = ports.filter((p) => !SYSTEMISH.test(p.name));

  return (
    <section className="ak-card">
      <div className="ak-live-title">
        <Play size={14} /> Run your project
        <button
          type="button"
          className="ak-btn ml-auto"
          onClick={() => setAttach((a) => !a)}
          title="Find an app that is already running by its port"
        >
          <Plug size={12} /> {attach ? "Hide ports" : "Attach by port"}
        </button>
      </div>
      <div className="ak-run-label">
        Measured on this computer while the project runs. Not an estimate.
      </div>

      {!st?.running && !pick && (
        <div className="ak-run-opts">
          {opts.length === 0 && (
            <div className="ak-muted text-[12.5px]">
              No start command found (docker compose, npm scripts, cargo run, python).
            </div>
          )}
          {opts.map((o) => (
            <button key={o.id} type="button" onClick={() => setPick(o)} title={o.why}>
              <b>{o.label}</b>
              <span className="ak-mono ak-muted">{o.command}</span>
            </button>
          ))}
        </div>
      )}

      {pick && (
        <div className="ak-run-confirm">
          <b>Run this on your computer?</b>
          <div className="ak-mono">{pick.command}</div>
          <div className="ak-muted ak-mono">in {pick.cwd}</div>
          <div className="ak-muted text-[12px]">
            It runs your project's own code with your user rights, the same as typing it in a
            terminal. FolderKnower does not change your files. {pick.why}.
          </div>
          <div className="flex gap-2">
            <button type="button" className="ak-btn primary" onClick={() => void start()}>
              Run
            </button>
            <button type="button" className="ak-btn" onClick={() => setPick(null)}>
              Cancel
            </button>
          </div>
        </div>
      )}
      {err && <div className="ak-run-err">{err}</div>}

      {st && (
        <>
          <div className="ak-run-head">
            <span className={`ak-run-dot ${st.running ? "on" : ""}`} />
            <span className="ak-mono truncate">{st.command}</span>
            <span className="ak-muted">
              {st.running ? `running, pid ${st.pid}` : `stopped (exit ${st.exitCode ?? "?"})`}
            </span>
            {st.running && (
              <button
                type="button"
                className="ak-btn ml-auto"
                onClick={() => void runStop().then(setSt)}
              >
                <Square size={12} /> Stop
              </button>
            )}
          </div>
          {st.running && tree && (
            <div className="ak-run-row">
              <span>Process tree ({tree.n} processes)</span>
              <Spark v={memHist.current} />
              <span className="ak-mono">{fmtBytes(tree.mem)}</span>
              {st.pid && (
                <button
                  type="button"
                  className="ak-btn"
                  onClick={() => onInspect(st.pid as number)}
                >
                  inspect
                </button>
              )}
            </div>
          )}
          <pre className="ak-run-log ak-mono" ref={logEl}>
            {st.log.length ? st.log.slice(-60).join("\n") : "(no output yet)"}
          </pre>
        </>
      )}

      {boxes.length > 0 && (
        <div className="ak-run-boxes">
          <div className="ak-muted text-[11.5px]">Docker containers (docker stats)</div>
          {boxes.map((b) => (
            <div key={b.name} className="ak-run-row">
              <span className="ak-mono truncate">{b.name}</span>
              <Spark v={boxHist.current.get(b.name) ?? []} />
              <span className="ak-mono">{b.memUsage.split("/")[0]}</span>
              <span className="ak-mono ak-muted">{b.cpu}</span>
            </div>
          ))}
        </div>
      )}

      {attach && (
        <div className="ak-run-boxes">
          <div className="ak-muted text-[11.5px]">
            Listening ports. Pick one to measure the process that owns it.
          </div>
          {mine.slice(0, 40).map((p) => (
            <button
              key={`${p.port}-${p.pid}`}
              type="button"
              className="ak-run-port"
              onClick={() => onInspect(p.pid)}
            >
              <span className="ak-mono">:{p.port}</span>
              <span className="truncate">{p.name}</span>
              <span className="ak-mono ak-muted">pid {p.pid}</span>
            </button>
          ))}
        </div>
      )}
      {!st && !pick && opts.length > 0 && (
        <div className="ak-muted mt-2 text-[11.5px]">
          Nothing runs until you press Run and confirm.
        </div>
      )}
    </section>
  );
}
