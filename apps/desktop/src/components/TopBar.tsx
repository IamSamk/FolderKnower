import {
  Activity,
  Boxes,
  Brain,
  Container,
  Database,
  Download,
  Laptop,
  Moon,
  MousePointer2,
  Network,
  RefreshCw,
  Route,
  Search,
  Sparkles,
  Sun,
  TriangleAlert,
  Wrench,
} from "lucide-react";
import { motion } from "motion/react";
import { exportJson } from "../lib/backend";
import { TIER_COLOR } from "../lib/theme";
import { type ThemePref, useTheme } from "../lib/themeMode";
import { type Focus, useStore, type View } from "../store";
import { originOf } from "./Portal";
import { requestUpdateCheck } from "./UpdatePopup";

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
  const workflowOpen = useStore((s) => s.workflowOpen);
  const openWorkflow = useStore((s) => s.openWorkflow);
  const closeWorkflow = useStore((s) => s.closeWorkflow);
  const toolsOpen = useStore((s) => s.toolsOpen);
  const toggleTools = useStore((s) => s.toggleTools);
  const toggleHealth = useStore((s) => s.toggleHealth);
  const healthOpen = useStore((s) => s.healthOpen);
  const health = useStore((s) => s.ix?.g.health);
  const errors = health?.issues.filter((i) => i.severity === "error").length ?? 0;
  if (!ix) return null;
  const top = stack[stack.length - 1];
  const views: { k: View; label: string; icon: React.ReactNode }[] = [
    { k: "system", label: "System", icon: <Container size={15} strokeWidth={2.2} /> },
    { k: "architecture", label: "Code map", icon: <Network size={15} strokeWidth={2.2} /> },
    { k: "ui", label: "UI map", icon: <Boxes size={15} strokeWidth={2.2} /> },
    { k: "er", label: "Data", icon: <Database size={15} strokeWidth={2.2} /> },
    { k: "insights", label: "How it works", icon: <Brain size={15} strokeWidth={2.2} /> },
    { k: "live", label: "Live", icon: <Activity size={15} strokeWidth={2.2} /> },
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
        {health && (
          <button
            type="button"
            className={`ak-tool ak-tool-label ak-problems ${errors ? "err" : "warn"} ${healthOpen ? "on" : ""}`}
            aria-pressed={healthOpen}
            onClick={toggleHealth}
            title="Problems found without running the code: syntax errors, missing files and packages, dead routes"
          >
            <TriangleAlert size={15} strokeWidth={2.3} />
            {health.issues.length} problem{health.issues.length === 1 ? "" : "s"}
          </button>
        )}
        <button
          type="button"
          className={`ak-tool ak-tool-label ${toolsOpen ? "on" : ""}`}
          aria-pressed={toolsOpen}
          onClick={toggleTools}
          title="DSA tools: structures in your code, hidden in your tools, and real-life problems"
        >
          <Wrench size={15} strokeWidth={2.2} />
          DSA tools
        </button>
        {view !== "live" && view !== "system" && (
          <button
            type="button"
            className={`ak-tool ak-tool-label ${workflowOpen ? "on" : ""}`}
            title="Workflow: the clean path through the code at this zoom level"
            aria-pressed={workflowOpen}
            onClick={() =>
              workflowOpen
                ? closeWorkflow()
                : openWorkflow(
                    view === "architecture" && top.level === "function" ? top.fnId : null,
                  )
            }
          >
            <Route size={15} strokeWidth={2.4} />
            Workflow
          </button>
        )}
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
          title="Settings: local AI models"
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
        <button
          type="button"
          className="ak-tool"
          title="Check for updates"
          onClick={requestUpdateCheck}
        >
          <RefreshCw size={15} strokeWidth={2.4} />
        </button>
        <ThemeSwitch />
      </div>
    </header>
  );
}
