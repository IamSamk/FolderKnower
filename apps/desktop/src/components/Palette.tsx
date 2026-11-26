import { AnimatePresence, motion } from "motion/react";
import { useEffect, useMemo, useState } from "react";
import { TIER_COLOR } from "../lib/theme";
import { useStore } from "../store";

interface Item {
  id: string;
  label: string;
  sub: string;
  kind: string;
  color: string;
}

export function Palette() {
  const open = useStore((s) => s.paletteOpen);
  const setOpen = useStore((s) => s.setPalette);
  const ix = useStore((s) => s.ix);
  const reveal = useStore((s) => s.reveal);
  const [q, setQ] = useState("");
  const [cur, setCur] = useState(0);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setOpen(!useStore.getState().paletteOpen);
      }
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setOpen]);

  const all = useMemo<Item[]>(() => {
    if (!ix) return [];
    const out: Item[] = [];
    for (const [, f] of ix.files)
      out.push({
        id: f.file.id,
        label: f.file.path.split("/").pop() ?? "",
        sub: f.file.path,
        kind: "file",
        color: TIER_COLOR[f.tier],
      });
    for (const [, f] of ix.fns)
      out.push({
        id: f.fn.id,
        label: `${f.fn.name}()`,
        sub: f.file.path,
        kind: "fn",
        color: TIER_COLOR[f.tier],
      });
    for (const m of ix.g.dataModels)
      out.push({
        id: m.id,
        label: m.name,
        sub: `${m.framework} ${m.kind}`,
        kind: "model",
        color: "oklch(69.6% 0.149 162)",
      });
    for (const r of ix.g.routes)
      out.push({
        id: r.handler ?? r.id,
        label: `${r.method} ${r.path}`,
        sub: r.fileId.slice(5),
        kind: "route",
        color: "oklch(79.7% 0.134 212)",
      });
    return out;
  }, [ix]);

  const hits = useMemo(() => {
    const s = q.trim().toLowerCase();
    if (!s) return all.slice(0, 12);
    return all
      .map((i) => {
        const l = i.label.toLowerCase();
        const score = l.startsWith(s)
          ? 0
          : l.includes(s)
            ? 1
            : i.sub.toLowerCase().includes(s)
              ? 2
              : 9;
        return { i, score };
      })
      .filter((x) => x.score < 9)
      .sort((a, b) => a.score - b.score || a.i.label.length - b.i.label.length)
      .slice(0, 14)
      .map((x) => x.i);
  }, [q, all]);

  const go = (it: Item) => {
    setOpen(false);
    setQ("");
    reveal(it.id);
  };

  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className="ak-palette-wrap"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          onClick={() => setOpen(false)}
        >
          <motion.div
            className="ak-palette"
            initial={{ y: -16, scale: 0.98, opacity: 0 }}
            animate={{ y: 0, scale: 1, opacity: 1 }}
            exit={{ y: -10, opacity: 0 }}
            transition={{ type: "spring", stiffness: 380, damping: 32 }}
            onClick={(e) => e.stopPropagation()}
          >
            <input
              // biome-ignore lint/a11y/noAutofocus: command palette
              autoFocus
              className="ak-palette-input"
              placeholder="Jump to a file, function, route or table…"
              value={q}
              onChange={(e) => {
                setQ(e.target.value);
                setCur(0);
              }}
              onKeyDown={(e) => {
                if (e.key === "ArrowDown") setCur((c) => Math.min(c + 1, hits.length - 1));
                if (e.key === "ArrowUp") setCur((c) => Math.max(c - 1, 0));
                if (e.key === "Enter" && hits[cur]) go(hits[cur]);
              }}
            />
            <ul className="max-h-[50vh] overflow-auto p-1.5">
              {hits.map((h, i) => (
                <li key={h.id + h.kind}>
                  <button
                    type="button"
                    className={`ak-palette-item ${i === cur ? "ak-palette-on" : ""}`}
                    onMouseEnter={() => setCur(i)}
                    onClick={() => go(h)}
                  >
                    <span className="ak-ref-dot" style={{ background: h.color }} />
                    <span className="ak-mono">{h.label}</span>
                    <span className="ak-muted ml-auto truncate pl-3 text-[11px]">{h.sub}</span>
                    <span className="ak-kind">{h.kind}</span>
                  </button>
                </li>
              ))}
            </ul>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
