// Animated, step-by-step pictures of data structures. When the code gives us real
// operations (push at line 12, pop at line 15) the animation replays them in source
// order; otherwise it plays a small textbook example and says so.

import { Pause, Play, RotateCcw, StepForward } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useMemo, useState } from "react";
import type { DsaFinding } from "../../gen";

interface Frame<S> {
  caption: string;
  state: S;
}

const spring = { type: "spring", stiffness: 420, damping: 32 } as const;
const VALUES = ["A", "B", "C", "D", "E", "F", "G", "H"];

function usePlayer(n: number, ms = 1100) {
  const [i, setI] = useState(0);
  const [playing, setPlaying] = useState(true);
  useEffect(() => {
    if (!playing || n <= 1) return;
    const t = setTimeout(() => setI((x) => (x + 1) % n), i === n - 1 ? ms * 2 : ms);
    return () => clearTimeout(t);
  }, [i, n, ms, playing]);
  useEffect(() => setI(0), [n]);
  return { i: Math.min(i, Math.max(0, n - 1)), setI, playing, setPlaying };
}

// ---------------------------------------------------------------- frame builders

type Seq = { op: string; line?: number; call?: string }[];

function opsOrExample(f: DsaFinding, example: Seq): { seq: Seq; real: boolean } {
  const real = f.ops.filter((o) => o.op !== "peek" && o.op !== "heapify");
  // A real trace needs both directions to be worth replaying.
  const kinds = new Set(real.map((o) => o.op));
  if (real.length >= 2 && kinds.size >= 2) {
    // Source order shows each operation once; repeat it as if a loop ran it 3 times.
    const out: Seq = [];
    for (let r = 0; r < 3; r++)
      for (const o of real) out.push({ op: o.op, line: o.line, call: o.call });
    return { seq: out.slice(0, 10), real: true };
  }
  return { seq: example, real: false };
}

const where = (s: Seq[number]) => (s.line ? `line ${s.line}: ${s.call}` : "");

function linearFrames(f: DsaFinding, kind: "stack" | "queue"): Frame<string[]>[] {
  const ex: Seq =
    kind === "stack"
      ? [
          { op: "push" },
          { op: "push" },
          { op: "push" },
          { op: "pop" },
          { op: "push" },
          { op: "pop" },
          { op: "pop" },
        ]
      : [
          { op: "enqueue" },
          { op: "enqueue" },
          { op: "enqueue" },
          { op: "dequeue" },
          { op: "enqueue" },
          { op: "dequeue" },
        ];
  const { seq, real } = opsOrExample(f, ex);
  const isAdd = (op: string) =>
    op === "push" || op === "enqueue" || op === "insert" || op === "push_front";
  // Real code often takes before it adds (BFS starts with the start node queued):
  // seed one item so the first removal has something to remove.
  const seeded = seq.length > 0 && !isAdd(seq[0].op);
  let items: string[] = seeded ? [VALUES[0]] : [];
  let v = seeded ? 1 : 0;
  const frames: Frame<string[]>[] = [
    { caption: seeded ? `starts with ${VALUES[0]} inside` : "empty", state: items },
  ];
  for (const s of seq) {
    if (isAdd(s.op)) {
      // Inside a loop a real queue usually takes several items per step (all neighbours).
      const n = real && kind === "queue" ? 2 : 1;
      const vals = Array.from({ length: n }, () => VALUES[v++ % VALUES.length]);
      items = s.op === "push_front" ? [...vals, ...items] : [...items, ...vals];
      frames.push({ caption: `${s.op} ${vals.join(", ")}  ${where(s)}`, state: items });
    } else if (items.length) {
      const front = kind === "queue" || s.op === "dequeue" || s.op === "pop_front";
      const out = front ? items[0] : items[items.length - 1];
      items = front ? items.slice(1) : items.slice(0, -1);
      frames.push({ caption: `${s.op} → ${out}  ${where(s)}`, state: items });
    }
  }
  return frames;
}

function heapFrames(): Frame<{ a: number[]; hi: number }>[] {
  const inserts = [7, 3, 9, 1, 5];
  const frames: Frame<{ a: number[]; hi: number }>[] = [
    { caption: "empty min-heap", state: { a: [], hi: -1 } },
  ];
  const a: number[] = [];
  for (const x of inserts) {
    a.push(x);
    let i = a.length - 1;
    frames.push({ caption: `heappush ${x}: add at the end`, state: { a: [...a], hi: i } });
    while (i > 0 && a[(i - 1) >> 1] > a[i]) {
      const p = (i - 1) >> 1;
      [a[p], a[i]] = [a[i], a[p]];
      i = p;
      frames.push({
        caption: `${x} is smaller than its parent: swap up`,
        state: { a: [...a], hi: i },
      });
    }
  }
  const min = a[0];
  a[0] = a[a.length - 1];
  a.pop();
  let i = 0;
  frames.push({
    caption: `heappop → ${min}: move last item to the root`,
    state: { a: [...a], hi: 0 },
  });
  for (;;) {
    const l = 2 * i + 1;
    const r = l + 1;
    let m = i;
    if (l < a.length && a[l] < a[m]) m = l;
    if (r < a.length && a[r] < a[m]) m = r;
    if (m === i) break;
    [a[m], a[i]] = [a[i], a[m]];
    i = m;
    frames.push({ caption: "sift down: swap with the smaller child", state: { a: [...a], hi: i } });
  }
  return frames;
}

function recursionFrames(f: DsaFinding): Frame<string[]>[] {
  const name = f.subject ?? "f";
  const fan = f.ops.length > 1;
  const frames: Frame<string[]>[] = [];
  const stack: string[] = [];
  const rec = (n: number) => {
    stack.push(`${name}(${n})`);
    frames.push({ caption: `call ${name}(${n}): push a frame`, state: [...stack] });
    if (n >= (fan ? 2 : 1)) {
      rec(n - 1);
      if (fan) rec(n - 2);
    }
    stack.pop();
    frames.push({ caption: `${name}(${n}) returns: pop its frame`, state: [...stack] });
  };
  rec(fan ? 3 : 3);
  return frames;
}

// A fixed 7-node tree / graph used by the traversal pictures.
const TREE = [
  { id: 4, x: 50, y: 14, l: 2, r: 6 },
  { id: 2, x: 26, y: 46, l: 1, r: 3 },
  { id: 6, x: 74, y: 46, l: 5, r: 7 },
  { id: 1, x: 13, y: 80 },
  { id: 3, x: 38, y: 80 },
  { id: 5, x: 62, y: 80 },
  { id: 7, x: 87, y: 80 },
];

function treeFrames(): Frame<{ visited: number[]; at: number }>[] {
  const order = [1, 2, 3, 4, 5, 6, 7];
  return order.map((n, i) => ({
    caption: `in-order: left subtree, then the node, then right → visit ${n}`,
    state: { visited: order.slice(0, i + 1), at: n },
  }));
}

const GRAPH = {
  nodes: [
    { id: "A", x: 12, y: 50 },
    { id: "B", x: 38, y: 18 },
    { id: "C", x: 38, y: 82 },
    { id: "D", x: 64, y: 18 },
    { id: "E", x: 64, y: 82 },
    { id: "F", x: 90, y: 50 },
  ],
  edges: [
    ["A", "B"],
    ["A", "C"],
    ["B", "D"],
    ["C", "E"],
    ["D", "F"],
    ["E", "F"],
    ["B", "C"],
  ],
};

function graphFrames(dfs: boolean): Frame<{ visited: string[]; frontier: string[]; at: string }>[] {
  const adj = new Map<string, string[]>();
  for (const [a, b] of GRAPH.edges) {
    adj.set(a, [...(adj.get(a) ?? []), b]);
    adj.set(b, [...(adj.get(b) ?? []), a]);
  }
  for (const v of adj.values()) v.sort();
  const frames: Frame<{ visited: string[]; frontier: string[]; at: string }>[] = [];
  const seen = new Set(["A"]);
  const frontier = ["A"];
  const visited: string[] = [];
  while (frontier.length) {
    const u = (dfs ? frontier.pop() : frontier.shift()) as string;
    visited.push(u);
    for (const v of adj.get(u) ?? [])
      if (!seen.has(v)) {
        seen.add(v);
        frontier.push(v);
      }
    frames.push({
      caption: `${dfs ? "pop" : "dequeue"} ${u}, mark neighbours as seen, ${dfs ? "push" : "enqueue"} them`,
      state: { visited: [...visited], frontier: [...frontier], at: u },
    });
  }
  return frames;
}

function bsearchFrames(): Frame<{ lo: number; hi: number; mid: number; found: boolean }>[] {
  const a = [2, 5, 8, 12, 16, 23, 38, 56, 72, 91, 99];
  const target = 72;
  let lo = 0;
  let hi = a.length - 1;
  const frames: Frame<{ lo: number; hi: number; mid: number; found: boolean }>[] = [];
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const found = a[mid] === target;
    frames.push({
      caption: found
        ? `a[${mid}] = ${target}: found in ${frames.length + 1} steps`
        : `look at the middle a[${mid}] = ${a[mid]}; ${a[mid] < target ? "too small, keep the right half" : "too big, keep the left half"}`,
      state: { lo, hi, mid, found },
    });
    if (found) break;
    if (a[mid] < target) lo = mid + 1;
    else hi = mid - 1;
  }
  return frames;
}

function hashFrames(): Frame<{ buckets: string[][]; hi: number }>[] {
  const keys = ["cat", "dog", "owl", "bee", "elk", "ant"];
  const h = (k: string) => [...k].reduce((a, c) => (a * 31 + c.charCodeAt(0)) % 997, 7) % 6;
  const buckets: string[][] = Array.from({ length: 6 }, () => []);
  return keys.map((k) => {
    const b = h(k);
    buckets[b] = [...buckets[b], k];
    return {
      caption: `hash("${k}") % 6 = ${b}: store in bucket ${b}${buckets[b].length > 1 ? " (collision: chain it)" : ""}`,
      state: { buckets: buckets.map((x) => [...x]), hi: b },
    };
  });
}

function memoFrames(): Frame<{ cells: (number | null)[]; hi: number }>[] {
  const cells: (number | null)[] = Array(8).fill(null);
  const frames: Frame<{ cells: (number | null)[]; hi: number }>[] = [];
  for (let i = 0; i < 8; i++) {
    cells[i] = i < 2 ? i : (cells[i - 1] as number) + (cells[i - 2] as number);
    frames.push({
      caption:
        i < 2
          ? `base case fib(${i}) = ${i}`
          : `fib(${i}) = fib(${i - 1}) + fib(${i - 2}) = ${cells[i]}, read from the table`,
      state: { cells: [...cells], hi: i },
    });
  }
  return frames;
}

function listFrames(): Frame<{ at: number }>[] {
  return [0, 1, 2, 3, 4].map((i) => ({
    caption: i < 4 ? `node = node.next → ${VALUES[i + 1]}` : "node.next is None: end of the list",
    state: { at: i },
  }));
}

function sortFrames(): Frame<{ a: number[]; hi: number[] }>[] {
  const a = [5, 2, 8, 1, 9, 3];
  const frames: Frame<{ a: number[]; hi: number[] }>[] = [
    { caption: "unsorted", state: { a: [...a], hi: [] } },
  ];
  for (let i = 1; i < a.length; i++) {
    let j = i;
    while (j > 0 && a[j - 1] > a[j]) {
      [a[j - 1], a[j]] = [a[j], a[j - 1]];
      frames.push({ caption: `${a[j]} > ${a[j - 1]}: swap`, state: { a: [...a], hi: [j - 1, j] } });
      j--;
    }
  }
  frames.push({ caption: "sorted", state: { a: [...a], hi: [] } });
  return frames;
}

// ---------------------------------------------------------------- renderers

function Cell({ v, hi, k }: { v: string | number; hi?: boolean; k: string }) {
  return (
    <motion.div
      layout
      key={k}
      initial={{ opacity: 0, scale: 0.6, y: -14 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.6, x: 18 }}
      transition={spring}
      className={`ak-dsa-cell ${hi ? "hi" : ""}`}
    >
      {v}
    </motion.div>
  );
}

function Picture({ f, frame }: { f: DsaFinding; frame: Frame<unknown> }) {
  const s = frame.state as never;
  switch (f.kind) {
    case "stack":
    case "recursion": {
      const items = s as string[];
      return (
        <div className="ak-dsa-stack">
          <AnimatePresence initial={false}>
            {[...items].reverse().map((v, i) => (
              <Cell
                key={`${v}-${items.length - i}`}
                k={`${v}-${items.length - i}`}
                v={v}
                hi={i === 0}
              />
            ))}
          </AnimatePresence>
          <div className="ak-dsa-floor">{f.kind === "recursion" ? "call stack" : "bottom"}</div>
        </div>
      );
    }
    case "queue":
    case "deque": {
      const items = s as string[];
      return (
        <div className="ak-dsa-queue">
          <span className="ak-dsa-end">front</span>
          <AnimatePresence initial={false} mode="popLayout">
            {items.map((v, i) => (
              <Cell key={v + String(i)} k={v} v={v} hi={i === 0} />
            ))}
          </AnimatePresence>
          <span className="ak-dsa-end">back</span>
        </div>
      );
    }
    case "heap": {
      const { a, hi } = s as { a: number[]; hi: number };
      const pos = (i: number) => {
        const depth = Math.floor(Math.log2(i + 1));
        const idx = i - (2 ** depth - 1);
        const slots = 2 ** depth;
        return { x: ((idx + 0.5) / slots) * 100, y: 14 + depth * 30 };
      };
      return (
        <div className="ak-dsa-tree">
          <svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden>
            {a.map((_, i) =>
              i === 0 ? null : (
                <line
                  key={`e${i}`}
                  x1={pos((i - 1) >> 1).x}
                  y1={pos((i - 1) >> 1).y}
                  x2={pos(i).x}
                  y2={pos(i).y}
                />
              ),
            )}
          </svg>
          {a.map((v, i) => (
            <motion.div
              key={`h${v}`}
              layout
              className={`ak-dsa-node ${i === hi ? "hi" : ""}`}
              style={{ left: `${pos(i).x}%`, top: `${pos(i).y}%` }}
              transition={spring}
            >
              {v}
            </motion.div>
          ))}
          <div className="ak-dsa-array">
            {a.map((v, i) => (
              <span key={`a${v}`} className={i === hi ? "hi" : ""}>
                {v}
              </span>
            ))}
          </div>
        </div>
      );
    }
    case "binaryTree": {
      const { visited, at } = s as { visited: number[]; at: number };
      const byId = new Map(TREE.map((n) => [n.id, n]));
      return (
        <div className="ak-dsa-tree">
          <svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden>
            {TREE.flatMap((n) =>
              [n.l, n.r].filter(Boolean).map((c) => {
                const ch = byId.get(c as number);
                return ch ? (
                  <line key={`${n.id}-${c}`} x1={n.x} y1={n.y} x2={ch.x} y2={ch.y} />
                ) : null;
              }),
            )}
          </svg>
          {TREE.map((n) => (
            <div
              key={n.id}
              className={`ak-dsa-node ${n.id === at ? "hi" : visited.includes(n.id) ? "done" : ""}`}
              style={{ left: `${n.x}%`, top: `${n.y}%` }}
            >
              {n.id}
            </div>
          ))}
          <div className="ak-dsa-array">
            {visited.map((v) => (
              <span key={v}>{v}</span>
            ))}
          </div>
        </div>
      );
    }
    case "graph": {
      const { visited, frontier, at } = s as { visited: string[]; frontier: string[]; at: string };
      const byId = new Map(GRAPH.nodes.map((n) => [n.id, n]));
      return (
        <div className="ak-dsa-tree">
          <svg viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden>
            {GRAPH.edges.map(([a, b]) => {
              const p = byId.get(a);
              const q = byId.get(b);
              return p && q ? <line key={a + b} x1={p.x} y1={p.y} x2={q.x} y2={q.y} /> : null;
            })}
          </svg>
          {GRAPH.nodes.map((n) => (
            <div
              key={n.id}
              className={`ak-dsa-node ${n.id === at ? "hi" : visited.includes(n.id) ? "done" : frontier.includes(n.id) ? "next" : ""}`}
              style={{ left: `${n.x}%`, top: `${n.y}%` }}
            >
              {n.id}
            </div>
          ))}
          <div className="ak-dsa-array">
            <em>frontier</em>
            {frontier.map((v) => (
              <span key={v}>{v}</span>
            ))}
          </div>
        </div>
      );
    }
    case "binarySearch": {
      const { lo, hi, mid, found } = s as { lo: number; hi: number; mid: number; found: boolean };
      const a = [2, 5, 8, 12, 16, 23, 38, 56, 72, 91, 99];
      return (
        <div className="ak-dsa-row">
          {a.map((v, i) => (
            <div
              key={v}
              className={`ak-dsa-cell sm ${i === mid ? (found ? "ok" : "hi") : i < lo || i > hi ? "out" : ""}`}
            >
              {v}
              {i === lo && <b className="lo">lo</b>}
              {i === hi && <b className="hi">hi</b>}
            </div>
          ))}
        </div>
      );
    }
    case "hashMap":
    case "hashSet": {
      const { buckets, hi } = s as { buckets: string[][]; hi: number };
      return (
        <div className="ak-dsa-buckets">
          {buckets.map((b, i) => (
            <div key={`b${String(i)}`} className={`ak-dsa-bucket ${i === hi ? "hi" : ""}`}>
              <span className="ak-dsa-end">{i}</span>
              <AnimatePresence>
                {b.map((k) => (
                  <Cell key={k} k={k} v={k} />
                ))}
              </AnimatePresence>
            </div>
          ))}
        </div>
      );
    }
    case "memoization": {
      const { cells, hi } = s as { cells: (number | null)[]; hi: number };
      return (
        <div className="ak-dsa-row">
          {cells.map((v, i) => (
            <div
              key={`m${String(i)}`}
              className={`ak-dsa-cell sm ${i === hi ? "hi" : v == null ? "out" : ""}`}
            >
              {v ?? "·"}
              <b className="lo">{i}</b>
            </div>
          ))}
        </div>
      );
    }
    case "linkedList": {
      const { at } = s as { at: number };
      return (
        <div className="ak-dsa-row">
          {VALUES.slice(0, 5).map((v, i) => (
            <div key={v} className="flex items-center">
              <div className={`ak-dsa-cell sm ${i === at ? "hi" : i < at ? "done" : ""}`}>{v}</div>
              {i < 4 && <span className="ak-dsa-arrow">→</span>}
            </div>
          ))}
          <span className="ak-dsa-end">None</span>
        </div>
      );
    }
    case "sorting": {
      const { a, hi } = s as { a: number[]; hi: number[] };
      return (
        <div className="ak-dsa-bars">
          {a.map((v, i) => (
            <motion.div
              layout
              key={`s${v}`}
              transition={spring}
              className={`ak-dsa-bar ${hi.includes(i) ? "hi" : ""}`}
              style={{ height: `${v * 10}%` }}
            >
              {v}
            </motion.div>
          ))}
        </div>
      );
    }
    default:
      return null;
  }
}

function framesFor(f: DsaFinding): { frames: Frame<unknown>[]; real: boolean } {
  switch (f.kind) {
    case "stack": {
      const real = opsOrExample(f, []).real;
      return { frames: linearFrames(f, "stack"), real };
    }
    case "queue":
    case "deque":
      return { frames: linearFrames(f, "queue"), real: opsOrExample(f, []).real };
    case "heap":
      return { frames: heapFrames(), real: false };
    case "recursion":
      return { frames: recursionFrames(f), real: false };
    case "binaryTree":
      return { frames: treeFrames(), real: false };
    case "graph":
      return {
        frames: graphFrames(f.evidence.some((e) => e.includes("depth-first"))),
        real: false,
      };
    case "binarySearch":
      return { frames: bsearchFrames(), real: false };
    case "hashMap":
    case "hashSet":
      return { frames: hashFrames(), real: false };
    case "memoization":
      return { frames: memoFrames(), real: false };
    case "linkedList":
      return { frames: listFrames(), real: false };
    case "sorting":
      return { frames: sortFrames(), real: false };
    default:
      return { frames: [], real: false };
  }
}

export function DsaAnim({ finding }: { finding: DsaFinding }) {
  const { frames, real } = useMemo(() => framesFor(finding), [finding]);
  const p = usePlayer(frames.length);
  if (!frames.length) return null;
  const fr = frames[p.i];
  return (
    <div className="ak-dsa">
      <div className="ak-dsa-stage">
        <Picture f={finding} frame={fr} />
      </div>
      <div className="ak-dsa-caption ak-mono">
        <span className="ak-dsa-step">
          {p.i + 1}/{frames.length}
        </span>
        {fr.caption}
      </div>
      <div className="ak-dsa-ctrl">
        <button
          type="button"
          onClick={() => p.setPlaying(!p.playing)}
          title={p.playing ? "Pause" : "Play"}
        >
          {p.playing ? <Pause size={13} /> : <Play size={13} />}
        </button>
        <button
          type="button"
          onClick={() => {
            p.setPlaying(false);
            p.setI((p.i + 1) % frames.length);
          }}
          title="Step"
        >
          <StepForward size={13} />
        </button>
        <button type="button" onClick={() => p.setI(0)} title="Restart">
          <RotateCcw size={13} />
        </button>
        <span className="ak-muted ml-auto text-[11px]">
          {real
            ? "replaying the operations found in this code"
            : "textbook example of the same structure"}
        </span>
      </div>
    </div>
  );
}
