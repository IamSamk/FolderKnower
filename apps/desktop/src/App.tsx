import { AnimatePresence, motion } from "motion/react";
import { Analyzing } from "./components/Analyzing";
import { Inspector } from "./components/Inspector";
import { Palette } from "./components/Palette";
import { Lens, PortalScene } from "./components/Portal";
import { Setup } from "./components/Setup";
import { Sidebar } from "./components/Sidebar";
import { TopBar } from "./components/TopBar";
import { UiMap } from "./components/UiMap";
import { UpdateBanner } from "./components/UpdateBanner";
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
          <PortalScene sceneKey={view}>
            {view === "architecture" && <ArchitectureView />}
            {view === "er" && <ErView />}
            {view === "ui" && <UiMap />}
          </PortalScene>
          <Lens />
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
    <>
      <UpdateBanner />
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
          <Setup />
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
    </>
  );
}
