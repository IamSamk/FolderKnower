import type { BigO, OntologyLayer, Protocol, TierLabel } from "../gen";

// Tier colours are fills: they always carry ink-coloured text, in both themes.
export const TIER_COLOR: Record<TierLabel, string> = {
  Frontend: "#8FB0FF",
  Gateway: "#FFD25E",
  Backend: "#BFA6FF",
  Storage: "#7FE0B4",
  Workers: "#FF9C8F",
  Shared: "#C9CDDC",
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
  "HTTP/REST": "#3D5AFE",
  WebSocket: "#2196F3",
  gRPC: "#7C4DFF",
  SQL: "#0E9F6E",
  IPC: "#E0A100",
  Queue: "#E5484D",
};

export const LAYER_COLOR: Record<OntologyLayer, string> = {
  os: "#FFB547",
  network: "#7AB8FF",
  cloud: "#C9A7FF",
  data: "#6EDCA8",
};

export const BIGO_COLOR: Record<BigO, string> = {
  "O(1)": "#7FE0B4",
  "O(log N)": "#A6EBA0",
  "O(N)": "#D9F27A",
  "O(N log N)": "#FFE07A",
  "O(N^2)": "#FFB870",
  "O(N^k)": "#FF9C8F",
  "O(2^N)": "#FF7A7A",
  unknown: "#C9CDDC",
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
