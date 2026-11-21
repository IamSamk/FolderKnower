import {
  Boxes,
  Database,
  Download,
  Laptop,
  Moon,
  MousePointer2,
  Network,
  Search,
  Sparkles,
  Sun,
} from "lucide-react";
import { motion } from "motion/react";
import { exportJson } from "../lib/backend";
import { TIER_COLOR } from "../lib/theme";
import { type ThemePref, useTheme } from "../lib/themeMode";
import { type Focus, useStore, type View } from "../store";
import { originOf } from "./Portal";

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

const LEVEL_NAMES = ["System tiers", "Subsystems", "Files", "Functions and calls", "One function"];

export function ThemeSwitch() {
  const pref = useTheme((s) => s.pref);
  const setPref = useTheme((s) => s.setPref);
  const opts: { k: ThemePref; icon: React.ReactNode; label: string }[] = [
    { k: "light", icon: <Sun size={14} strokeWidth={2.4} />, label: "Light" },
    { k: "system", icon: <Laptop size={14} strokeWidth={2.4} />, label: "Match system" },
    { k: "dark", icon: <Moon size={14} strokeWidth={2.4} />, label: "Dark" },
  ];
  return (
    <div className="ak-themesw" role="radiogroup" aria-label="Theme">
      {opts.map((o) => (
        <button
          key={o.k}
          type="button"
          role="radio"
          aria-checked={pref === o.k}
          title={o.label}
          className={pref === o.k ? "on" : ""}
          onClick={() => setPref(o.k)}
        >
          {pref === o.k && (
            <motion.span
              layoutId="themepill"
              className="ak-themesw-pill"
              transition={{ type: "spring", stiffness: 500, damping: 34 }}
            />
          )}
          <span className="relative">{o.icon}</span>
        </button>
      ))}
    </div>
  );
}

export function TopBar() {
  const ix = useStore((s) => s.ix);
  const view = useStore((s) => s.view);
  const stack = useStore((s) => s.stack);
  const inspect = useStore((s) => s.inspect);
  const setInspect = useStore((s) => s.setInspect);
  const reset = useStore((s) => s.reset);
  const setPalette = useStore((s) => s.setPalette);
  const go = useStore((s) => s.go);
  if (!ix) return null;
  const views: { k: View; label: string; icon: React.ReactNode }[] = [
    { k: "architecture", label: "Architecture", icon: <Network size={15} strokeWidth={2.2} /> },
    { k: "ui", label: "UI map", icon: <Boxes size={15} strokeWidth={2.2} /> },
    { k: "er", label: "Data", icon: <Database size={15} strokeWidth={2.2} /> },
  ];
  const switchView = (k: View, el: Element) => {
    if (k === view) return;
    go(originOf(el), "tab", () => useStore.setState({ view: k }));
  };
  const back = (i: number) => {
    if (i === stack.length - 1) return;
    const main = document.querySelector(".ak-main")?.getBoundingClientRect();
    go({ x: (main?.width ?? 800) / 2, y: (main?.height ?? 600) / 2 }, "out", () =>
      useStore.setState({ stack: stack.slice(0, i + 1) }),
    );
  };
  return (
    <header className="ak-topbar">
      <button type="button" className="ak-brand" onClick={reset} title="Open another folder">
        <img src="/icon.png" alt="" />
        <span>{ix.g.repoName}</span>
      </button>
      <nav className="ak-tabs-main" aria-label="Views">
        {views.map((v) => (
          <button
            key={v.k}
            type="button"
            className={view === v.k ? "on" : ""}
            onClick={(e) => switchView(v.k, e.currentTarget)}
          >
            {view === v.k && (
              <motion.span
                layoutId="viewpill"
                className="ak-tabpill"
                transition={{ type: "spring", stiffness: 480, damping: 34 }}
              />
            )}
            <span className="relative flex items-center gap-1.5">
              {v.icon}
              {v.label}
            </span>
          </button>
        ))}
      </nav>
      {view === "architecture" && (
        <nav className="ak-crumbs" aria-label="Zoom path">
          {stack.map((f, i) => {
            const c = crumb(f, ix.label);
            return (
              <motion.button
                key={`${i}-${c.text}`}
                type="button"
                initial={{ opacity: 0, y: -6 }}
                animate={{ opacity: 1, y: 0 }}
                className={`ak-crumb ${i === stack.length - 1 ? "on" : ""}`}
                onClick={() => back(i)}
                style={c.color ? ({ "--c": c.color } as React.CSSProperties) : undefined}
              >
                {c.text}
              </motion.button>
            );
          })}
          <span className="ak-level" title="Zoom level">
            {LEVEL_NAMES[stack.length - 1]}
          </span>
        </nav>
      )}
      <div className="ml-auto flex items-center gap-2">
        <button
          type="button"
          className="ak-tool"
          onClick={() => setPalette(true)}
          title="Jump to anything (Ctrl K)"
        >
          <Search size={15} strokeWidth={2.4} />
          <kbd>Ctrl K</kbd>
        </button>
        <button
          type="button"
          className={`ak-tool ${inspect ? "on" : ""}`}
          title="Pointer: pick a UI element and follow it"
          aria-pressed={inspect}
          onClick={(e) => {
            setInspect(!inspect);
            if (!inspect && view !== "ui") switchView("ui", e.currentTarget);
          }}
        >
          <MousePointer2 size={15} strokeWidth={2.4} />
        </button>
        <button
          type="button"
          className="ak-tool"
          title="Local AI models"
          onClick={() => useStore.setState({ setupOpen: true })}
        >
          <Sparkles size={15} strokeWidth={2.4} />
        </button>
        <button
          type="button"
          className="ak-tool"
          title="Export architecture.json"
          onClick={() => void exportJson(ix.g)}
        >
          <Download size={15} strokeWidth={2.4} />
        </button>
        <ThemeSwitch />
      </div>
    </header>
  );
}
