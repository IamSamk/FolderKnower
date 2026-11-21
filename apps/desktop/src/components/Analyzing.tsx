import { AnimatePresence, motion } from "motion/react";
import type { Progress } from "../lib/backend";
import { useStore } from "../store";

const STAGES: { key: Progress["stage"]; label: string }[] = [
  { key: "scan", label: "Searching" },
  { key: "parse", label: "Reading code" },
  { key: "link", label: "Connecting" },
  { key: "classify", label: "Understanding" },
  { key: "assemble", label: "Drawing" },
];

export function Analyzing() {
  const log = useStore((s) => s.log);
  const root = useStore((s) => s.root);
  const current = log[log.length - 1]?.stage ?? "scan";
  const idx = STAGES.findIndex((s) => s.key === current);
  return (
    <div className="ak-welcome">
      <div className="ak-aurora" />
      <div className="relative z-10 w-[640px] max-w-[92vw]">
        <div className="ak-muted mb-1 text-[12px] tracking-wide uppercase">Analyzing</div>
        <div className="ak-mono mb-6 truncate text-[15px]">
          {root?.startsWith("demo:") ? "demo / fullstack-todo" : root}
        </div>
        <div className="mb-6 flex gap-2">
          {STAGES.map((s, i) => (
            <div key={s.key} className="flex-1">
              <div className="ak-stagebar">
                <motion.div
                  className="ak-stagebar-fill"
                  initial={{ width: 0 }}
                  animate={{
                    width: i < idx || current === "done" ? "100%" : i === idx ? "60%" : "0%",
                  }}
                  transition={{ duration: 0.5 }}
                />
              </div>
              <div
                className={`mt-1.5 text-[11.5px] ${i <= idx || current === "done" ? "" : "ak-muted"}`}
              >
                {s.label}
              </div>
            </div>
          ))}
        </div>
        <div className="ak-narration">
          <AnimatePresence initial={false}>
            {log.slice(-9).map((l) => (
              <motion.div
                key={l.id}
                layout
                initial={{ opacity: 0, y: 10, filter: "blur(4px)" }}
                animate={{ opacity: 1, y: 0, filter: "blur(0px)" }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.35 }}
                className="ak-narration-line"
              >
                <span className="ak-narration-stage">{l.stage}</span>
                <span>{l.text}</span>
              </motion.div>
            ))}
          </AnimatePresence>
          <div className="ak-caret" />
        </div>
      </div>
    </div>
  );
}
