import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import { aiStartNarrator, type Sentence, slmChoose, slmExplain } from "../lib/backend";
import { moves, run as runMove, selection, start } from "../lib/explore";
import type { Indexed } from "../lib/graph";
import { useStore } from "../store";

const defaultQuestion = (name: string) =>
  `Explain what ${name} does and why it matters in this part of the codebase.`;
const MAX_LOOKS = 3;

export function Explain({ id, ix }: { id: string; ix: Indexed }) {
  const slm = useStore((s) => s.slm);
  const model = useStore((s) => s.model);
  const reveal = useStore((s) => s.reveal);
  const name = selection(ix, id)?.label ?? "this";
  // What the reader typed; until then the question follows the selection.
  const [typed, setQ] = useState<string | null>(null);
  const q = typed ?? defaultQuestion(name);
  const [look, setLook] = useState(true);
  const [busy, setBusy] = useState<string | null>(null);
  const [trail, setTrail] = useState<{ key: string; id: string }[]>([]);
  const [out, setOut] = useState<{ sentences: Sentence[]; dropped: number } | null>(null);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    setOut(null);
    setErr(null);
    setTrail([]);
    setQ(null);
  }, [id]);
  const sel = selection(ix, id);

  const about = sel ? (
    <div className="ak-explain-about">
      <span className="ak-muted">Explaining the selected {sel.kind}</span>
      <b className="ak-mono break-all">{sel.label}</b>
      <span className="ak-muted ak-mono break-all text-[11px]">{sel.where}</span>
    </div>
  ) : (
    <div className="ak-callout">Select a function, file, route or table to explain it.</div>
  );
  if (!sel) return about;

  if (!slm?.available || !slm.baseUrl || !model) {
    return (
      <div>
        {about}
        <div className="ak-callout">
          <div className="font-semibold">Local model (optional)</div>
          <p className="ak-muted mt-1 text-[12.5px] leading-relaxed">
            {slm?.hint ?? "Checking for a local model…"}
          </p>
          <p className="ak-muted mt-2 text-[12.5px] leading-relaxed">
            Everything you see is computed statically. A small local model only narrates the
            selected subgraph, must cite node ids for every sentence, and nothing is sent off this
            machine.
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            <button
              type="button"
              className="ak-btn"
              onClick={() => useStore.setState({ setupOpen: true })}
            >
              Local AI settings
            </button>
            <button
              type="button"
              className="ak-btn ghost"
              onClick={() => void useStore.getState().refreshSlm()}
            >
              Check again
            </button>
          </div>
        </div>
      </div>
    );
  }

  const run = async () => {
    setErr(null);
    setOut(null);
    setTrail([]);
    try {
      setBusy("Starting the local model…");
      const base = slm.baseUrl === "builtin" ? await aiStartNarrator() : (slm.baseUrl as string);
      const ctx = start(ix, id);
      // The selected code itself is always read: it is what the question is about.
      const own = moves(ix, ctx).find((m) => m.tool === "source" && m.id === id);
      if (own) await runMove(ix, ctx, own);
      for (let i = 0; look && i < MAX_LOOKS; i++) {
        const open = moves(ix, ctx);
        if (!open.length) break;
        setBusy("Looking around…");
        const picked = await slmChoose({
          baseUrl: base,
          model,
          question: `${q}\nSelected: ${sel.kind} ${sel.label} in ${sel.where}`,
          seen: [...ctx.nodes.values()].map((n) => `${n.kind} ${n.label}`),
          moves: open.map((m) => m.key),
          // A model that cannot answer in the asked shape just explains what it has.
        }).catch(() => null);
        const move = open.find((m) => m.key === picked);
        if (!move) break;
        await runMove(ix, ctx, move);
        setTrail((t) => [...t, { key: move.key.replace(/ \[\d+\]$/, ""), id: move.id }]);
      }
      setBusy("Thinking locally…");
      const r = await slmExplain({
        baseUrl: base,
        model,
        question: q,
        nodes: [...ctx.nodes.values()],
        edges: ctx.edges,
      });
      setOut(r);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div>
      {about}
      <div className="ak-muted mb-2 text-[11.5px]">
        {slm.server} · <span className="ak-mono">{model}</span>
      </div>
      <textarea className="ak-input h-20 w-full" value={q} onChange={(e) => setQ(e.target.value)} />
      <label className="mt-2 flex items-center gap-2 text-[12px]">
        <input type="checkbox" checked={look} onChange={(e) => setLook(e.target.checked)} />
        Let the model look around first (up to {MAX_LOOKS} steps, slower)
      </label>
      <button
        type="button"
        className="ak-btn-primary mt-2 w-full"
        disabled={!!busy}
        onClick={() => void run()}
      >
        {busy ?? "Explain with citations"}
      </button>
      {trail.length > 0 && (
        <ol className="ak-muted mt-2 text-[11.5px]">
          {trail.map((s) => (
            <li key={s.key + s.id}>
              Looked:{" "}
              <button type="button" className="underline" onClick={() => reveal(s.id)}>
                {s.key}
              </button>
            </li>
          ))}
        </ol>
      )}
      {err && <div className="mt-2 text-[12px] text-rose-400">{err}</div>}
      <AnimatePresence>
        {out && (
          <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="mt-3 space-y-2">
            {out.sentences.map((s, i) => (
              <motion.p
                key={s.text}
                initial={{ opacity: 0, y: 6 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: i * 0.15 }}
                className="text-[13px] leading-relaxed"
              >
                {s.text}{" "}
                {s.cites.map((c) => (
                  <button key={c} type="button" className="ak-cite" onClick={() => reveal(c)}>
                    {ix.label(c)}
                  </button>
                ))}
              </motion.p>
            ))}
            {out.dropped > 0 && (
              <div className="ak-muted text-[11px]">
                {out.dropped} ungrounded sentence(s) removed.
              </div>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
