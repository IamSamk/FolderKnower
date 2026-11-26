import type { BigO, OntologyLayer, Protocol, TierLabel } from "../gen";

// Tier colours are fills: they always carry ink-coloured text, in both themes.
export const TIER_COLOR: Record<TierLabel, string> = {
  Frontend: "oklch(76.4% 0.120 266)",
  Gateway: "oklch(88.1% 0.142 88)",
  Backend: "oklch(78.0% 0.127 296)",
  Storage: "oklch(83.5% 0.112 163)",
  Workers: "oklch(79.1% 0.121 28)",
  Shared: "oklch(85.0% 0.021 274)",
};

export const TIER_BLURB: Record<TierLabel, string> = {
  Frontend: "What users see and click",
  Gateway: "Middleware, auth and proxies in front of handlers",
  Backend: "Request handlers and business logic",
  Storage: "Tables, ORM models and data access",
  Workers: "Background jobs and schedulers",
  Shared: "Utilities and types used across tiers",
};

export const TIER_ORDER: TierLabel[] = [
  "Frontend",
  "Gateway",
  "Backend",
  "Workers",
  "Storage",
  "Shared",
];

export const PROTOCOL_COLOR: Record<Protocol, string> = {
  "HTTP/REST": "oklch(55.5% 0.243 269)",
  WebSocket: "oklch(65.8% 0.169 249)",
  gRPC: "oklch(57.9% 0.247 288)",
  SQL: "oklch(62.2% 0.133 162)",
  IPC: "oklch(75.0% 0.155 80)",
  Queue: "oklch(62.6% 0.193 23)",
};

export const LAYER_COLOR: Record<OntologyLayer, string> = {
  os: "oklch(82.4% 0.149 73)",
  network: "oklch(76.9% 0.121 253)",
  cloud: "oklch(79.1% 0.127 301)",
  data: "oklch(81.5% 0.127 161)",
};

export const BIGO_COLOR: Record<BigO, string> = {
  "O(1)": "oklch(83.5% 0.112 163)",
  "O(log N)": "oklch(87.6% 0.123 143)",
  "O(N)": "oklch(91.8% 0.148 119)",
  "O(N log N)": "oklch(91.2% 0.126 93)",
  "O(N^2)": "oklch(83.4% 0.121 65)",
  "O(N^k)": "oklch(79.1% 0.121 28)",
  "O(2^N)": "oklch(73.5% 0.163 22)",
  unknown: "oklch(85.0% 0.021 274)",
};

export const BASIS_TEXT: Record<string, string> = {
  "bigo.no-loops": "No loops or recursion: runs a fixed number of steps.",
  "bigo.loop.depth-1": "One loop over the input: grows linearly.",
  "bigo.loop.depth-2": "A loop inside a loop: every pair is visited.",
  "bigo.loop.depth-3+": "Three or more nested loops.",
  "bigo.loop.halving": "The loop halves (or doubles) its index each step.",
  "bigo.loop.depth-2-halving": "An outer loop around a halving inner loop.",
  "bigo.lib.sort": "Calls a sort, which costs N log N.",
  "bigo.recursion.fanout": "Calls itself more than once per call: the call tree explodes.",
  "bigo.recursion.divide-and-conquer":
    "Splits the input and recurses on the halves (merge-sort style).",
  "bigo.recursion.halving": "Recurses on half the input (binary-search style).",
  "bigo.recursion.linear": "Recurses once per element.",
  "space.alloc-in-loop": "Grows a collection inside a loop.",
  "space.recursion-stack": "Each recursive call keeps a stack frame alive.",
  "space.lib.sort": "Sorting needs a scratch buffer proportional to N.",
};

export const ROLE_ICON: Record<string, string> = {
  component: "◧",
  page: "▣",
  hook: "⟲",
  "api-client": "⇄",
  "route-handler": "⇥",
  middleware: "⛨",
  worker: "⚙",
  "orm-models": "▤",
  schemas: "⌗",
  entrypoint: "▶",
  repository: "⛁",
  service: "◆",
  types: "τ",
  config: "⚑",
  utility: "✧",
};
