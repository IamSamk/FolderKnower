// Memory map: where every kind of runtime data lives while the system runs. One
// column per place (GPU memory, RAM, disk, browser, hosted provider), grouped by the
// container or process that holds it, with how long it lives and what bounds it.
// Everything comes from `memoryMap` in the graph; each card links to its proof.

import { Cpu, Database, Globe, HardDrive, MemoryStick, Monitor } from "lucide-react";
import { motion } from "motion/react";
import type { Feature, MemoryPlacement } from "../../gen";
import { fmtBytes } from "../../lib/insights";
import {
  type Depth,
  featureBoxes,
  LIFE_NAME,
  PLACE_COLOR,
  PLACE_NAME,
  plainName,
  proofTarget,
  SVC_COLOR,
} from "../../lib/system";
import { useStore } from "../../store";

const ORDER: MemoryPlacement["place"][] = ["vram", "ram", "disk", "browser", "remote"];

const PLAIN_PLACE: Record<MemoryPlacement["place"], string> = {
  vram: "On the graphics card",
  ram: "In working memory",
  disk: "Saved on disk",
  browser: "In the user's browser",
  remote: "At an outside provider",
};

const PLACE_HINT: Record<MemoryPlacement["place"], string> = {
  vram: "Fastest and smallest. Models and their KV cache live here when a GPU is reserved.",
  ram: "Fast, gone on restart. Indexes, caches and models without a GPU.",
  disk: "Slow but kept. Volumes, database files, saved indexes and model downloads.",
  browser: "Kept on the user's device, per site. Never shared between users.",
  remote: "Kept by a hosted API. This project only sends requests.",
};

const PLACE_ICON: Record<MemoryPlacement["place"], React.ReactNode> = {
  vram: <Cpu size={15} />,
  ram: <MemoryStick size={15} />,
  disk: <HardDrive size={15} />,
  browser: <Monitor size={15} />,
  remote: <Globe size={15} />,
};

const LIFE_COLOR: Record<MemoryPlacement["lifetime"], string> = {
  request: "oklch(88.5% 0.132 87)",
  session: "oklch(81.2% 0.129 349)",
  process: "oklch(83.7% 0.087 245)",
  persistent: "oklch(86.8% 0.149 154)",
};

export function MemoryMap({ feature, depth }: { feature?: Feature; depth: Depth }) {
  const ix = useStore((s) => s.ix);
  const reveal = useStore((s) => s.reveal);
  if (!ix) return null;
  const services = ix.g.insights?.deployment?.services ?? [];
  const synthetic = services.length === 0;
  const boxes = feature ? featureBoxes(ix, feature, synthetic) : undefined;
  const files = feature ? new Set([...feature.files, ...feature.sharedFiles]) : undefined;
  const stages = feature ? new Set(feature.aiStages) : undefined;
  const all = ix.g.memoryMap ?? [];
  const shown = all.filter((p) => {
    if (!boxes) return true;
    if (boxes.has(p.holder) || stages?.has(p.source)) return true;
    if (p.holder === "browser") return (feature?.ui.length ?? 0) > 0;
    return p.proof.some((x) => files?.has(`file:${x.replace(/:\d+$/, "")}`));
  });
  if (!all.length)
    return (
      <div className="ak-empty">
        <h2>No runtime data found</h2>
        <p>
          No cache, model, vector index, database container or browser storage was recognized in
          this project.
        </p>
      </div>
    );
  const simple = depth === "simple";
  const places = ORDER.filter((pl) => shown.some((p) => p.place === pl));
  return (
    <div className="ak-memmap">
      <div className="ak-memmap-legend">
        <span className="ak-muted">
          {simple
            ? "Where the system keeps things while it runs, and for how long."
            : "Where each piece of runtime data lives, who holds it and what limits it. Click a line to see the code."}
        </span>
        <span className="ml-auto flex flex-wrap gap-1.5">
          {(Object.keys(LIFE_NAME) as MemoryPlacement["lifetime"][]).map((l) => (
            <span key={l} className="ak-life" style={{ ["--l" as string]: LIFE_COLOR[l] }}>
              {LIFE_NAME[l]}
            </span>
          ))}
        </span>
      </div>
      <div className="ak-memmap-cols">
        {places.map((pl, ci) => {
          const items = shown.filter((p) => p.place === pl);
          const holders = [...new Set(items.map((p) => p.holder))];
          return (
            <motion.section
              key={pl}
              className="ak-memmap-col"
              style={{ ["--p" as string]: PLACE_COLOR[pl] }}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: ci * 0.05 }}
            >
              <header>
                <div className="flex items-center gap-1.5">
                  {PLACE_ICON[pl]}
                  <b>{simple ? PLAIN_PLACE[pl] : PLACE_NAME[pl]}</b>
                  <span className="ak-count ml-auto">{items.length}</span>
                </div>
                <p className="ak-muted">{PLACE_HINT[pl]}</p>
              </header>
              {holders.map((h) => {
                const svc = services.find((s) => s.id === h);
                const list = items.filter((p) => p.holder === h);
                const name = svc
                  ? simple
                    ? `${plainName(ix, svc)} (${svc.name})`
                    : svc.name
                  : list[0].holderName;
                return (
                  <div key={h} className="ak-memmap-holder">
                    <div className="ak-memmap-hname">
                      <i style={{ background: svc ? SVC_COLOR[svc.kind] : "var(--line)" }} />
                      <span className="truncate">{name}</span>
                      {!simple && svc?.memoryLimit && pl === "ram" ? (
                        <span className="ak-chip ml-auto">limit {fmtBytes(svc.memoryLimit)}</span>
                      ) : !simple && svc && !svc.memoryLimit && pl === "ram" ? (
                        <span className="ak-chip ak-chip-dim ml-auto">no limit</span>
                      ) : null}
                      {!simple && svc?.gpu && <span className="ak-chip ak-chip-gpu">GPU</span>}
                    </div>
                    {list.map((p) => (
                      <div key={p.id} className="ak-memmap-item">
                        <div className="flex items-start gap-2">
                          <b className="min-w-0 flex-1">{p.what}</b>
                          <span
                            className="ak-life shrink-0"
                            style={{ ["--l" as string]: LIFE_COLOR[p.lifetime] }}
                          >
                            {LIFE_NAME[p.lifetime]}
                          </span>
                        </div>
                        {!simple && p.bound && (
                          <div className="ak-memmap-bound ak-mono">{p.bound}</div>
                        )}
                        <p className={depth === "deep" ? "" : "ak-clamp2"}>{p.explain}</p>
                        {!simple && (
                          <div className="flex flex-wrap gap-1">
                            {p.proof.map((x) => {
                              const t = proofTarget(ix, x);
                              return t ? (
                                <button
                                  key={x}
                                  type="button"
                                  className="ak-ev ak-mono ak-ev-link"
                                  onClick={() => reveal(t)}
                                >
                                  {x}
                                </button>
                              ) : (
                                <span key={x} className="ak-ev ak-mono">
                                  {x}
                                </span>
                              );
                            })}
                          </div>
                        )}
                      </div>
                    ))}
                  </div>
                );
              })}
            </motion.section>
          );
        })}
      </div>
      {feature && shown.length === 0 && (
        <p className="ak-muted px-5">
          <Database size={13} className="mr-1 inline" />
          This feature keeps nothing outside the request itself.
        </p>
      )}
    </div>
  );
}
