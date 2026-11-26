// Problems drawer: what stops this code from running as written, found without
// running it. Grouped by kind, each with the line and a one-sentence fix. The rest of
// the app keeps working on broken code; this says where the gaps are.

import { Cpu, FileWarning, PackageX, Route, Unplug, X } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect } from "react";
import type { HealthIssue } from "../../gen";
import { useStore } from "../../store";

const KIND: Record<HealthIssue["kind"], { title: string; icon: React.ReactNode; why: string }> = {
  syntaxError: {
    title: "Syntax errors",
    icon: <FileWarning size={14} />,
    why: "The file will not run. The map still shows what could be read around the error.",
  },
  unresolvedImport: {
    title: "Imports of missing files",
    icon: <Unplug size={14} />,
    why: "The import fails at start-up, so everything that uses this file fails too.",
  },
  missingEntry: {
    title: "Start commands that name a missing file",
    icon: <Unplug size={14} />,
    why: "The container starts and exits at once.",
  },
  missingPackage: {
    title: "Packages missing from the manifest",
    icon: <PackageX size={14} />,
    why: "Works on the machine that has it installed, breaks on a fresh install or in Docker.",
  },
  noGpu: {
    title: "GPU code without a GPU",
    icon: <Cpu size={14} />,
    why: "The model fails to load (or crawls on the CPU) because the container cannot see a GPU.",
  },
  deadRoute: {
    title: "Calls to routes that do not exist",
    icon: <Route size={14} />,
    why: "The request reaches the backend and gets a 404.",
  },
};

const ORDER: HealthIssue["kind"][] = [
  "syntaxError",
  "unresolvedImport",
  "missingEntry",
  "noGpu",
  "noGpu",
  "missingPackage",
  "deadRoute",
];

/** `code` spans in a sentence, without a Markdown parser. */
function Ticks({ text }: { text: string }) {
  return (
    <>
      {text.split("`").map((part, i) =>
        i % 2 ? (
          <code key={i} className="ak-tick">
            {part}
          </code>
        ) : (
          part
        ),
      )}
    </>
  );
}

export function HealthDrawer() {
  const open = useStore((s) => s.healthOpen);
  const close = useStore((s) => s.closeHealth);
  const ix = useStore((s) => s.ix);
  const reveal = useStore((s) => s.reveal);
  useEffect(() => {
    if (!open) return;
    const k = (e: KeyboardEvent) => e.key === "Escape" && close();
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  }, [open, close]);
  const h = ix?.g.health;
  return (
    <AnimatePresence>
      {open && h && ix && (
        <motion.aside
          className="ak-tools"
          initial={{ opacity: 0, x: 40 }}
          animate={{ opacity: 1, x: 0 }}
          exit={{ opacity: 0, x: 40 }}
          transition={{ type: "spring", stiffness: 360, damping: 34 }}
        >
          <div className="ak-tools-top">
            <div>
              <div className="text-[15px] font-bold">Problems</div>
              <div className="ak-muted text-[12px]">
                {h.issues.length} found in {h.filesChecked} files without running anything
                {h.filesWithErrors ? `; ${h.filesWithErrors} will not parse` : ""}.
              </div>
            </div>
            <button type="button" className="ak-iconbtn ml-auto" onClick={close} aria-label="Close">
              <X size={15} />
            </button>
          </div>
          <div className="ak-tools-body">
            {ORDER.map((k) => {
              const list = h.issues.filter((i) => i.kind === k);
              if (!list.length) return null;
              return (
                <div key={k} className="ak-tool-card">
                  <div className="flex items-center gap-1.5">
                    {KIND[k].icon}
                    <b>{KIND[k].title}</b>
                    <span className="ak-count ml-auto">{list.length}</span>
                  </div>
                  <p className="ak-muted">{KIND[k].why}</p>
                  {list.map((i) => (
                    <button
                      key={i.id}
                      type="button"
                      className={`ak-health-row ${i.severity}`}
                      onClick={() => reveal(i.fileId)}
                      title="Open in the code view"
                    >
                      <span className="ak-mono ak-muted text-[11.5px]">
                        {i.fileId.replace(/^file:/, "")}:{i.line}
                      </span>
                      <span>
                        <Ticks text={i.message} />
                      </span>
                      <span className="ak-muted text-[12px]">
                        <Ticks text={i.fix} />
                      </span>
                    </button>
                  ))}
                </div>
              );
            })}
          </div>
        </motion.aside>
      )}
    </AnimatePresence>
  );
}
