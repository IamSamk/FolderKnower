import { AnimatePresence, motion } from "motion/react";
import { useMemo, useState } from "react";
import type { UiElement } from "../gen";
import { shortPath } from "../lib/graph";
import { useStore } from "../store";
import { TraceStrip } from "./Trace";

function Mock({
  el,
  active,
  hot,
  inspect,
  onPick,
  onHover,
}: {
  el: UiElement;
  active: boolean;
  hot: boolean;
  inspect: boolean;
  onPick: () => void;
  onHover: (b: boolean) => void;
}) {
  const label = el.label ?? el.tag;
  const common = {
    onMouseEnter: () => onHover(true),
    onMouseLeave: () => onHover(false),
    onClick: (e: React.MouseEvent) => {
      e.preventDefault();
      onPick();
    },
    className: `ak-mock ${active ? "ak-mock-active" : ""} ${hot && inspect ? "ak-mock-hot" : ""} ${el.handler ? "" : "ak-mock-unwired"}`,
  };
  let body: React.ReactNode;
  switch (el.tag) {
    case "input":
    case "textarea":
      body = <div className="ak-mock-input">{label}</div>;
      break;
    case "select":
      body = <div className="ak-mock-input">{label} ▾</div>;
      break;
    case "a":
    case "Link":
      body = <span className="ak-mock-link">{label}</span>;
      break;
    case "form":
      body = <div className="ak-mock-form">form · {el.event ?? "submit"}</div>;
      break;
    default:
      body = <div className="ak-mock-button">{label}</div>;
  }
  return (
    <div {...common}>
      {body}
      <AnimatePresence>
        {hot && inspect && (
          <motion.div
            initial={{ opacity: 0, y: 4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            className="ak-devtip"
          >
            <span className="text-[#F472B6]">{el.tag}</span>
            {el.event && <span className="text-[#93C5FD]">.{el.event}</span>}
            <span className="ak-muted"> → </span>
            <span>{el.handler ? el.handler.split("#").pop() : "unresolved"}</span>
            <span className="ak-muted"> · L{el.line}</span>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

export function UiMap() {
  const ix = useStore((s) => s.ix);
  const selected = useStore((s) => s.selected);
  const select = useStore((s) => s.select);
  const inspect = useStore((s) => s.inspect);
  const [hover, setHover] = useState<string | null>(null);
  const groups = useMemo(() => {
    if (!ix) return [];
    const out: { id: string; name: string; path: string; els: UiElement[] }[] = [];
    for (const [comp, els] of ix.uiByComponent) {
      const fe = ix.fns.get(comp);
      const path = fe?.file.path ?? ix.files.get(comp)?.file.path ?? comp;
      out.push({
        id: comp,
        name: fe?.fn.name ?? path.split("/").pop() ?? comp,
        path,
        els: [...els].sort((a, b) => a.line - b.line),
      });
    }
    return out.sort((a, b) => a.path.localeCompare(b.path) || a.name.localeCompare(b.name));
  }, [ix]);
  if (!ix) return null;
  if (groups.length === 0)
    return (
      <div className="ak-empty">
        <div className="text-lg font-semibold">No interactive UI found</div>
        <div className="ak-muted mt-1 max-w-md text-sm">
          This view lists buttons, inputs, forms and links in JSX/TSX components and follows their
          handlers into the code.
        </div>
      </div>
    );
  return (
    <>
      <div className={`ak-uimap ${inspect ? "ak-inspecting" : ""}`}>
        <div className="ak-uimap-grid">
          {groups.map((g, gi) => (
            <motion.div
              key={g.id}
              className={`ak-uicard ${selected === g.id ? "ak-node-selected" : ""}`}
              initial={{ opacity: 0, y: 14 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: gi * 0.05, type: "spring", stiffness: 240, damping: 26 }}
            >
              <div className="ak-uicard-bar">
                <span className="ak-dot" style={{ background: "#F87171" }} />
                <span className="ak-dot" style={{ background: "#FBBF24" }} />
                <span className="ak-dot" style={{ background: "#34D399" }} />
                <button type="button" className="ak-uicard-title" onClick={() => select(g.id)}>
                  &lt;{g.name} /&gt;
                </button>
                <span className="ak-muted ml-auto truncate text-[11px]">
                  {shortPath(g.path, 3)}
                </span>
              </div>
              <div className="ak-uicard-body">
                {g.els.map((el) => (
                  <Mock
                    key={el.id}
                    el={el}
                    active={selected === el.id}
                    hot={hover === el.id}
                    inspect={inspect}
                    onHover={(b) => setHover(b ? el.id : null)}
                    onPick={() => select(el.id)}
                  />
                ))}
              </div>
            </motion.div>
          ))}
        </div>
      </div>
      <TraceStrip />
    </>
  );
}
