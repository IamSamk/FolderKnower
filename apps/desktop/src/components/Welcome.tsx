import { FolderOpen, Sparkles } from "lucide-react";
import { motion, useReducedMotion } from "motion/react";
import { useEffect, useState } from "react";
import {
  checkIsTauri,
  DEMO_AI_ROOT,
  DEMO_BROKEN_ROOT,
  DEMO_ROOT,
  DEMO_SELF_ROOT,
  isTauri,
  onFolderDrop,
  pickFolder,
} from "../lib/backend";
import { TIER_COLOR } from "../lib/theme";
import { useStore } from "../store";
import { ThemeSwitch } from "./TopBar";

const LANGS: { name: string; color: string }[] = [
  { name: "TypeScript", color: TIER_COLOR.Frontend },
  { name: "JavaScript", color: TIER_COLOR.Gateway },
  { name: "Python", color: TIER_COLOR.Backend },
  { name: "Rust", color: TIER_COLOR.Workers },
  { name: "Go", color: TIER_COLOR.Storage },
];

/** Three tiers stacking into a building: the one orchestrated moment on this screen. */
function Stack() {
  const reduce = useReducedMotion();
  const slabs = [
    { c: TIER_COLOR.Frontend, label: "Frontend" },
    { c: TIER_COLOR.Backend, label: "Backend" },
    { c: TIER_COLOR.Storage, label: "Data" },
  ];
  return (
    <div className="relative mb-6 flex flex-col items-center gap-[10px]" aria-hidden>
      {slabs.map((s, i) => (
        <motion.div
          key={s.label}
          initial={reduce ? false : { y: -160, rotate: i % 2 ? 7 : -7, opacity: 0 }}
          animate={{ y: 0, rotate: 0, opacity: 1 }}
          transition={{ delay: 0.15 + (2 - i) * 0.16, type: "spring", stiffness: 220, damping: 15 }}
          style={{
            width: 210 - i * 0,
            height: 44,
            background: s.c,
            border: "2.5px solid var(--line)",
            borderRadius: 12,
            boxShadow: "5px 5px 0 var(--shade)",
            display: "grid",
            placeItems: "center",
            fontWeight: 750,
            color: "#1f1d36",
            fontSize: 15,
            position: "relative",
            zIndex: 1,
          }}
        >
          {s.label}
        </motion.div>
      ))}
      <motion.div
        className="absolute"
        style={{
          left: "50%",
          top: -14,
          bottom: -10,
          width: 4,
          marginLeft: -2,
          background: "var(--line)",
          borderRadius: 4,
          transformOrigin: "top",
          zIndex: 0,
        }}
        initial={reduce ? false : { scaleY: 0 }}
        animate={{ scaleY: 1 }}
        transition={{ delay: 0.9, duration: 0.5, ease: [0.65, 0, 0.35, 1] }}
      />
      <motion.div
        className="absolute"
        style={{
          zIndex: 2,
          bottom: -20,
          left: "50%",
          marginLeft: -9,
          width: 18,
          height: 18,
          borderRadius: 99,
          background: "var(--mark)",
          border: "2.5px solid var(--line)",
        }}
        initial={reduce ? false : { scale: 0 }}
        animate={{ scale: 1 }}
        transition={{ delay: 1.35, type: "spring", stiffness: 400, damping: 12 }}
      />
    </div>
  );
}

export function Welcome() {
  const start = useStore((s) => s.start);
  const [hover, setHover] = useState(false);
  const inTauri = isTauri || checkIsTauri();
  useEffect(() => {
    let un: (() => void) | undefined;
    void onFolderDrop((p) => void start(p), setHover).then((u) => {
      un = u;
    });
    return () => un?.();
  }, [start]);

  return (
    <div className="ak-welcome">
      <div className="absolute top-4 right-4 z-20 flex items-center gap-2">
        <button
          type="button"
          className="ak-btn ghost"
          onClick={() => useStore.setState({ setupOpen: true })}
        >
          <Sparkles size={15} /> Local AI
        </button>
        <ThemeSwitch />
      </div>
      <div className="relative z-10 flex flex-col items-center px-6">
        <Stack />
        <h1 className="ak-hero">Architekton</h1>
        <p className="ak-hero-sub mt-4">
          Drop in a repository and see how it is really built: what is frontend, what is backend,
          which button calls which API, and which table it ends up in.
        </p>
        <motion.button
          type="button"
          className={`ak-drop ${hover ? "ak-drop-hover" : ""}`}
          onClick={async () => {
            const p = await pickFolder();
            if (p) void start(p);
          }}
          whileTap={{ scale: 0.985 }}
        >
          <FolderOpen size={32} strokeWidth={2.2} />
          <div className="ak-drop-title">
            {hover
              ? "Let go to open it"
              : inTauri
                ? "Drop a folder here, or click to choose one"
                : "Open the example: Architekton itself"}
          </div>
          <div className="ak-muted mt-1 text-[13.5px]">
            Read on this computer. Nothing is uploaded.
          </div>
          <div className="ak-stickers">
            {LANGS.map((l) => (
              <span key={l.name} className="ak-sticker" style={{ background: l.color }}>
                {l.name}
              </span>
            ))}
          </div>
        </motion.button>
        <div className="mt-6 flex flex-col items-center gap-2.5">
          {inTauri && (
            <button type="button" className="ak-btn" onClick={() => void start(DEMO_SELF_ROOT)}>
              <Sparkles size={15} /> See how Architekton itself is built
            </button>
          )}
          <div className="ak-more-examples">
            <span>More examples:</span>
            <button type="button" onClick={() => void start(DEMO_ROOT)}>
              web app
            </button>
            <button type="button" onClick={() => void start(DEMO_AI_ROOT)}>
              AI app
            </button>
            <button type="button" onClick={() => void start(DEMO_BROKEN_ROOT)}>
              broken project
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
