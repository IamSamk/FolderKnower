// Student-facing labels, colours and small algorithms for the Insights, Workflow and
// Live views. Everything here is pure and deterministic.

import type { AiRole, DsaKind, FlowNode } from "../gen";

export const ROLE_LABEL: Record<AiRole, string> = {
  loader: "Load documents",
  splitter: "Chunk",
  embedder: "Embed",
  vectorStore: "Vector store",
  keywordIndex: "Keyword search",
  retriever: "Retrieve top-k",
  fusion: "Fuse rankings",
  reranker: "Rerank",
  prompt: "Build prompt",
  router: "Decide next step",
  llm: "Language model",
  tool: "Tool",
  agent: "Agent loop",
  memory: "Chat memory",
  cache: "Response cache",
  guardrail: "Guardrail",
  evaluator: "Evaluate",
  modelServer: "Model server",
  tokenizer: "Tokenizer",
  neuralLayer: "Layer",
  trainer: "Training",
};

/** Lane = which phase of the system a stage belongs to (left-to-right story). */
export const ROLE_LANE: Record<AiRole, 0 | 1 | 2 | 3> = {
  loader: 0,
  splitter: 0,
  embedder: 0,
  vectorStore: 0,
  keywordIndex: 0,
  retriever: 1,
  fusion: 1,
  reranker: 1,
  memory: 2,
  cache: 2,
  prompt: 2,
  guardrail: 2,
  llm: 2,
  router: 2,
  agent: 2,
  tool: 2,
  evaluator: 2,
  modelServer: 3,
  tokenizer: 3,
  neuralLayer: 3,
  trainer: 3,
};

export const LANE_TITLE = [
  "Ingest: once, offline",
  "Retrieve: every question",
  "Generate and decide",
  "Inside the model",
];

export const ROLE_COLOR: Record<AiRole, string> = {
  loader: "oklch(85.0% 0.021 274)",
  splitter: "oklch(88.1% 0.142 88)",
  embedder: "oklch(76.4% 0.120 266)",
  vectorStore: "oklch(83.5% 0.112 163)",
  keywordIndex: "oklch(87.6% 0.123 143)",
  retriever: "oklch(86.5% 0.115 207)",
  fusion: "oklch(87.7% 0.137 130)",
  reranker: "oklch(83.4% 0.121 65)",
  prompt: "oklch(87.0% 0.075 336)",
  router: "oklch(79.1% 0.121 28)",
  llm: "oklch(78.0% 0.127 296)",
  tool: "oklch(91.2% 0.126 93)",
  agent: "oklch(79.1% 0.121 28)",
  memory: "oklch(86.0% 0.078 297)",
  cache: "oklch(87.5% 0.077 176)",
  guardrail: "oklch(83.9% 0.089 19)",
  evaluator: "oklch(85.0% 0.021 274)",
  modelServer: "oklch(78.5% 0.104 275)",
  tokenizer: "oklch(92.4% 0.115 96)",
  neuralLayer: "oklch(81.1% 0.101 294)",
  trainer: "oklch(80.8% 0.103 20)",
};

export const ROLE_GLYPH: Record<AiRole, string> = {
  loader: "▤",
  splitter: "✂",
  embedder: "⇶",
  vectorStore: "◈",
  keywordIndex: "≣",
  retriever: "⌕",
  fusion: "⋈",
  reranker: "⇅",
  prompt: "✎",
  router: "◇",
  llm: "✦",
  tool: "⚙",
  agent: "↻",
  memory: "☰",
  cache: "⚡",
  guardrail: "⛨",
  evaluator: "✓",
  modelServer: "▦",
  tokenizer: "⁝",
  neuralLayer: "▥",
  trainer: "∇",
};

/** Human names for the settings the recognizer copies out of calls. */
export const PARAM_LABEL: Record<string, string> = {
  chunk_size: "chunk size",
  chunkSize: "chunk size",
  chunk_overlap: "overlap",
  chunkOverlap: "overlap",
  k: "top-k",
  top_k: "top-k",
  similarity_top_k: "top-k",
  n_results: "top-k",
  top_n: "keep top-n",
  fetch_k: "fetch-k (MMR pool)",
  model: "model",
  model_name: "model",
  temperature: "temperature",
  num_ctx: "context window",
  n_ctx: "context window",
  max_model_len: "context window",
  max_tokens: "max output tokens",
  gpu_memory_utilization: "GPU memory share",
  weights: "fusion weights",
  search_type: "search type",
  nodes: "graph nodes",
  uses: "used",
};

export const DSA_LABEL: Record<DsaKind, string> = {
  stack: "Stack (LIFO)",
  queue: "Queue (FIFO)",
  deque: "Deque",
  heap: "Heap / priority queue",
  hashMap: "Hash map",
  hashSet: "Hash set",
  linkedList: "Linked list",
  binaryTree: "Binary tree",
  trie: "Trie",
  graph: "Graph",
  unionFind: "Union-find",
  recursion: "Recursion (call stack)",
  memoization: "Memoization / DP",
  binarySearch: "Binary search",
  sorting: "Sorting",
};

export const DSA_EXPLAIN: Record<DsaKind, string> = {
  stack:
    "Last in, first out. push puts an item on top, pop takes the top one back. Undo, bracket matching and depth-first search all use one. Push and pop are O(1).",
  queue:
    "First in, first out. Items join at the back and leave from the front, like a line at a counter. Breadth-first search and job queues use one. Use a deque: list.pop(0) is O(N).",
  deque: "A double-ended queue: push and pop at both ends in O(1).",
  heap: "A tree kept in an array where every parent is smaller than its children, so the smallest item is always at index 0. Insert and extract-min are O(log N).",
  hashMap:
    "Stores key → value pairs. The key is hashed to pick a bucket, so lookup, insert and delete are O(1) on average.",
  hashSet:
    "A hash map with keys only. Answers 'have I seen this?' in O(1); graph searches keep a visited set.",
  linkedList:
    "Nodes that each point to the next one. Inserting after a node is O(1), but reaching the i-th node is O(N).",
  binaryTree:
    "Each node has up to two children, left and right. Recursion visits them: pre-order, in-order (sorted order in a search tree) or post-order.",
  trie: "A tree of characters. Each path from the root spells a prefix, so prefix lookups cost O(length of the word).",
  graph:
    "Nodes connected by edges, stored as an adjacency list. BFS explores in waves with a queue; DFS goes deep with a stack or recursion.",
  unionFind:
    "Tracks which items are in the same group. find follows parent links to a root; union joins two roots. Near O(1) with path compression.",
  recursion:
    "The function calls itself on a smaller input. Every call pushes a frame on the call stack (its arguments and locals) and pops it on return; too deep and the stack overflows.",
  memoization:
    "Remembers answers to sub-problems so each one is computed once. Turns exponential recursion like naive Fibonacci into linear time.",
  binarySearch:
    "Looks at the middle of a sorted range and throws away the half that cannot hold the answer. O(log N): 1,000,000 items need about 20 steps.",
  sorting:
    "Puts items in order. Good general sorts (merge sort, quicksort, Timsort) cost O(N log N).",
};

// ---------------------------------------------------------------- ideal path

export type PathMark = "ideal" | "error" | "branch" | "skipped";

const hasError = (n: FlowNode): boolean => Boolean(n.error) || (n.children ?? []).some(hasError);

/**
 * Mark every flow node: `ideal` when a clean run passes through it, `error` when it
 * is (or leads only to) an error exit, `skipped` otherwise. The ideal path takes the
 * branch that does not raise, enters `try` and skips `catch`, and stops at the first
 * normal `return`.
 */
export function idealPath(nodes: FlowNode[]): Map<FlowNode, PathMark> {
  const marks = new Map<FlowNode, PathMark>();
  const markAll = (ns: FlowNode[], m: PathMark) => {
    for (const n of ns) {
      marks.set(n, n.error ? "error" : m);
      markAll(n.children ?? [], m);
    }
  };
  // Returns true when the walk ended (a normal return was reached).
  const walk = (ns: FlowNode[]): boolean => {
    let i = 0;
    while (i < ns.length) {
      const n = ns[i];
      if (n.kind === "branch") {
        // The if / elif / else chain. A clean run takes the first arm that cannot
        // raise; a guard clause (`if bad: raise`) with no clean arm is simply not taken.
        let len = 1;
        while (ns[i + len]?.kind === "else") len++;
        const chain = ns.slice(i, i + len);
        const clean = chain.find((c) => !hasError(c));
        for (const c of chain) {
          if (c !== clean) markAll([c], "skipped");
          if (c !== clean && hasError(c)) marks.set(c, "error");
        }
        if (clean) {
          marks.set(clean, "branch");
          if (walk(clean.children ?? [])) {
            markAll(ns.slice(i + len), "skipped");
            return true;
          }
        }
        i += len;
        continue;
      }
      if (n.kind === "catch") {
        markAll([n], "skipped");
        i++;
        continue;
      }
      if (n.error) {
        marks.set(n, "error");
        i++;
        continue;
      }
      marks.set(n, "ideal");
      if (n.kind === "return") {
        markAll(ns.slice(i + 1), "skipped");
        return true;
      }
      if (n.children?.length && walk(n.children)) {
        markAll(ns.slice(i + 1), "skipped");
        return true;
      }
      i++;
    }
    return false;
  };
  walk(nodes);
  return marks;
}

/** Flatten the ideal path in order, for the step-by-step player. */
export function idealSteps(nodes: FlowNode[], marks: Map<FlowNode, PathMark>): FlowNode[] {
  const out: FlowNode[] = [];
  const rec = (ns: FlowNode[]) => {
    for (const n of ns) {
      const m = marks.get(n);
      if (m === "ideal" || m === "branch") {
        out.push(n);
        rec(n.children ?? []);
      }
    }
  };
  rec(nodes);
  return out;
}

export const FLOW_TEXT: Record<FlowNode["kind"], string> = {
  branch: "decision",
  else: "otherwise",
  loop: "loop",
  try: "try",
  catch: "on error",
  finally: "always",
  switch: "match",
  case: "case",
  call: "step",
  return: "return",
  raise: "error exit",
};

// ---------------------------------------------------------------- memory maths

export function fmtBytes(n: number | undefined | null): string {
  if (n == null || !Number.isFinite(n)) return "–";
  const u = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < u.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v >= 100 || i === 0 ? v.toFixed(0) : v.toFixed(1)} ${u[i]}`;
}

export interface ModelShape {
  name: string;
  layers: number;
  kvHeads: number;
  headDim: number;
  /** Parameters in billions, for the weights estimate. */
  paramsB: number;
  slidingWindow?: number;
}

/** Shapes from each model's published config.json (num_hidden_layers, num_key_value_heads, head_dim). */
export const MODEL_PRESETS: ModelShape[] = [
  { name: "Qwen3-0.6B", layers: 28, kvHeads: 8, headDim: 128, paramsB: 0.6 },
  { name: "Qwen3-4B", layers: 36, kvHeads: 8, headDim: 128, paramsB: 4 },
  { name: "Llama-3.1-8B", layers: 32, kvHeads: 8, headDim: 128, paramsB: 8 },
  {
    name: "Mistral-7B v0.1",
    layers: 32,
    kvHeads: 8,
    headDim: 128,
    paramsB: 7.2,
    slidingWindow: 4096,
  },
  { name: "GPT-2 small", layers: 12, kvHeads: 12, headDim: 64, paramsB: 0.124 },
];

export const DTYPE_BYTES: Record<string, number> = {
  "fp16 / bf16": 2,
  "fp8 / q8_0": 1,
  q4_0: 0.5625,
  fp32: 4,
};

/** KV cache bytes = 2 (K and V) × layers × kv_heads × head_dim × tokens × bytes per value. */
export function kvBytes(m: ModelShape, tokens: number, bytesPerValue: number): number {
  return 2 * m.layers * m.kvHeads * m.headDim * tokens * bytesPerValue;
}
