// KV cache, worked out in front of the student. The formula is the real one
// (2 × layers × kv_heads × head_dim × tokens × bytes) with shapes from each model's
// config.json; the toggles show what PagedAttention, prefix caching, KV quantization and
// a sliding window each save.

import { motion } from "motion/react";
import { useEffect, useMemo, useState } from "react";
import { DTYPE_BYTES, fmtBytes, kvBytes, MODEL_PRESETS } from "../../lib/insights";

const BLOCK = 16;
const SINKS = 4;
const CTX_STEPS = [512, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072];

/** A row of token slots filling one by one; with a window the oldest leave (rolling buffer). */
function TokenStrip({ window, sinks }: { window: number | null; sinks: boolean }) {
  const total = 28;
  const [n, setN] = useState(1);
  useEffect(() => {
    const t = setTimeout(() => setN((x) => (x >= total ? 1 : x + 1)), n >= total ? 1600 : 260);
    return () => clearTimeout(t);
  }, [n]);
  const w = window ?? total;
  const kept = (i: number) => {
    if (i >= n) return "empty";
    if (sinks && i < SINKS) return "sink";
    return i >= n - w ? "kept" : "gone";
  };
  return (
    <div>
      <div className="ak-kv-strip">
        {Array.from({ length: total }, (_, i) => (
          <motion.span
            key={i}
            className={`ak-kv-tok ${kept(i)}`}
            animate={{ scale: i === n - 1 ? [1.4, 1] : 1 }}
            transition={{ duration: 0.25 }}
          />
        ))}
      </div>
      <div className="ak-muted mt-1 text-[11.5px]">
        token {n}: the model stores one K and one V vector per layer for every token it keeps.
        {window
          ? ` Only the last ${window} stay${sinks ? `, plus ${SINKS} "attention sink" tokens at the start` : ""}.`
          : " Nothing is dropped, so memory grows with every token."}
      </div>
    </div>
  );
}

export function KvCalculator({ ramBytes }: { ramBytes?: number }) {
  const [mi, setMi] = useState(1);
  const [ctxI, setCtxI] = useState(4);
  const [dtype, setDtype] = useState("fp16 / bf16");
  const [wq, setWq] = useState(0.5625);
  const [seqs, setSeqs] = useState(4);
  const [fill, setFill] = useState(35);
  const [paged, setPaged] = useState(true);
  const [prefix, setPrefix] = useState(true);
  const [prefixTok, setPrefixTok] = useState(600);
  const [sliding, setSliding] = useState(false);
  const [sinks, setSinks] = useState(false);
  const m = MODEL_PRESETS[mi];
  const ctx = CTX_STEPS[ctxI];
  const bpv = DTYPE_BYTES[dtype];
  const win = sliding ? (m.slidingWindow ?? 4096) : null;

  const r = useMemo(() => {
    const perTok = kvBytes(m, 1, bpv);
    const cap = (t: number) => (win ? Math.min(t, win + (sinks ? SINKS : 0)) : t);
    const used = cap(Math.round((ctx * fill) / 100));
    const reserve = cap(ctx);
    const shared = prefix ? Math.min(prefixTok, used) : 0;
    const blocks = (t: number) => Math.ceil(t / BLOCK) * BLOCK;
    const naive = perTok * reserve * seqs;
    const pagedBytes = perTok * (blocks(shared) + seqs * blocks(used - shared));
    const kv = paged ? pagedBytes : naive - (prefix ? perTok * shared * (seqs - 1) : 0);
    const weights = m.paramsB * 1e9 * wq;
    return {
      perTok,
      used,
      reserve,
      naive,
      kv,
      weights,
      total: kv + weights,
      wasted: naive - pagedBytes,
    };
  }, [m, bpv, win, sinks, ctx, fill, prefix, prefixTok, seqs, paged, wq]);

  const max = Math.max(r.total, ramBytes ?? 0, r.naive + r.weights);
  const pct = (b: number) => `${Math.max(0.5, (b / max) * 100)}%`;

  return (
    <div className="ak-kvcalc">
      <div className="ak-kv-formula ak-mono">
        KV bytes = 2 (K and V) × {m.layers} layers × {m.kvHeads} KV heads × {m.headDim} head dim ×
        tokens × {bpv} bytes
        <br />= <b>{fmtBytes(r.perTok)}</b> per token
      </div>
      <TokenStrip window={win ? 10 : null} sinks={sinks} />
      <div className="ak-kv-grid">
        <label>
          model
          <select className="ak-input" value={mi} onChange={(e) => setMi(Number(e.target.value))}>
            {MODEL_PRESETS.map((p, i) => (
              <option key={p.name} value={i}>
                {p.name}
              </option>
            ))}
          </select>
        </label>
        <label>
          context window: <b>{ctx.toLocaleString()}</b> tokens
          <input
            type="range"
            min={0}
            max={CTX_STEPS.length - 1}
            value={ctxI}
            onChange={(e) => setCtxI(Number(e.target.value))}
          />
        </label>
        <label>
          requests at the same time: <b>{seqs}</b>
          <input
            type="range"
            min={1}
            max={64}
            value={seqs}
            onChange={(e) => setSeqs(Number(e.target.value))}
          />
        </label>
        <label>
          an average request fills <b>{fill}%</b> of the window
          <input
            type="range"
            min={1}
            max={100}
            value={fill}
            onChange={(e) => setFill(Number(e.target.value))}
          />
        </label>
        <label>
          KV cache type
          <select className="ak-input" value={dtype} onChange={(e) => setDtype(e.target.value)}>
            {Object.keys(DTYPE_BYTES).map((k) => (
              <option key={k}>{k}</option>
            ))}
          </select>
        </label>
        <label>
          weights stored as
          <select className="ak-input" value={wq} onChange={(e) => setWq(Number(e.target.value))}>
            <option value={2}>fp16 / bf16 (2 bytes)</option>
            <option value={1}>8-bit (1 byte)</option>
            <option value={0.5625}>4-bit Q4_K_M (~0.56 byte)</option>
          </select>
        </label>
      </div>
      <div className="ak-kv-toggles">
        <label title="vLLM's PagedAttention: KV lives in 16-token blocks handed out on demand, like virtual memory pages.">
          <input type="checkbox" checked={paged} onChange={(e) => setPaged(e.target.checked)} />{" "}
          PagedAttention blocks
        </label>
        <label title="Requests that start with the same tokens (a system prompt) share those KV blocks.">
          <input type="checkbox" checked={prefix} onChange={(e) => setPrefix(e.target.checked)} />{" "}
          prefix caching, shared prefix
          <input
            className="ak-input ak-kv-num"
            type="number"
            min={0}
            value={prefixTok}
            onChange={(e) => setPrefixTok(Math.max(0, Number(e.target.value) || 0))}
          />{" "}
          tokens
        </label>
        <label title="Mistral-style rolling buffer: keep only the last W tokens of KV.">
          <input type="checkbox" checked={sliding} onChange={(e) => setSliding(e.target.checked)} />{" "}
          sliding window ({m.slidingWindow ?? 4096})
        </label>
        <label title="StreamingLLM: keep the first few tokens forever; attention leans on them.">
          <input
            type="checkbox"
            checked={sinks}
            disabled={!sliding}
            onChange={(e) => setSinks(e.target.checked)}
          />{" "}
          attention sinks
        </label>
      </div>
      <div className="ak-kv-bars">
        <div className="ak-kv-bar">
          <span className="ak-kv-seg w" style={{ width: pct(r.weights) }} title="weights" />
          <span className="ak-kv-seg k" style={{ width: pct(r.kv) }} title="KV cache" />
        </div>
        {ramBytes ? (
          <div className="ak-kv-ram" style={{ left: pct(ramBytes) }}>
            <span>this computer's RAM {fmtBytes(ramBytes)}</span>
          </div>
        ) : null}
        <div className="ak-kv-legend">
          <span>
            <i className="w" /> weights {fmtBytes(r.weights)}
          </span>
          <span>
            <i className="k" /> KV cache {fmtBytes(r.kv)}
          </span>
          <span className="ml-auto font-semibold">total {fmtBytes(r.total)}</span>
        </div>
      </div>
      <ul className="ak-kv-notes">
        <li>
          Each request keeps {r.used.toLocaleString()} tokens of KV
          {win ? `, capped by the ${win}-token window` : ""}.
        </li>
        <li>
          Reserving the whole window up front (no paging) would take <b>{fmtBytes(r.naive)}</b>
          {paged && r.wasted > 0 ? (
            <>
              ; paging saves <b>{fmtBytes(r.wasted)}</b>.
            </>
          ) : (
            "."
          )}
        </li>
        {prefix && seqs > 1 && (
          <li>
            The shared {prefixTok}-token prefix is stored once instead of {seqs} times.
          </li>
        )}
        {ramBytes && r.total > ramBytes && (
          <li className="ak-err">This does not fit in this computer's memory.</li>
        )}
      </ul>
    </div>
  );
}
