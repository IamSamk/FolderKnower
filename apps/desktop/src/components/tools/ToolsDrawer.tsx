// Tools sidebar for students: where data structures show up in this project, the ones
// it relies on without writing them, everyday problems mapped to structures (and to
// functions here that look like the same problem), and complexity hot spots.

import { Flame, Lightbulb, ScanSearch, Shapes, X } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useMemo, useState } from "react";
import { candidatesFor, hiddenStructures, hotSpots, PROBLEMS } from "../../lib/dsaTools";
import { DSA_EXPLAIN, DSA_LABEL } from "../../lib/insights";
import { useStore } from "../../store";

type Tab = "spot" | "hidden" | "life" | "hot";

export function ToolsDrawer() {
  const open = useStore((s) => s.toolsOpen);
  const close = useStore((s) => s.closeTools);
  const ix = useStore((s) => s.ix);
  const reveal = useStore((s) => s.reveal);
  const [tab, setTab] = useState<Tab>("spot");
  const [pid, setPid] = useState(PROBLEMS[0].id);
  useEffect(() => {
    if (!open) return;
    const k = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  }, [open, close]);
  const found = ix?.g.insights?.dataStructures ?? [];
  const byKind = useMemo(() => {
    const m = new Map<string, typeof found>();
    for (const f of found) m.set(f.kind, [...(m.get(f.kind) ?? []), f]);
    return [...m.entries()];
  }, [found]);
  const hidden = useMemo(() => (ix ? hiddenStructures(ix) : []), [ix]);
  const hot = useMemo(() => (ix ? hotSpots(ix) : []), [ix]);
  const problem = PROBLEMS.find((p) => p.id === pid) ?? PROBLEMS[0];
  const cands = useMemo(() => (ix ? candidatesFor(ix, problem) : []), [ix, problem]);
  const tabs: { k: Tab; label: string; icon: React.ReactNode; n?: number }[] = [
    { k: "spot", label: "In your code", icon: <ScanSearch size={14} />, n: found.length },
    { k: "hidden", label: "Hidden inside tools", icon: <Shapes size={14} />, n: hidden.length },
    { k: "life", label: "Real-life problem", icon: <Lightbulb size={14} /> },
    { k: "hot", label: "Slow spots", icon: <Flame size={14} />, n: hot.length },
  ];
  return (
    <AnimatePresence>
      {open && ix && (
        <motion.aside
          className="ak-tools"
          initial={{ opacity: 0, x: 40 }}
          animate={{ opacity: 1, x: 0 }}
          exit={{ opacity: 0, x: 40 }}
          transition={{ type: "spring", stiffness: 360, damping: 34 }}
        >
          <div className="ak-tools-top">
            <div className="text-[15px] font-bold">DSA tools</div>
            <button type="button" className="ak-iconbtn ml-auto" onClick={close} aria-label="Close">
              <X size={15} />
            </button>
          </div>
          <nav className="ak-tools-tabs">
            {tabs.map((t) => (
              <button
                key={t.k}
                type="button"
                className={tab === t.k ? "on" : ""}
                onClick={() => setTab(t.k)}
              >
                {t.icon}
                {t.label}
                {t.n !== undefined && <span className="ak-count">{t.n}</span>}
              </button>
            ))}
          </nav>
          <div className="ak-tools-body">
            {tab === "spot" &&
              (byKind.length ? (
                byKind.map(([kind, list]) => (
                  <div key={kind} className="ak-tool-card">
                    <b>{DSA_LABEL[kind as keyof typeof DSA_LABEL]}</b>
                    <p className="ak-muted">{DSA_EXPLAIN[kind as keyof typeof DSA_EXPLAIN]}</p>
                    {list.slice(0, 5).map((f) => (
                      <button
                        key={f.id}
                        type="button"
                        className="ak-tool-row"
                        onClick={() => reveal(f.functionId)}
                      >
                        <span className="ak-mono">{ix.label(f.functionId)}()</span>
                        <span className="ak-muted ak-mono truncate">
                          {ix.files.get(f.fileId)?.file.path}:{f.line}
                        </span>
                        {f.subject && <span className="ak-chip">{f.subject}</span>}
                      </button>
                    ))}
                  </div>
                ))
              ) : (
                <p className="ak-muted">
                  No stack, queue, tree or graph code was written by hand here. Check the hidden
                  tab: the tools you use are full of them.
                </p>
              ))}
            {tab === "hidden" &&
              (hidden.length ? (
                hidden.map((h) => (
                  <div key={h.key} className="ak-tool-card">
                    <div className="flex items-baseline gap-2">
                      <b>{h.title}</b>
                      <span className="ak-chip ml-auto">{DSA_LABEL[h.kind]}</span>
                    </div>
                    <p>{h.why}</p>
                    <button type="button" className="ak-tool-row" onClick={() => reveal(h.ref)}>
                      <span className="ak-mono truncate">{h.where}</span>
                    </button>
                  </div>
                ))
              ) : (
                <p className="ak-muted">No vector store, agent graph, cache or routes found.</p>
              ))}
            {tab === "life" && (
              <>
                <div className="ak-tool-problems">
                  {PROBLEMS.map((p) => (
                    <button
                      key={p.id}
                      type="button"
                      className={p.id === pid ? "on" : ""}
                      onClick={() => setPid(p.id)}
                    >
                      {p.title}
                    </button>
                  ))}
                </div>
                <div className="ak-tool-card">
                  <b>{problem.title}</b>
                  <p className="ak-muted">In real life: {problem.life}</p>
                  <p>
                    Use{" "}
                    {problem.kinds.map((k, i) => (
                      <span key={k}>
                        {i > 0 && " or "}
                        <b>{DSA_LABEL[k]}</b>
                      </span>
                    ))}
                    . {problem.how}
                  </p>
                  <div className="ak-section-title mt-2">In this project</div>
                  {cands.length ? (
                    cands.map((c) => (
                      <button
                        key={c.id}
                        type="button"
                        className="ak-tool-row"
                        onClick={() => reveal(c.id)}
                      >
                        <span className="ak-mono">{c.name}()</span>
                        <span className="ak-muted ak-mono truncate">{c.path}</span>
                        <span className={`ak-chip ${c.found ? "ak-chip-yes" : ""}`}>
                          {c.found ? "uses it" : "name match, check it"}
                        </span>
                      </button>
                    ))
                  ) : (
                    <p className="ak-muted">
                      Nothing here looks like this problem yet. A good place to practise: add it as
                      a feature.
                    </p>
                  )}
                </div>
              </>
            )}
            {tab === "hot" &&
              (hot.length ? (
                <div className="ak-tool-card">
                  <p className="ak-muted">
                    Functions whose loops nest or whose cost grows fast with the input. A hash map
                    or a sort first often turns O(N²) into O(N) or O(N log N).
                  </p>
                  {hot.map((h) => (
                    <button
                      key={h.fn.id}
                      type="button"
                      className="ak-tool-row"
                      onClick={() => reveal(h.fn.id)}
                    >
                      <span className="ak-mono">{h.fn.name}()</span>
                      <span className="ak-muted ak-mono truncate">{h.path}</span>
                      <span className="ak-chip">{h.why}</span>
                    </button>
                  ))}
                </div>
              ) : (
                <p className="ak-muted">No deeply nested or quadratic functions were found.</p>
              ))}
          </div>
        </motion.aside>
      )}
    </AnimatePresence>
  );
}
