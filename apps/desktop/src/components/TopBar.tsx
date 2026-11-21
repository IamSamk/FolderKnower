import {
  Boxes,
  Database,
  Download,
  FolderOpen,
  MousePointer2,
  Network,
  Search,
} from "lucide-react";
import { motion } from "motion/react";
import { exportJson } from "../lib/backend";
import { TIER_COLOR } from "../lib/theme";
import { type Focus, useStore, type View } from "../store";

function crumb(f: Focus, label: (id: string) => string): { text: string; color?: string } {
  switch (f.level) {
    case "system":
      return { text: "System" };
    case "tier":
      return { text: f.tier, color: TIER_COLOR[f.tier] };
    case "cluster":
      return { text: f.clusterId.split(":").pop() ?? f.clusterId };
    case "file":
      return { text: label(f.fileId) };
    case "function":
      return { text: `${label(f.fnId)}()` };
  }
}

const LEVEL_NAMES = ["System tiers", "Subsystems", "Files", "Functions & calls", "Function detail"];

export function TopBar() {
  const ix = useStore((s) => s.ix);
  const view = useStore((s) => s.view);
  const setView = useStore((s) => s.setView);
  const stack = useStore((s) => s.stack);
  const back = useStore((s) => s.back);
  const inspect = useStore((s) => s.inspect);
  const setInspect = useStore((s) => s.setInspect);
  const reset = useStore((s) => s.reset);
  const setPalette = useStore((s) => s.setPalette);
  if (!ix) return null;
  const views: { k: View; label: string; icon: React.ReactNode }[] = [
    { k: "architecture", label: "Architecture", icon: <Network size={14} /> },
    { k: "ui", label: "UI map", icon: <Boxes size={14} /> },
    { k: "er", label: "Data (ER)", icon: <Database size={14} /> },
  ];
  return (
    <header className="ak-topbar">
      <button type="button" className="ak-brand" onClick={reset} title="Open another folder">
        <img src="/icon.png" alt="" className="h-6 w-6" />
        <span>{ix.g.repoName}</span>
        <FolderOpen size={13} className="ak-muted" />
      </button>
      <div className="ak-seg">
        {views.map((v) => (
          <button
            key={v.k}
            type="button"
            className={`ak-seg-btn ${view === v.k ? "ak-seg-on" : ""}`}
            onClick={() => setView(v.k)}
          >
            {view === v.k && (
              <motion.span
                layoutId="seg"
                className="ak-seg-bg"
                transition={{ type: "spring", stiffness: 380, damping: 30 }}
              />
            )}
            <span className="relative z-10 flex items-center gap-1.5">
              {v.icon}
              {v.label}
            </span>
          </button>
        ))}
      </div>
      {view === "architecture" && (
        <nav className="ak-crumbs">
          {stack.map((f, i) => {
            const c = crumb(f, ix.label);
            return (
              <motion.button
                key={`${i}-${c.text}`}
                type="button"
                initial={{ opacity: 0, x: -6 }}
                animate={{ opacity: 1, x: 0 }}
                className={`ak-crumb ${i === stack.length - 1 ? "ak-crumb-on" : ""}`}
                onClick={() => back(i)}
                style={c.color ? ({ "--c": c.color } as React.CSSProperties) : undefined}
              >
                {i > 0 && <span className="ak-muted mr-1.5">/</span>}
                {c.text}
              </motion.button>
            );
          })}
          <span className="ak-level">
            L{stack.length} · {LEVEL_NAMES[stack.length - 1]}
          </span>
        </nav>
      )}
      <div className="ml-auto flex items-center gap-2">
        <button
          type="button"
          className="ak-iconbtn"
          onClick={() => setPalette(true)}
          title="Search (Ctrl+K)"
        >
          <Search size={15} />
          <span className="ak-kbd">Ctrl K</span>
        </button>
        <button
          type="button"
          className={`ak-iconbtn ${inspect ? "ak-iconbtn-on" : ""}`}
          title="Inspect pointer: pick a UI element and follow it"
          onClick={() => {
            setInspect(!inspect);
            if (!inspect) setView("ui");
          }}
        >
          <MousePointer2 size={15} />
        </button>
        <button
          type="button"
          className="ak-iconbtn"
          title="Export architecture.json"
          onClick={() => void exportJson(ix.g)}
        >
          <Download size={15} />
        </button>
      </div>
    </header>
  );
}
