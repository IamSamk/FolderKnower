// "How it works" view: the recognized AI system, the data structures used, the memory
// tricks configured, and the named libraries. Everything shown comes from the static
// analysis; each card links back to the line that proved it.

import { ArrowRight, Brain, Cpu, Library, Network } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useMemo, useState } from "react";
import type { AiArchitecture, DsaFinding, Insights } from "../../gen";
import { DSA_EXPLAIN, DSA_LABEL } from "../../lib/insights";
import { useStore } from "../../store";
import { AiPipeline } from "./AiPipeline";
import { DsaAnim } from "./DsaAnim";
import { KvCalculator } from "./KvCache";

type Tab = "ai" | "dsa" | "memory" | "libs";

function GoCode({ id, line }: { id: string; line?: number }) {
  const reveal = useStore((s) => s.reveal);
  return (
    <button type="button" className="ak-link-btn" onClick={() => reveal(id)}>
      {line ? `line ${line}` : "open"} <ArrowRight size={12} />
    </button>
  );
}

function DsaCard({ f }: { f: DsaFinding }) {
  const ix = useStore((s) => s.ix);
  return (
    <motion.div layout className="ak-card ak-dsa-card">
      <div className="flex items-baseline gap-2">
        <div className="text-[15px] font-bold">{DSA_LABEL[f.kind]}</div>
        {f.subject && <span className="ak-chip ak-mono">{f.subject}</span>}
        <span className="ak-muted ml-auto text-[11px]">{Math.round(f.confidence * 100)}% sure</span>
      </div>
      <div className="ak-muted ak-mono mt-0.5 truncate text-[11.5px]">
        {ix?.label(f.functionId)}() in {ix?.label(f.fileId)}{" "}
        <GoCode id={f.functionId} line={f.line} />
      </div>
      <DsaAnim finding={f} />
      <p className="mt-2 text-[13px] leading-relaxed">{DSA_EXPLAIN[f.kind]}</p>
      <div className="mt-1.5 flex flex-wrap gap-1">
        {f.evidence.slice(0, 4).map((e) => (
          <span key={e} className="ak-ev ak-mono">
            {e}
          </span>
        ))}
      </div>
    </motion.div>
  );
}

function DsaGallery({ items }: { items: DsaFinding[] }) {
  const kinds = useMemo(() => [...new Set(items.map((f) => f.kind))], [items]);
  const [kind, setKind] = useState<string>("all");
  const shown = kind === "all" ? items : items.filter((f) => f.kind === kind);
  return (
    <div>
      <div className="mb-3 flex flex-wrap gap-1.5">
        {["all", ...kinds].map((k) => (
          <button
            key={k}
            type="button"
            className={`ak-filter ${kind === k ? "on" : ""}`}
            onClick={() => setKind(k)}
          >
            {k === "all" ? `All (${items.length})` : DSA_LABEL[k as DsaFinding["kind"]]}
          </button>
        ))}
      </div>
      <div className="ak-grid">
        <AnimatePresence>
          {shown.slice(0, 60).map((f) => (
            <DsaCard key={f.id} f={f} />
          ))}
        </AnimatePresence>
      </div>
    </div>
  );
}

const BLOCK = [
  { k: "tok", t: "Tokens", d: "Text is cut into token ids.", match: ["tokenizer", "Tokenizer"] },
  {
    k: "emb",
    t: "Embedding",
    d: "Each id becomes a vector of d_model numbers.",
    match: ["Embedding"],
  },
  {
    k: "ln1",
    t: "LayerNorm",
    d: "Rescales each vector so training stays stable.",
    match: ["LayerNorm", "RMSNorm"],
  },
  {
    k: "att",
    t: "Self-attention",
    d: "Each token makes a query Q and compares it with every earlier token's key K, then mixes their values V. K and V of past tokens are kept in the KV cache so they are not recomputed.",
    match: ["Attention", "attention"],
  },
  {
    k: "ln2",
    t: "LayerNorm",
    d: "Normalize again before the MLP.",
    match: ["LayerNorm", "RMSNorm"],
  },
  {
    k: "mlp",
    t: "MLP",
    d: "A small feed-forward network applied to each token on its own.",
    match: ["MLP", "FeedForward", "Linear"],
  },
  {
    k: "out",
    t: "Logits → next token",
    d: "A last Linear layer scores every word in the vocabulary.",
    match: ["lm_head", "Linear"],
  },
];

function TransformerBlock({ ai }: { ai?: AiArchitecture }) {
  const names = (ai?.stages ?? [])
    .filter((s) => s.role === "neuralLayer" || s.role === "tokenizer")
    .map((s) => s.name);
  const [at, setAt] = useState(0);
  useEffect(() => {
    const t = setTimeout(() => setAt((i) => (i + 1) % BLOCK.length), 1400);
    return () => clearTimeout(t);
  }, [at]);
  return (
    <div className="ak-card">
      <div className="ak-section-title">Inside a transformer, one token at a time</div>
      <div className="ak-tf">
        {BLOCK.map((b, i) => {
          const found = names.some((n) => b.match.some((m) => n.includes(m)));
          return (
            <div key={b.k} className="flex items-stretch gap-2">
              <motion.div
                className={`ak-tf-box ${i === at ? "on" : ""} ${found ? "found" : ""} ${b.k === "ln1" ? "first" : ""}`}
                animate={{ scale: i === at ? 1.03 : 1 }}
              >
                <b>{b.t}</b>
                {(b.k === "att" || b.k === "mlp") && (
                  <span className="ak-muted text-[11px]"> (its input is added back: residual)</span>
                )}
                {found && <span className="ak-chip ml-2">in this code</span>}
                {i === at && <div className="ak-muted mt-0.5 text-[12px]">{b.d}</div>}
              </motion.div>
            </div>
          );
        })}
        <div className="ak-muted mt-1 text-[11.5px]">
          The LayerNorm → attention → LayerNorm → MLP part repeats once per layer (12 in GPT-2
          small, 36 in Qwen3-4B).
        </div>
      </div>
    </div>
  );
}

function MemoryTab({ insights }: { insights: Insights }) {
  const ix = useStore((s) => s.ix);
  const llm = insights.ai?.stages.some((s) => s.role === "llm" || s.role === "neuralLayer");
  return (
    <div className="space-y-4">
      {insights.memoryTechniques.length > 0 ? (
        <div className="ak-grid">
          {insights.memoryTechniques.map((m) => (
            <div key={m.id} className="ak-card">
              <div className="flex items-baseline gap-2">
                <div className="text-[15px] font-bold">{m.name}</div>
                <span className="ak-chip ml-auto">{m.engine}</span>
              </div>
              <div className="ak-mono mt-1 text-[12px]">
                {m.setting} <span className="ak-muted">in {ix?.label(m.fileId)}</span>{" "}
                <GoCode id={m.fileId} line={m.line} />
              </div>
              <p className="mt-2 text-[13px] leading-relaxed">{m.explain}</p>
            </div>
          ))}
        </div>
      ) : (
        <p className="ak-muted">No caching or memory-saving settings were found in the code.</p>
      )}
      {llm && <TransformerBlock ai={insights.ai} />}
      <div className="ak-card">
        <div className="ak-section-title">How much memory does a model need? Try it</div>
        <KvCalculator />
      </div>
    </div>
  );
}

function LibsTab({ insights }: { insights: Insights }) {
  const groups = useMemo(() => {
    const m = new Map<string, Insights["libraries"]>();
    for (const l of insights.libraries) m.set(l.category, [...(m.get(l.category) ?? []), l]);
    return [...m.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  }, [insights.libraries]);
  return (
    <div className="space-y-5">
      {groups.map(([cat, libs]) => (
        <div key={cat}>
          <div className="ak-section-title capitalize">{cat}</div>
          <div className="ak-grid">
            {libs.map((l) => (
              <div key={l.name} className="ak-card">
                <div className="text-[15px] font-bold">{l.name}</div>
                <p className="mt-1 text-[13px] leading-relaxed">{l.explain}</p>
                <div className="mt-1.5 flex flex-wrap gap-1">
                  {l.evidence.slice(0, 3).map((e) => (
                    <span key={e} className="ak-ev ak-mono">
                      {e}
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

export function InsightsView() {
  const insights = useStore((s) => s.ix?.g.insights);
  const tabs = useMemo(() => {
    const t: { k: Tab; label: string; n?: number; icon: React.ReactNode }[] = [];
    if (!insights) return t;
    if (insights.ai) t.push({ k: "ai", label: "AI pipeline", icon: <Brain size={15} /> });
    t.push({
      k: "dsa",
      label: "Data structures",
      n: insights.dataStructures.length,
      icon: <Network size={15} />,
    });
    t.push({
      k: "memory",
      label: "Memory and caching",
      n: insights.memoryTechniques.length,
      icon: <Cpu size={15} />,
    });
    t.push({
      k: "libs",
      label: "Libraries",
      n: insights.libraries.length,
      icon: <Library size={15} />,
    });
    return t;
  }, [insights]);
  const [tab, setTab] = useState<Tab>(insights?.ai ? "ai" : "dsa");
  if (!insights)
    return (
      <div className="ak-empty">
        <h2>Nothing recognized yet</h2>
        <p>No data structures, AI pipeline or known libraries were found in this project.</p>
      </div>
    );
  return (
    <div className="ak-insights">
      <nav className="ak-subtabs">
        {tabs.map((t) => (
          <button
            key={t.k}
            type="button"
            className={tab === t.k ? "on" : ""}
            onClick={() => setTab(t.k)}
          >
            {tab === t.k && <motion.span layoutId="subtab" className="ak-tabpill" />}
            <span className="relative flex items-center gap-1.5">
              {t.icon}
              {t.label}
              {t.n !== undefined && <span className="ak-count">{t.n}</span>}
            </span>
          </button>
        ))}
      </nav>
      <div className="ak-insights-body">
        {tab === "ai" && insights.ai && <AiPipeline ai={insights.ai} />}
        {tab === "dsa" &&
          (insights.dataStructures.length ? (
            <DsaGallery items={insights.dataStructures} />
          ) : (
            <p className="ak-muted">No stacks, queues, trees or graphs were recognized.</p>
          ))}
        {tab === "memory" && <MemoryTab insights={insights} />}
        {tab === "libs" && <LibsTab insights={insights} />}
      </div>
    </div>
  );
}
