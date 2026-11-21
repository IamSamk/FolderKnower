import { AnimatePresence, motion } from "motion/react";
import { Analyzing } from "./components/Analyzing";
import { Inspector } from "./components/Inspector";
import { Palette } from "./components/Palette";
import { Sidebar } from "./components/Sidebar";
import { TopBar } from "./components/TopBar";
import { UiMap } from "./components/UiMap";
import { ArchitectureView, ErView } from "./components/Views";
import { Welcome } from "./components/Welcome";
import { useStore } from "./store";

function Workspace() {
  const view = useStore((s) => s.view);
  return (
    <div className="ak-shell">
      <TopBar />
      <div className="ak-body">
        <Sidebar />
        <main className="ak-main">
          <AnimatePresence mode="wait">
            <motion.div
              key={view}
              className="absolute inset-0"
              initial={{ opacity: 0, scale: 0.985, filter: "blur(6px)" }}
              animate={{ opacity: 1, scale: 1, filter: "blur(0px)" }}
              exit={{ opacity: 0, scale: 1.01, filter: "blur(6px)" }}
              transition={{ duration: 0.32, ease: [0.4, 0, 0.2, 1] }}
            >
              {view === "architecture" && <ArchitectureView />}
              {view === "er" && <ErView />}
              {view === "ui" && <UiMap />}
            </motion.div>
          </AnimatePresence>
        </main>
        <Inspector />
      </div>
      <Palette />
    </div>
  );
}

export default function App() {
  const phase = useStore((s) => s.phase);
  const error = useStore((s) => s.error);
  const reset = useStore((s) => s.reset);
  return (
    <AnimatePresence mode="wait">
      <motion.div
        key={phase}
        className="h-full w-full"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        transition={{ duration: 0.35 }}
      >
        {phase === "welcome" && <Welcome />}
        {phase === "analyzing" && <Analyzing />}
        {phase === "ready" && <Workspace />}
        {phase === "error" && (
          <div className="ak-welcome">
            <div className="relative z-10 max-w-lg text-center">
              <div className="text-lg font-semibold">Analysis failed</div>
              <div className="ak-muted ak-mono mt-2 text-sm break-all">{error}</div>
              <button type="button" className="ak-btn mt-4" onClick={reset}>
                Back
              </button>
            </div>
          </div>
        )}
      </motion.div>
    </AnimatePresence>
  );
}
