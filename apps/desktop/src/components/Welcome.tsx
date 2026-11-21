import { FolderOpen, Sparkles } from "lucide-react";
import { motion } from "motion/react";
import { useEffect, useState } from "react";
import { DEMO_ROOT, isTauri, onFolderDrop, pickFolder } from "../lib/backend";
import { TIER_COLOR } from "../lib/theme";
import { useStore } from "../store";

function Logo() {
  const tiers = [TIER_COLOR.Frontend, TIER_COLOR.Backend, TIER_COLOR.Storage];
  return (
    <svg viewBox="0 0 200 170" className="h-36 w-44" aria-hidden>
      <title>Architekton</title>
      {tiers.map((c, i) => (
        <motion.polygon
          key={c}
          points={`100,${20 + i * 40} 180,${50 + i * 40} 100,${80 + i * 40} 20,${50 + i * 40}`}
          fill={c}
          fillOpacity={0.85}
          stroke="white"
          strokeOpacity={0.6}
          strokeWidth={1.5}
          initial={{ opacity: 0, y: -18 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.15 + (2 - i) * 0.14, type: "spring", stiffness: 160, damping: 14 }}
        />
      ))}
      <motion.line
        x1="100"
        y1="22"
        x2="100"
        y2="140"
        stroke="white"
        strokeWidth="3"
        strokeLinecap="round"
        initial={{ pathLength: 0 }}
        animate={{ pathLength: 1 }}
        transition={{ delay: 0.7, duration: 0.7 }}
      />
      <motion.circle
        cx="100"
        cy="142"
        r="6"
        fill="white"
        initial={{ scale: 0 }}
        animate={{ scale: 1 }}
        transition={{ delay: 1.3 }}
      />
    </svg>
  );
}

export function Welcome() {
  const start = useStore((s) => s.start);
  const [hover, setHover] = useState(false);
  useEffect(() => {
    let un: (() => void) | undefined;
    void onFolderDrop((p) => void start(p), setHover).then((u) => {
      un = u;
    });
    return () => un?.();
  }, [start]);

  return (
    <div className="ak-welcome">
      <div className="ak-aurora" />
      <motion.div
        initial={{ opacity: 0, y: 16 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.6 }}
        className="relative z-10 flex flex-col items-center"
      >
        <Logo />
        <h1 className="ak-hero mt-2">Architekton</h1>
        <p className="ak-muted mt-2 max-w-xl text-center text-[15px] leading-relaxed">
          Drop a repository. Architekton reads it offline, works out what each part <em>does</em>{" "}
          (frontend, gateway, backend, workers, data) and draws how a click travels all the way to
          the database.
        </p>
        <motion.button
          type="button"
          className={`ak-drop ${hover ? "ak-drop-hover" : ""}`}
          onClick={async () => {
            const p = await pickFolder();
            if (p) void start(p);
          }}
          whileHover={{ scale: 1.015 }}
          whileTap={{ scale: 0.99 }}
        >
          <FolderOpen size={30} strokeWidth={1.6} />
          <div className="mt-3 text-[16px] font-semibold">
            {hover
              ? "Release to analyze"
              : isTauri
                ? "Drop a folder here, or click to choose"
                : "Open the demo project"}
          </div>
          <div className="ak-muted mt-1 text-[12.5px]">
            TypeScript · JavaScript · Python · Rust · Go — nothing leaves your machine
          </div>
        </motion.button>
        {isTauri && (
          <button type="button" className="ak-btn mt-4" onClick={() => void start(DEMO_ROOT)}>
            <Sparkles size={14} /> Explore the demo full-stack app
          </button>
        )}
      </motion.div>
    </div>
  );
}
