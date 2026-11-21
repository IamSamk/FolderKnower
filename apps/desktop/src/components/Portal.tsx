// Scene transitions.
//
// Going deeper: a magnifying lens lands on what you opened, then the next level is
// revealed through a circle that grows out of the lens. Going back: the current scene
// collapses into a circle at the centre. Switching tabs: the new view opens out of the
// tab you clicked. All of it is driven by one `portal` value in the store.

import { AnimatePresence, motion, useReducedMotion } from "motion/react";
import { useStore } from "../store";

export interface PortalOrigin {
  x: number;
  y: number;
  dir: "in" | "out" | "tab";
  /** Text shown inside the lens while it zooms. */
  label?: string;
  /** Tier colour of what is being opened. */
  color?: string;
}

const EASE = [0.65, 0, 0.35, 1] as const;

/** Wraps one scene; the key of the child decides when a transition plays. */
export function PortalScene({
  sceneKey,
  children,
}: {
  sceneKey: string;
  children: React.ReactNode;
}) {
  const portal = useStore((s) => s.portal);
  const reduce = useReducedMotion();
  const ox = portal ? `${portal.x}px` : "50%";
  const oy = portal ? `${portal.y}px` : "50%";
  const dir = portal?.dir ?? "tab";
  const variants = {
    initial: () =>
      reduce
        ? { opacity: 0 }
        : dir === "out"
          ? { clipPath: "circle(150% at 50% 50%)", zIndex: 1 }
          : { clipPath: `circle(0px at ${ox} ${oy})`, zIndex: 2 },
    animate: () =>
      reduce ? { opacity: 1 } : { clipPath: `circle(150% at ${ox} ${oy})`, zIndex: 1 },
    exit: () =>
      reduce
        ? { opacity: 0 }
        : dir === "out"
          ? { clipPath: "circle(0px at 50% 50%)", zIndex: 2 }
          : // keep the old scene on screen underneath while the new one opens
            { zIndex: 0, opacity: 0.98, transition: { duration: dir === "in" ? 1.05 : 0.6 } },
  };
  return (
    <AnimatePresence initial={false}>
      <motion.div
        key={sceneKey}
        className="ak-scene"
        variants={variants}
        initial="initial"
        animate="animate"
        exit="exit"
        transition={{
          duration: reduce ? 0.15 : dir === "tab" ? 0.55 : 0.7,
          ease: EASE,
          delay: dir === "in" && !reduce ? 0.32 : 0,
        }}
      >
        {children}
      </motion.div>
    </AnimatePresence>
  );
}

/** The magnifying glass that lands on the thing being opened. */
export function Lens() {
  const lens = useStore((s) => s.lens);
  const reduce = useReducedMotion();
  return (
    <AnimatePresence>
      {lens && !reduce && (
        <motion.div
          key={lens.id}
          className="ak-lens"
          style={
            {
              left: lens.x,
              top: lens.y,
              "--lc": lens.color ?? "var(--accent)",
            } as React.CSSProperties
          }
          initial={{ scale: 0.2, opacity: 0, rotate: -25 }}
          animate={{ scale: [0.2, 1.05, 1, 6], opacity: [0, 1, 1, 0], rotate: [-25, 0, 0, 0] }}
          transition={{ duration: 1.0, times: [0, 0.28, 0.42, 1], ease: EASE }}
          onAnimationComplete={() => useStore.setState({ lens: null })}
        >
          <div className="ak-lens-glass">
            <span>{lens.label}</span>
          </div>
          <div className="ak-lens-handle" />
        </motion.div>
      )}
    </AnimatePresence>
  );
}

/** Centre of an element relative to the main scene area. */
export function originOf(
  el: Element | null,
  label?: string,
  color?: string,
): Omit<PortalOrigin, "dir"> {
  const main = document.querySelector(".ak-main")?.getBoundingClientRect();
  const r = el?.getBoundingClientRect();
  if (!main || !r)
    return { x: (main?.width ?? 800) / 2, y: (main?.height ?? 600) / 2, label, color };
  return { x: r.left + r.width / 2 - main.left, y: r.top + r.height / 2 - main.top, label, color };
}

export { EASE };
