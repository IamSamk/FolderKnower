import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import type { TierLabel } from "../gen";
import { CONCEPT_BY_ID, type Sentence, slmExplain } from "../lib/backend";
import type { Indexed } from "../lib/graph";
import {
  BASIS_TEXT,
  BIGO_COLOR,
  LAYER_COLOR,
  PROTOCOL_COLOR,
  ROLE_ICON,
  TIER_BLURB,
  TIER_COLOR,
} from "../lib/theme";
import { useStore } from "../store";
import { CodeView } from "./CodeView";

type Tab = "details" | "code" | "explain";

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="ak-section">
      <div className="ak-section-title">{title}</div>
      {children}
    </div>
  );
}

function Ref({ id, ix, sub }: { id: string; ix: Indexed; sub?: string }) {
  const reveal = useStore((s) => s.reveal);
  const t = ix.tierOf(id);
  return (
    <button
      type="button"
      className="ak-ref"
      onClick={() => reveal(id)}
      style={{ "--c": t ? TIER_COLOR[t] : "#64748B" } as React.CSSProperties}
    >
      <span className="ak-ref-dot" />
      <span className="ak-mono truncate">{ix.label(id)}</span>
      {sub && <span className="ak-muted ml-auto shrink-0 pl-2 text-[10.5px]">{sub}</span>}
    </button>
  );
}

function codeTarget(ix: Indexed, id: string): { path: string; range?: [number, number] } | null {
  const fe = ix.fns.get(id);
  if (fe) return { path: fe.file.path, range: fe.fn.lineRange };
  const f = ix.files.get(id);
  if (f) return { path: f.file.path };
  const m = ix.models.get(id);
  if (m) return { path: m.fileId.slice(5), range: [m.line, m.line + Math.max(m.fields.length, 1)] };
  const r = ix.routes.get(id);
  if (r) {
    const h = r.handler ? ix.fns.get(r.handler) : undefined;
    return { path: r.fileId.slice(5), range: h ? h.fn.lineRange : [r.line, r.line] };
  }
  const u = ix.ui.get(id);
  if (u) return { path: u.fileId.slice(5), range: [u.line, u.line] };
  return null;
}

function Details({ id, ix }: { id: string; ix: Indexed }) {
  const drill = useStore((s) => s.drill);
  const stack = useStore((s) => s.stack);
  const hops = useStore((s) => s.hops);

  if (id.startsWith("tier:")) {
    const t = id.slice(5) as TierLabel;
    const tn = ix.tiers.get(t);
    if (!tn) return null;
    return (
      <>
        <Head color={TIER_COLOR[t]} kind="tier" title={t} sub={TIER_BLURB[t]} />
        <button
          type="button"
          className="ak-btn-primary mb-3 w-full"
          onClick={() => drill({ level: "tier", tier: t })}
        >
          Open {t} →
        </button>
        <Section title="Why these files are here">
          <div className="flex flex-wrap gap-1">
            {tn.evidence.map((e) => (
              <span key={e} className="ak-chip ak-mono">
                {e}
              </span>
            ))}
          </div>
        </Section>
        <Section title="Subsystems">
          {tn.clusters.map((c) => (
            <button
              key={c.id}
              type="button"
              className="ak-ref"
              onClick={() => drill({ level: "cluster", clusterId: c.id })}
            >
              <span className="ak-mono">{c.name}</span>
              <span className="ak-muted ml-auto text-[11px]">{c.files.length} files</span>
            </button>
          ))}
        </Section>
      </>
    );
  }

  const ce = ix.clusters.get(id);
  if (ce) {
    return (
      <>
        <Head
          color={TIER_COLOR[ce.tier]}
          kind={`${ce.tier} subsystem`}
          title={ce.cluster.name}
          sub={`${ce.cluster.method.algo} clustering over imports + calls`}
        />
        <button
          type="button"
          className="ak-btn-primary mb-3 w-full"
          onClick={() => drill({ level: "cluster", clusterId: id })}
        >
          Open subsystem →
        </button>
        <Section title="Files">
          {ce.cluster.files.map((f) => (
            <Ref key={f.id} id={f.id} ix={ix} sub={f.role ?? undefined} />
          ))}
        </Section>
      </>
    );
  }

  const fe = ix.files.get(id);
  if (fe) {
    const f = fe.file;
    const ui = ix.g.uiElements.filter((u) => u.fileId === id);
    const routes = ix.g.routes.filter((r) => r.fileId === id);
    const models = ix.g.dataModels.filter((m) => m.fileId === id);
    const inFile = stack[stack.length - 1].level === "file";
    return (
      <>
        <Head
          color={TIER_COLOR[fe.tier]}
          kind={`${fe.tier} · ${f.role ?? "module"}`}
          title={f.path.split("/").pop() ?? f.path}
          sub={f.path}
          icon={ROLE_ICON[f.role ?? ""]}
        />
        {!inFile && (
          <button
            type="button"
            className="ak-btn-primary mb-3 w-full"
            onClick={() => drill({ level: "file", fileId: id })}
          >
            Open call graph →
          </button>
        )}
        <div className="mb-3 grid grid-cols-3 gap-2">
          <Mini n={f.functions.length} l="functions" />
          <Mini n={f.lines} l="lines" />
          <Mini n={`${Math.round(f.tierConfidence * 100)}%`} l="tier confidence" />
        </div>
        {routes.length > 0 && (
          <Section title="Routes declared">
            {routes.map((r) => (
              <Ref key={r.id} id={r.handler ?? r.id} ix={ix} sub={`${r.method} ${r.path}`} />
            ))}
          </Section>
        )}
        {models.length > 0 && (
          <Section title="Data models">
            {models.map((m) => (
              <Ref key={m.id} id={m.id} ix={ix} sub={m.kind} />
            ))}
          </Section>
        )}
        {ui.length > 0 && (
          <Section title="Interactive elements">
            {ui.map((u) => (
              <Ref key={u.id} id={u.id} ix={ix} sub={u.event ?? ""} />
            ))}
          </Section>
        )}
        <Section title="Functions">
          {f.functions.map((x) => (
            <Ref key={x.id} id={x.id} ix={ix} sub={x.complexity.timeBigO} />
          ))}
        </Section>
        {f.imports.length > 0 && (
          <Section title="Imports (resolved)">
            {f.imports.map((e) => (
              <Ref key={e.target} id={e.target} ix={ix} />
            ))}
          </Section>
        )}
      </>
    );
  }

  const x = ix.fns.get(id);
  if (x) {
    const f = x.fn;
    const routes = ix.routesByHandler.get(id) ?? [];
    const out = ix.crossOut.get(id) ?? [];
    const inn = ix.crossIn.get(id) ?? [];
    return (
      <>
        <Head
          color={TIER_COLOR[x.tier]}
          kind={`${x.tier} · function`}
          title={`${f.name}()`}
          sub={`${x.file.path}:${f.lineRange[0]}`}
        />
        {stack[stack.length - 1].level !== "function" && (
          <button
            type="button"
            className="ak-btn-primary mb-3 w-full"
            onClick={() => drill({ level: "function", fnId: id })}
          >
            Focus this function →
          </button>
        )}
        <div className="ak-sig ak-mono">
          <span className="text-[#C4B5FD]">{f.name}</span>(
          {f.signature.parameters.map((p, i) => (
            <span key={p.name + String(i)}>
              {i > 0 && ", "}
              <span>{p.name}</span>
              {p.type && <span className="text-[#67E8F9]">: {p.type}</span>}
              {p.defaultValue && <span className="ak-muted"> = {p.defaultValue}</span>}
            </span>
          ))}
          )
          {f.signature.returnType && (
            <span className="text-[#67E8F9]"> → {f.signature.returnType}</span>
          )}
        </div>
        {routes.map((r) => (
          <div key={r.id} className="ak-route">
            <span className="ak-method">{r.method}</span>
            <span className="ak-mono">{r.path}</span>
            <span className="ak-muted ml-auto text-[11px]">{r.framework}</span>
          </div>
        ))}
        <Section title="Complexity">
          <div className="grid grid-cols-2 gap-2">
            <BigOCard label="time" o={f.complexity.timeBigO} />
            <BigOCard label="space" o={f.complexity.spaceBigO} />
            <Mini n={f.complexity.cyclomatic} l="cyclomatic" />
            <Mini n={f.complexity.cognitive} l="cognitive" />
          </div>
          <ul className="mt-2 space-y-1">
            {f.complexity.basis.map((b) => (
              <li key={b} className="ak-basis">
                <span className="ak-mono text-[10.5px] text-[#A1A1AA]">{b}</span>
                <span>{BASIS_TEXT[b] ?? ""}</span>
              </li>
            ))}
          </ul>
          {f.complexity.memoryFlags.length > 0 && (
            <div className="mt-2 flex flex-wrap gap-1">
              {f.complexity.memoryFlags.map((m) => (
                <span key={m} className="ak-chip" style={{ borderColor: "#FB923C" }}>
                  {m}
                </span>
              ))}
            </div>
          )}
        </Section>
        {f.ontologyTags.length > 0 && (
          <Section title="Systems concepts used here">
            {f.ontologyTags.map((t) => (
              <div
                key={t.ruleId}
                className="ak-concept"
                style={{ "--c": LAYER_COLOR[t.layer] } as React.CSSProperties}
              >
                <div className="flex items-center gap-2">
                  <span className="ak-layer">{t.layer}</span>
                  <span className="font-semibold">{t.concept}</span>
                </div>
                <div className="ak-muted mt-0.5 text-[12px] leading-snug">
                  {CONCEPT_BY_ID.get(t.ruleId)?.explain}
                </div>
              </div>
            ))}
          </Section>
        )}
        {hops.length > 1 && <TraceList ix={ix} />}
        {(out.length > 0 || inn.length > 0) && (
          <Section title="Across tiers">
            {inn.map((e) => (
              <Ref key={e.id} id={e.source} ix={ix} sub={`← ${e.protocol}`} />
            ))}
            {out.map((e) => (
              <Ref key={e.id} id={e.target} ix={ix} sub={`→ ${e.protocol}`} />
            ))}
          </Section>
        )}
        <Section title={`Calls (${f.calls.length})`}>
          {f.calls.map((c) => (
            <Ref
              key={c.target}
              id={c.target}
              ix={ix}
              sub={`${c.provenance.replace("ts_", "")} ${Math.round(c.confidence * 100)}%`}
            />
          ))}
          {f.calls.length === 0 && (
            <div className="ak-muted text-[12px]">No resolved calls into this repo.</div>
          )}
        </Section>
        <Section title={`Called by (${f.calledBy.length})`}>
          {f.calledBy.map((c) => (
            <Ref key={c} id={c} ix={ix} />
          ))}
        </Section>
      </>
    );
  }

  const m = ix.models.get(id);
  if (m) {
    const users = ix.crossIn.get(id) ?? [];
    return (
      <>
        <Head
          color={m.kind === "schema" ? "#A78BFA" : "#10B981"}
          kind={`${m.framework} ${m.kind}`}
          title={m.name}
          sub={m.table ? `table ${m.table}` : m.fileId.slice(5)}
        />
        <Section title="Fields">
          <table className="ak-fields">
            <tbody>
              {m.fields.map((f) => (
                <tr key={f.name}>
                  <td className="ak-key">{f.primaryKey ? "PK" : f.references ? "FK" : ""}</td>
                  <td className="ak-mono">{f.name}</td>
                  <td className="ak-mono ak-muted">
                    {f.type}
                    {f.nullable ? "?" : ""}
                  </td>
                  <td className="ak-mono ak-muted text-[10.5px]">
                    {f.references ? `→ ${f.references}` : ""}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </Section>
        {m.relations.length > 0 && (
          <Section title="Relations">
            {m.relations.map((r) => (
              <Ref
                key={r.target + r.via}
                id={r.target}
                ix={ix}
                sub={`${r.kind.replaceAll("_", " ")} via ${r.via}`}
              />
            ))}
          </Section>
        )}
        {m.mirrors.length > 0 && (
          <Section title={m.kind === "schema" ? "Mirrors table" : "Exposed through schemas"}>
            {m.mirrors.map((x) => (
              <Ref key={x} id={x} ix={ix} />
            ))}
          </Section>
        )}
        {users.length > 0 && (
          <Section title="Functions that touch it">
            {users.map((e) => (
              <Ref
                key={e.id}
                id={e.source}
                ix={ix}
                sub={e.evidence.find((v) => v !== `orm.${m.framework}`) ?? e.protocol}
              />
            ))}
          </Section>
        )}
      </>
    );
  }

  const u = ix.ui.get(id);
  if (u) {
    return (
      <>
        <Head
          color={TIER_COLOR[ix.tierOf(u.fileId) ?? "Frontend"]}
          kind={`UI element · ${u.event ?? "static"}`}
          title={`<${u.tag}>`}
          sub={`${u.label ?? ""} · ${u.fileId.slice(5)}:${u.line}`}
        />
        <TraceList ix={ix} />
      </>
    );
  }
  return <div className="ak-muted text-sm">Nothing selected.</div>;
}

function TraceList({ ix }: { ix: Indexed }) {
  const hops = useStore((s) => s.hops);
  if (hops.length === 0) return null;
  return (
    <Section title="Trace">
      <div className="relative pl-3">
        <div className="ak-trace-rail" />
        {hops.map((h, i) => (
          <motion.div
            key={`${h.id}-${i}`}
            initial={{ opacity: 0, x: -8 }}
            animate={{ opacity: 1, x: 0 }}
            transition={{ delay: i * 0.12 }}
          >
            {h.via && (
              <div
                className="ak-via"
                style={{
                  color:
                    h.via === "http"
                      ? PROTOCOL_COLOR["HTTP/REST"]
                      : h.via === "sql"
                        ? PROTOCOL_COLOR.SQL
                        : "#A1A1AA",
                }}
              >
                {h.via === "http"
                  ? "HTTP request"
                  : h.via === "sql"
                    ? "SQL via ORM"
                    : h.via === "handler"
                      ? "runs handler"
                      : "calls"}
              </div>
            )}
            <Ref id={h.id} ix={ix} sub={h.kind} />
          </motion.div>
        ))}
      </div>
    </Section>
  );
}

function Head({
  color,
  kind,
  title,
  sub,
  icon,
}: {
  color: string;
  kind: string;
  title: string;
  sub?: string;
  icon?: string;
}) {
  return (
    <div className="ak-head" style={{ "--c": color } as React.CSSProperties}>
      <div className="ak-head-kind">{kind}</div>
      <div className="ak-head-title">
        {icon && <span className="mr-1.5">{icon}</span>}
        {title}
      </div>
      {sub && <div className="ak-muted break-all text-[11.5px]">{sub}</div>}
    </div>
  );
}

function Mini({ n, l }: { n: number | string; l: string }) {
  return (
    <div className="ak-stat">
      <div className="ak-stat-n">{typeof n === "number" ? n.toLocaleString() : n}</div>
      <div className="ak-stat-l">{l}</div>
    </div>
  );
}

function BigOCard({ label, o }: { label: string; o: string }) {
  return (
    <div className="ak-stat" style={{ borderColor: BIGO_COLOR[o as keyof typeof BIGO_COLOR] }}>
      <div
        className="ak-stat-n ak-mono"
        style={{ color: BIGO_COLOR[o as keyof typeof BIGO_COLOR] }}
      >
        {o}
      </div>
      <div className="ak-stat-l">{label}</div>
    </div>
  );
}

function grounded(ix: Indexed, id: string) {
  const nodes = new Map<string, { id: string; kind: string; label: string; facts: unknown }>();
  const edges: unknown[] = [];
  const add = (nid: string) => {
    if (nodes.has(nid) || nodes.size > 24) return;
    const fe = ix.fns.get(nid);
    if (fe) {
      nodes.set(nid, {
        id: nid,
        kind: "function",
        label: fe.fn.name,
        facts: {
          file: fe.file.path,
          tier: fe.tier,
          signature: fe.fn.signature,
          bigO: fe.fn.complexity.timeBigO,
          concepts: fe.fn.ontologyTags.map((t) => t.concept),
          routes: (ix.routesByHandler.get(nid) ?? []).map((r) => `${r.method} ${r.path}`),
        },
      });
      return;
    }
    const m = ix.models.get(nid);
    if (m)
      nodes.set(nid, {
        id: nid,
        kind: "table",
        label: m.name,
        facts: { table: m.table, fields: m.fields.map((f) => f.name), framework: m.framework },
      });
    const f = ix.files.get(nid);
    if (f)
      nodes.set(nid, {
        id: nid,
        kind: "file",
        label: f.file.path,
        facts: { tier: f.tier, role: f.file.role, functions: f.file.functions.map((x) => x.name) },
      });
    const u = ix.ui.get(nid);
    if (u)
      nodes.set(nid, {
        id: nid,
        kind: "ui-element",
        label: `<${u.tag}> ${u.label ?? ""}`,
        facts: { event: u.event, file: u.fileId },
      });
  };
  add(id);
  for (const h of useStore.getState().hops) add(h.id);
  const fe = ix.fns.get(id);
  for (const c of fe?.fn.calls ?? []) {
    add(c.target);
    edges.push({ from: id, to: c.target, kind: "call", provenance: c.provenance });
  }
  for (const c of fe?.fn.calledBy ?? []) {
    add(c);
    edges.push({ from: c, to: id, kind: "call" });
  }
  for (const e of [...(ix.crossOut.get(id) ?? []), ...(ix.crossIn.get(id) ?? [])]) {
    add(e.source);
    add(e.target);
    edges.push({ from: e.source, to: e.target, kind: e.protocol, evidence: e.evidence });
  }
  return { nodes: [...nodes.values()], edges };
}

function Explain({ id, ix }: { id: string; ix: Indexed }) {
  const slm = useStore((s) => s.slm);
  const model = useStore((s) => s.model);
  const reveal = useStore((s) => s.reveal);
  const [q, setQ] = useState(
    "Explain what this does and which OS, network and data mechanisms it relies on.",
  );
  const [busy, setBusy] = useState(false);
  const [out, setOut] = useState<{ sentences: Sentence[]; dropped: number } | null>(null);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    setOut(null);
    setErr(null);
  }, [id]);
  if (!slm?.available || !slm.baseUrl || !model) {
    return (
      <div className="ak-callout">
        <div className="font-semibold">Local model (optional)</div>
        <p className="ak-muted mt-1 text-[12.5px] leading-relaxed">
          {slm?.hint ?? "Checking for a local model…"}
        </p>
        <p className="ak-muted mt-2 text-[12.5px] leading-relaxed">
          Everything you see is computed statically. A small local model only narrates the selected
          subgraph, must cite node ids for every sentence, and is never sent anything off this
          machine.
        </p>
        <button
          type="button"
          className="ak-btn mt-3"
          onClick={() => void useStore.getState().refreshSlm()}
        >
          Check again
        </button>
      </div>
    );
  }
  const run = async () => {
    setBusy(true);
    setErr(null);
    try {
      const g = grounded(ix, id);
      const r = await slmExplain({
        baseUrl: slm.baseUrl as string,
        model,
        question: q,
        nodes: g.nodes,
        edges: g.edges,
      });
      setOut(r);
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <div>
      <div className="ak-muted mb-2 text-[11.5px]">
        {slm.server} · <span className="ak-mono">{model}</span>
      </div>
      <textarea className="ak-input h-20 w-full" value={q} onChange={(e) => setQ(e.target.value)} />
      <button
        type="button"
        className="ak-btn-primary mt-2 w-full"
        disabled={busy}
        onClick={() => void run()}
      >
        {busy ? "Thinking locally…" : "Explain with citations"}
      </button>
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

export function Inspector() {
  const ix = useStore((s) => s.ix);
  const selected = useStore((s) => s.selected);
  const root = useStore((s) => s.root);
  const [tab, setTab] = useState<Tab>("details");
  if (!ix || !root) return null;
  const target = selected ? codeTarget(ix, selected) : null;
  return (
    <aside className="ak-inspector">
      <AnimatePresence mode="wait">
        {!selected ? (
          <motion.div
            key="empty"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="p-5"
          >
            <div className="ak-section-title">Inspector</div>
            <p className="ak-muted text-[13px] leading-relaxed">
              Click anything to inspect it. Double-click to zoom in a level. Click a glowing arrow
              or a ghost node to travel along it. Turn on the pointer in the UI view to pick a
              button and follow it to the database.
            </p>
          </motion.div>
        ) : (
          <motion.div
            key={selected}
            initial={{ opacity: 0, x: 16 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -10 }}
            transition={{ duration: 0.22 }}
            className="flex h-full min-h-0 flex-col"
          >
            <div className="ak-tabs">
              {(["details", "code", "explain"] as Tab[]).map((t) => (
                <button
                  key={t}
                  type="button"
                  className={`ak-tab ${tab === t ? "ak-tab-on" : ""}`}
                  onClick={() => setTab(t)}
                  disabled={t === "code" && !target}
                >
                  {t === "explain" ? "Explain ✦" : t[0].toUpperCase() + t.slice(1)}
                  {tab === t && <motion.span layoutId="tabline" className="ak-tabline" />}
                </button>
              ))}
            </div>
            <div className="min-h-0 flex-1 overflow-auto p-4">
              {tab === "details" && <Details id={selected} ix={ix} />}
              {tab === "code" && target && (
                <CodeView root={root} path={target.path} range={target.range} />
              )}
              {tab === "explain" && <Explain id={selected} ix={ix} />}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </aside>
  );
}
