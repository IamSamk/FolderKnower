import { AnimatePresence, motion } from "motion/react";
import type { Hop } from "../lib/graph";
import { TIER_COLOR } from "../lib/theme";
import { useStore } from "../store";

const VIA: Record<NonNullable<Hop["via"]>, string> = {
  handler: "on event",
  call: "calls",
  http: "HTTP",
  sql: "SQL",
};
const KIND_ICON: Record<Hop["kind"], string> = { ui: "◉", fn: "ƒ", route: "⇥", model: "▤" };

function HopCard({ h, i, onClick }: { h: Hop; i: number; onClick: () => void }) {
  const c = h.tier ? TIER_COLOR[h.tier] : "#94A3B8";
  return (
    <motion.button
      type="button"
      onClick={onClick}
      className="ak-hop"
      style={{ "--c": c } as React.CSSProperties}
      initial={{ opacity: 0, x: -12, scale: 0.96 }}
      animate={{ opacity: 1, x: 0, scale: 1 }}
      transition={{ delay: 0.12 + i * 0.22, type: "spring", stiffness: 260, damping: 24 }}
    >
      <div className="flex items-center gap-1.5">
        <span className="ak-hop-icon">{KIND_ICON[h.kind]}</span>
        <span className="ak-mono truncate text-[12.5px] font-semibold">{h.label}</span>
      </div>
      <div className="ak-muted truncate text-[10.5px]">
        {h.tier ?? ""} {h.detail ? `· ${h.detail}` : ""}
      </div>
    </motion.button>
  );
}

function Link({ via, i, color }: { via?: Hop["via"]; i: number; color: string }) {
  return (
    <motion.div
      className="ak-hop-link"
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ delay: 0.05 + i * 0.22 }}
    >
      <div className="ak-hop-line" style={{ background: color }}>
        <motion.span
          className="ak-hop-comet"
          initial={{ left: "0%" }}
          animate={{ left: "100%" }}
          transition={{
            delay: 0.05 + i * 0.22,
            duration: 0.5,
            repeat: Number.POSITIVE_INFINITY,
            repeatDelay: 1.6,
            ease: "easeInOut",
          }}
        />
      </div>
      <span className="ak-hop-via" style={{ color }}>
        {via ? VIA[via] : ""}
      </span>
    </motion.div>
  );
}

export function TraceStrip() {
  const hops = useStore((s) => s.hops);
  const reveal = useStore((s) => s.reveal);
  const selected = useStore((s) => s.selected);
  return (
    <AnimatePresence>
      {hops.length > 0 && (
        <motion.div
          key={selected ?? "trace"}
          className="ak-trace"
          initial={{ opacity: 0, y: 24 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: 24 }}
          transition={{ type: "spring", stiffness: 220, damping: 26 }}
        >
          <div className="ak-trace-title">Trace · how this travels through the system</div>
          <div className="ak-trace-row">
            {hops.map((h, i) => (
              <div key={`${h.id}-${i}`} className="flex items-center">
                {i > 0 && (
                  <Link
                    via={h.via}
                    i={i}
                    color={h.via === "http" ? "#22D3EE" : h.via === "sql" ? "#34D399" : "#71717A"}
                  />
                )}
                <HopCard h={h} i={i} onClick={() => reveal(h.id)} />
              </div>
            ))}
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
