import type { BigO, OntologyLayer, Protocol, TierLabel } from "../gen";

export const TIER_COLOR: Record<TierLabel, string> = {
  Frontend: "#5B8CFF",
  Gateway: "#F5A524",
  Backend: "#A78BFA",
  Storage: "#10B981",
  Workers: "#F43F5E",
  Shared: "#94A3B8",
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
  "HTTP/REST": "#22D3EE",
  WebSocket: "#38BDF8",
  gRPC: "#818CF8",
  SQL: "#34D399",
  IPC: "#FBBF24",
  Queue: "#FB7185",
};

export const LAYER_COLOR: Record<OntologyLayer, string> = {
  os: "#F59E0B",
  network: "#22D3EE",
  cloud: "#A78BFA",
  data: "#34D399",
};

export const BIGO_COLOR: Record<BigO, string> = {
  "O(1)": "#34D399",
  "O(log N)": "#4ADE80",
  "O(N)": "#A3E635",
  "O(N log N)": "#FACC15",
  "O(N^2)": "#FB923C",
  "O(N^k)": "#F87171",
  "O(2^N)": "#EF4444",
  unknown: "#64748B",
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
