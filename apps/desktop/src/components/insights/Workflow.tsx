// The Workflow drawer: for the function in focus (or the functions at the current zoom
// level) it draws the control flow and lights up the clean path, the run where nothing
// goes wrong. Error exits are shown in red: the code says where a run *can* leave
// early, but which way a real run goes depends on its input, so that part stays a map.

import { CircleAlert, GitBranch, Pause, Play, Repeat, Search, X } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { FlowNode } from "../../gen";
import type { FnEntry } from "../../lib/graph";
import { FLOW_TEXT, idealPath, idealSteps, type PathMark } from "../../lib/insights";
import { type Focus, useStore } from "../../store";

function candidates(fns: Map<string, FnEntry>, focus: Focus): FnEntry[] {
  const all = [...fns.values()].filter((f) => f.fn.flow?.length);
  const pick = (() => {
    switch (focus.level) {
      case "function":
        return all.filter((f) => f.fn.id === focus.fnId);
      case "file":
        return all.filter((f) => f.file.id === focus.fileId);
      case "cluster":
        return all.filter((f) => f.clusterId === focus.clusterId);
      case "tier":
        return all.filter((f) => f.tier === focus.tier);
      default:
        return all;
    }
  })();
  return pick.sort(
    (a, b) =>
      b.fn.complexity.cyclomatic - a.fn.complexity.cyclomatic || a.fn.name.localeCompare(b.fn.name),
  );
}

const KIND_ICON: Partial<Record<FlowNode["kind"], React.ReactNode>> = {
  branch: <GitBranch size={13} />,
  else: <GitBranch size={13} />,
  loop: <Repeat size={13} />,
  raise: <CircleAlert size={13} />,
};

function FlowRow({
  n,
  marks,
  at,
  depth,
}: {
  n: FlowNode;
  marks: Map<FlowNode, PathMark>;
  at: FlowNode | null;
  depth: number;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const m = marks.get(n) ?? "skipped";
  const on = at === n;
  useEffect(() => {
    if (on) ref.current?.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }, [on]);
  return (
    <div className={`ak-flow-node ${m}`}>
      <motion.div
        ref={ref}
        className={`ak-flow-row ${on ? "on" : ""}`}
        animate={{ x: on ? 4 : 0 }}
        transition={{ type: "spring", stiffness: 500, damping: 30 }}
      >
        <span className="ak-flow-kind">
          {KIND_ICON[n.kind]}
          {n.error && n.kind !== "raise" ? "error exit" : FLOW_TEXT[n.kind]}
        </span>
        <span className="ak-mono min-w-0 flex-1 truncate">{n.label}</span>
        <span className="ak-muted ak-mono text-[10.5px]">:{n.line}</span>
      </motion.div>
      {(n.children?.length ?? 0) > 0 && depth < 8 && (
        <div className="ak-flow-kids">
          {n.children?.map((c) => (
            <FlowRow
              key={`${c.line}-${c.kind}-${c.label}`}
              n={c}
              marks={marks}
              at={at}
              depth={depth + 1}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function FlowPanel({ e }: { e: FnEntry }) {
  const flow = e.fn.flow ?? [];
  const marks = useMemo(() => idealPath(flow), [flow]);
  const steps = useMemo(() => idealSteps(flow, marks), [flow, marks]);
  const errors = useMemo(() => [...marks.values()].filter((m) => m === "error").length, [marks]);
  const [i, setI] = useState(-1);
  useEffect(() => setI(-1), [e.fn.id]);
  useEffect(() => {
    if (i < 0) return;
    const t = setTimeout(() => setI((x) => (x + 1 < steps.length ? x + 1 : -1)), 900);
    return () => clearTimeout(t);
  }, [i, steps.length]);
  const select = useStore((s) => s.select);
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="ak-flow-head">
        <div className="min-w-0">
          <div className="ak-mono truncate text-[15px] font-bold">{e.fn.name}()</div>
          <div className="ak-muted truncate text-[12px]">{e.file.path}</div>
        </div>
        <button
          type="button"
          className="ak-btn primary ml-auto"
          onClick={() => setI(i < 0 ? 0 : -1)}
        >
          {i < 0 ? <Play size={14} /> : <Pause size={14} />}
          {i < 0 ? "Walk the clean path" : "Stop"}
        </button>
      </div>
      <div className="ak-flow-legend">
        <span className="ideal">clean path ({steps.length} steps)</span>
        <span className="branch">decision taken</span>
        <span className="error">can leave on an error ({errors})</span>
        <span className="skipped">not on the clean path</span>
      </div>
      <div className="ak-flow-body">
        {flow.map((n) => (
          <FlowRow
            key={`${n.line}-${n.kind}-${n.label}`}
            n={n}
            marks={marks}
            at={i >= 0 ? steps[i] : null}
            depth={0}
          />
        ))}
      </div>
      <div className="ak-flow-foot">
        Red steps are where a run <i>can</i> stop early (a raise, a{" "}
        <span className="ak-mono">return err</span>, a <span className="ak-mono">?</span>). Whether
        it does depends on the input at run time, which reading the code cannot know.{" "}
        <button type="button" className="ak-link-btn" onClick={() => select(e.fn.id)}>
          Inspect this function
        </button>
      </div>
    </div>
  );
}

export function WorkflowDrawer() {
  const open = useStore((s) => s.workflowOpen);
  const fnId = useStore((s) => s.workflowFn);
  const close = useStore((s) => s.closeWorkflow);
  const openWorkflow = useStore((s) => s.openWorkflow);
  const ix = useStore((s) => s.ix);
  const stack = useStore((s) => s.stack);
  const view = useStore((s) => s.view);
  const [q, setQ] = useState("");
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, close]);
  const focus = view === "architecture" ? stack[stack.length - 1] : ({ level: "system" } as Focus);
  const list = useMemo(() => (ix ? candidates(ix.fns, focus) : []), [ix, focus]);
  const shown = q ? list.filter((f) => f.fn.name.toLowerCase().includes(q.toLowerCase())) : list;
  const current = (fnId && ix?.fns.get(fnId)) || list[0];
  return (
    <AnimatePresence>
      {open && ix && (
        <motion.aside
          className="ak-workflow"
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: 30 }}
          transition={{ type: "spring", stiffness: 360, damping: 34 }}
        >
          <div className="ak-workflow-top">
            <div className="text-[15px] font-bold">Workflow: the clean path</div>
            <span className="ak-muted text-[12px]">
              {focus.level === "system" ? "whole project" : `this ${focus.level}`}, {list.length}{" "}
              functions
            </span>
            <button
              type="button"
              className="ak-iconbtn ml-auto"
              onClick={close}
              title="Close (Esc)"
            >
              <X size={15} />
            </button>
          </div>
          <div className="flex min-h-0 flex-1">
            {list.length > 1 && (
              <div className="ak-workflow-list">
                <label className="ak-workflow-search">
                  <Search size={13} />
                  <input
                    value={q}
                    onChange={(e) => setQ(e.target.value)}
                    placeholder="find a function"
                  />
                </label>
                {shown.slice(0, 200).map((f) => (
                  <button
                    key={f.fn.id}
                    type="button"
                    className={current?.fn.id === f.fn.id ? "on" : ""}
                    onClick={() => openWorkflow(f.fn.id)}
                  >
                    <span className="ak-mono truncate">{f.fn.name}</span>
                    <span className="ak-muted ml-auto text-[10.5px]">
                      {f.fn.complexity.cyclomatic}
                    </span>
                  </button>
                ))}
              </div>
            )}
            {current ? (
              <FlowPanel e={current} />
            ) : (
              <div className="ak-empty">
                <p>No function with branches or steps at this level.</p>
              </div>
            )}
          </div>
        </motion.aside>
      )}
    </AnimatePresence>
  );
}
