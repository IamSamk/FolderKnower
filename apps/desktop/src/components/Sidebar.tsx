import { motion } from "motion/react";
import { useState } from "react";
import type { Question, TierLabel } from "../gen";
import { slmClassify } from "../lib/backend";
import { TIER_COLOR, TIER_ORDER } from "../lib/theme";
import { useStore } from "../store";

function QuestionCard({ q }: { q: Question }) {
  const answer = useStore((s) => s.answer);
  const ix = useStore((s) => s.ix);
  const slm = useStore((s) => s.slm);
  const model = useStore((s) => s.model);
  const [hint, setHint] = useState<{ tier: TierLabel; reason: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const ask = async () => {
    if (!ix || !slm?.baseUrl || !model) return;
    setBusy(true);
    try {
      const files = [...ix.files.values()]
        .filter((f) => f.file.path.startsWith(q.subject))
        .slice(0, 12)
        .map((f) => ({
          path: f.file.path,
          imports: f.file.imports.map((i) => i.target),
          functions: f.file.functions.map((x) => x.name).slice(0, 15),
          routes: ix.g.routes
            .filter((r) => r.fileId === f.file.id)
            .map((r) => `${r.method} ${r.path}`),
        }));
      const r = await slmClassify({
        baseUrl: slm.baseUrl,
        model,
        questionId: q.id,
        subject: q.subject,
        options: q.options,
        files,
      });
      setHint({ tier: r.tier, reason: r.reason });
    } catch (e) {
      setHint({ tier: q.suggested, reason: `Model unavailable: ${String(e)}` });
    } finally {
      setBusy(false);
    }
  };
  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      className="ak-question"
    >
      <div className="text-[12.5px] leading-snug">{q.text}</div>
      <div className="mt-2 flex flex-wrap gap-1.5">
        {q.options.map((o) => (
          <button
            key={o}
            type="button"
            className={`ak-opt ${o === (hint?.tier ?? q.suggested) ? "ak-opt-on" : ""}`}
            style={{ "--c": TIER_COLOR[o] } as React.CSSProperties}
            onClick={() => void answer(q.subject, o)}
          >
            {o}
          </button>
        ))}
      </div>
      {slm?.available && !hint && (
        <button type="button" className="ak-link mt-2" onClick={() => void ask()} disabled={busy}>
          {busy ? "asking local model…" : "✦ ask the local model"}
        </button>
      )}
      {hint && <div className="ak-muted mt-2 text-[11.5px] leading-snug">✦ {hint.reason}</div>}
    </motion.div>
  );
}

export function Sidebar() {
  const ix = useStore((s) => s.ix);
  const result = useStore((s) => s.result);
  const drill = useStore((s) => s.drill);
  const back = useStore((s) => s.back);
  const select = useStore((s) => s.select);
  const reveal = useStore((s) => s.reveal);
  if (!ix || !result) return null;
  const g = ix.g;
  return (
    <aside className="ak-sidebar">
      <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="ak-section">
        <div className="ak-section-title">{g.summary.kind.replace("-", " ")}</div>
        <div className="text-[14px] leading-snug font-medium">{g.summary.headline}</div>
        <ul className="mt-3 space-y-2">
          {g.summary.findings.map((f, i) => (
            <motion.li
              key={f.text}
              initial={{ opacity: 0, x: -8 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: 0.2 + i * 0.12 }}
              className="ak-finding"
              onClick={() => f.evidence[0] && !f.evidence[0].includes("*") && reveal(f.evidence[0])}
            >
              {f.text}
            </motion.li>
          ))}
        </ul>
      </motion.div>

      <div className="ak-section">
        <div className="ak-section-title">Tiers</div>
        {TIER_ORDER.filter((t) => ix.tiers.has(t)).map((t) => {
          const tn = ix.tiers.get(t);
          const n = tn?.clusters.reduce((a, c) => a + c.files.length, 0) ?? 0;
          return (
            <button
              key={t}
              type="button"
              className="ak-tierrow"
              style={{ "--c": TIER_COLOR[t] } as React.CSSProperties}
              onMouseEnter={() => select(`tier:${t}`)}
              onClick={() => {
                back(0);
                drill({ level: "tier", tier: t });
                useStore.setState({ view: "architecture" });
              }}
            >
              <span className="ak-tierrow-dot" />
              <span>{t}</span>
              <span className="ak-muted ml-auto text-[11px]">{n}</span>
            </button>
          );
        })}
      </div>

      {g.questions.length > 0 && (
        <div className="ak-section">
          <div className="ak-section-title">Help me classify</div>
          <div className="space-y-2">
            {g.questions.map((q) => (
              <QuestionCard key={q.id} q={q} />
            ))}
          </div>
        </div>
      )}

      <div className="ak-section ak-muted text-[11px] leading-relaxed">
        {result.stats.files} files · {result.stats.parsed} parsed · {result.stats.cached} from cache
        · {result.stats.millis} ms
        <br />
        ruleset {g.generator.rulesetVersion}
      </div>
    </aside>
  );
}
