import { AnimatePresence, motion } from "motion/react";
import { useLayoutEffect, useMemo, useRef, useState } from "react";
import type { UiElement } from "../gen";
import { shortPath, trace } from "../lib/graph";
import { serviceOfFile } from "../lib/system";
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

interface Dest {
  key: string;
  kind: "route" | "local" | "unwired";
  title: string;
  sub?: string;
  goesTo: string[];
  service?: string;
  run?: string;
  els: string[];
}

/** Where each element ends up: a backend route (and what it touches), or the browser. */
function destinations(ix: NonNullable<ReturnType<typeof useStore.getState>["ix"]>) {
  const out = new Map<string, Dest>();
  const of = new Map<string, string>();
  const dep = ix.g.insights?.deployment;
  for (const el of ix.ui.values()) {
    const hops = el.handler ? trace(ix, el.id) : [];
    const route = hops.find((h) => h.kind === "route");
    let d: Dest;
    if (route) {
      const after = hops.slice(hops.indexOf(route) + 1);
      const models = after.filter((h) => h.kind === "model").map((h) => h.label);
      const fns = after.filter((h) => h.kind === "fn").map((h) => h.label);
      const path = route.fileId ? ix.files.get(route.fileId)?.file.path : undefined;
      const svc = path && dep ? serviceOfFile(dep.services, path) : undefined;
      d = out.get(route.id) ?? {
        key: route.id,
        kind: "route",
        title: route.label,
        sub: `${fns[0] ?? "handler"} in ${path ?? "backend"}`,
        goesTo: [...new Set([...fns.slice(1, 4), ...models])],
        service: svc?.name,
        els: [],
      };
      const run = ix.g.exampleRuns?.find((r) => r.start === el.id);
      if (run) d.run = run.id;
    } else if (el.handler) {
      const k = `local:${el.handler}`;
      d = out.get(k) ?? {
        key: k,
        kind: "local",
        title: `${ix.label(el.handler)}()`,
        sub: "stays in the browser (no request found)",
        goesTo: hops
          .filter((h) => h.kind === "fn")
          .slice(1, 4)
          .map((h) => h.label),
        els: [],
      };
    } else {
      d = out.get("unwired") ?? {
        key: "unwired",
        kind: "unwired",
        title: "Not wired",
        sub: "no handler was found for these elements",
        goesTo: [],
        els: [],
      };
    }
    d.els.push(el.id);
    out.set(d.key, d);
    of.set(el.id, d.key);
  }
  const order = { route: 0, local: 1, unwired: 2 };
  return {
    list: [...out.values()].sort(
      (a, b) => order[a.kind] - order[b.kind] || a.title.localeCompare(b.title),
    ),
    of,
  };
}

interface Wire {
  el: string;
  dest: string;
  d: string;
}

export function UiMap() {
  const ix = useStore((s) => s.ix);
  const selected = useStore((s) => s.selected);
  const select = useStore((s) => s.select);
  const inspect = useStore((s) => s.inspect);
  const playRun = useStore((s) => s.playRun);
  const [hover, setHover] = useState<string | null>(null);
  const [hoverDest, setHoverDest] = useState<string | null>(null);
  const wrap = useRef<HTMLDivElement>(null);
  const [wires, setWires] = useState<Wire[]>([]);
  const [box, setBox] = useState({ w: 0, h: 0 });
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
  const dests = useMemo(() => (ix ? destinations(ix) : { list: [], of: new Map() }), [ix]);

  // Wires are measured from the DOM so they follow the real layout of both columns.
  useLayoutEffect(() => {
    const root = wrap.current;
    if (!root) return;
    const measure = () => {
      const r0 = root.getBoundingClientRect();
      const out: Wire[] = [];
      for (const [el, dest] of dests.of) {
        const a = root.querySelector(`[data-ui="${CSS.escape(el)}"]`);
        const b = root.querySelector(`[data-dest="${CSS.escape(dest)}"]`);
        if (!a || !b) continue;
        const ra = a.getBoundingClientRect();
        const rb = b.getBoundingClientRect();
        // Leave from the screen's edge at the element's height, so wires never cross a screen.
        const card = a.closest(".ak-uicard")?.getBoundingClientRect() ?? ra;
        const x1 = card.right - r0.left + root.scrollLeft;
        const y1 = ra.top + ra.height / 2 - r0.top + root.scrollTop;
        const x2 = rb.left - r0.left + root.scrollLeft;
        const y2 = rb.top + Math.min(rb.height / 2, 28) - r0.top + root.scrollTop;
        const mx = (x1 + x2) / 2;
        out.push({ el, dest, d: `M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}` });
      }
      setWires(out);
      setBox({ w: root.scrollWidth, h: root.scrollHeight });
    };
    measure();
    const t = setTimeout(measure, 450);
    const ro = new ResizeObserver(measure);
    ro.observe(root);
    return () => {
      clearTimeout(t);
      ro.disconnect();
    };
  }, [dests, groups]);

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
  const litDest = hoverDest ?? (hover ? dests.of.get(hover) : undefined);
  return (
    <>
      <div ref={wrap} className={`ak-uiflow ${inspect ? "ak-inspecting" : ""}`}>
        <svg className="ak-uiflow-wires" width={box.w} height={box.h} aria-hidden="true">
          {wires.map((w) => (
            <path
              key={w.el}
              d={w.d}
              className={`${w.dest === litDest ? "on" : ""} ${w.dest === "unwired" ? "dim" : ""}`}
            />
          ))}
        </svg>
        <div className="ak-uiflow-col">
          <div className="ak-uiflow-head">Screens</div>
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
                  <div key={el.id} data-ui={el.id} className="ak-uiflow-el">
                    <Mock
                      el={el}
                      active={selected === el.id}
                      hot={hover === el.id || (!!hoverDest && dests.of.get(el.id) === hoverDest)}
                      inspect={inspect || !!hoverDest}
                      onHover={(b) => setHover(b ? el.id : null)}
                      onPick={() => select(el.id)}
                    />
                  </div>
                ))}
              </div>
            </motion.div>
          ))}
        </div>
        <div className="ak-uiflow-col ak-uiflow-dests">
          <div className="ak-uiflow-head">Where it goes</div>
          {dests.list.map((d) => (
            // biome-ignore lint/a11y/noStaticElementInteractions: hover only highlights the wires
            <div
              key={d.key}
              data-dest={d.key}
              className={`ak-uidest ak-uidest-${d.kind} ${litDest === d.key ? "on" : ""}`}
              onMouseEnter={() => setHoverDest(d.key)}
              onMouseLeave={() => setHoverDest(null)}
            >
              <div className="flex items-baseline gap-2">
                <b className="ak-mono text-[13px]">{d.title}</b>
                {d.service && <span className="ak-chip ml-auto">{d.service} container</span>}
              </div>
              {d.sub && <div className="ak-muted mt-0.5 text-[11.5px]">{d.sub}</div>}
              {d.goesTo.length > 0 && (
                <div className="mt-1 flex flex-wrap gap-1">
                  {d.goesTo.map((x) => (
                    <span key={x} className="ak-ev ak-mono">
                      → {x}
                    </span>
                  ))}
                </div>
              )}
              <div className="mt-1.5 flex gap-2">
                {d.kind !== "unwired" && (
                  <button type="button" className="ak-link-btn" onClick={() => select(d.els[0])}>
                    trace it
                  </button>
                )}
                {d.run && (
                  <button
                    type="button"
                    className="ak-link-btn"
                    onClick={() => playRun(d.run ?? "")}
                  >
                    ▶ play run
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>
      <TraceStrip />
    </>
  );
}
