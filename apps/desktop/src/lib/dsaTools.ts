// Tools that connect data structures to the student's own project:
// 1. structures written in the code (from the DSA recognizers),
// 2. structures the project relies on without writing them (inside the vector DB,
//    the agent graph, the cache, the router, the database) - each tied to a fact the
//    analysis found, never guessed,
// 3. everyday problems and the structure that solves them, with the functions in this
//    project whose names suggest the same problem (shown as a name match to check).

import type { DsaKind, FunctionNode } from "../gen";
import type { Indexed } from "./graph";

export interface Hidden {
  key: string;
  kind: DsaKind;
  title: string;
  where: string;
  why: string;
  ref: string;
  basis: string;
}

/** Structures that live inside tools the project uses, proven by what was detected. */
export function hiddenStructures(ix: Indexed): Hidden[] {
  const out: Hidden[] = [];
  const ins = ix.g.insights;
  const path = (id: string) => ix.files.get(id)?.file.path ?? id.replace(/^file:/, "");
  for (const s of ins?.ai?.stages ?? []) {
    const at = `${path(s.fileId)}:${s.line}`;
    if (s.role === "vectorStore")
      out.push({
        key: `h:${s.id}`,
        kind: "graph",
        title: "HNSW graph inside the vector store",
        where: `${s.name} at ${at}`,
        why: "Vector stores index embeddings as a layered graph (HNSW). A search starts at the top layer and walks greedily towards closer neighbours, so it checks a few hundred vectors instead of all of them.",
        ref: s.functionId ?? s.fileId,
        basis: s.ruleId,
      });
    if (s.role === "retriever" || s.role === "reranker")
      out.push({
        key: `h:${s.id}`,
        kind: "heap",
        title:
          s.role === "retriever" ? "Top-k selection (a heap)" : "Keep the best n (sort / heap)",
        where: `${s.name} at ${at}`,
        why: "Returning the k best matches out of N scores is the classic heap problem: keep a min-heap of size k and push every score through it, O(N log k) instead of sorting everything.",
        ref: s.functionId ?? s.fileId,
        basis: s.ruleId,
      });
    if (s.role === "keywordIndex")
      out.push({
        key: `h:${s.id}`,
        kind: "hashMap",
        title: "Inverted index (hash map of word → chunks)",
        where: `${s.name} at ${at}`,
        why: "Keyword search keeps a map from every word to the list of chunks that contain it, so a query only touches the lists of its own words.",
        ref: s.functionId ?? s.fileId,
        basis: s.ruleId,
      });
    if (s.role === "router")
      out.push({
        key: `h:${s.id}`,
        kind: "graph",
        title: "State graph (the agent loop)",
        where: `${s.name} at ${at}`,
        why: "The agent is a directed graph: nodes are steps (call the model, run a tool), edges are decisions. A cycle in the graph is what makes it a loop; the step limit stops it.",
        ref: s.functionId ?? s.fileId,
        basis: s.ruleId,
      });
    if (s.role === "fusion")
      out.push({
        key: `h:${s.id}`,
        kind: "sorting",
        title: "Merging ranked lists (rank fusion)",
        where: `${s.name} at ${at}`,
        why: "Fusion is a merge of sorted lists: every chunk gets a score from each list, the scores are added in a hash map, and the result is sorted once more.",
        ref: s.functionId ?? s.fileId,
        basis: s.ruleId,
      });
  }
  for (const m of ins?.memoryTechniques ?? []) {
    const at = `${path(m.fileId)}:${m.line}`;
    if (/capped|rolling|window/i.test(m.name))
      out.push({
        key: `h:${m.id}`,
        kind: "deque",
        title: "Ring buffer (bounded deque)",
        where: `${m.name} at ${at}`,
        why: "Keeping only the last N items is a deque with a maximum length: push on one end, drop from the other, both O(1).",
        ref: m.fileId,
        basis: m.id,
      });
    if (/ttl|cache|lru/i.test(m.name) && !/kv|prefix|prompt/i.test(m.name))
      out.push({
        key: `h:${m.id}`,
        kind: "hashMap",
        title: "Hash map with eviction (LRU cache)",
        where: `${m.name} at ${at}`,
        why: "A cache is a hash map for O(1) lookups plus an order of use (a doubly linked list in an LRU). When it is full the least recently used key is dropped.",
        ref: m.fileId,
        basis: m.id,
      });
    if (/paged|kv cache|prefix/i.test(m.name))
      out.push({
        key: `h:${m.id}`,
        kind: "hashMap",
        title: "Block table (pages of the KV cache)",
        where: `${m.name} at ${at}`,
        why: "PagedAttention keeps a table from each request's logical blocks to physical GPU blocks, like an OS page table. Prefix caching hashes a block's tokens so equal prefixes share one block.",
        ref: m.fileId,
        basis: m.id,
      });
  }
  if (ix.g.routes.length >= 2) {
    const r = ix.g.routes[0];
    out.push({
      key: "h:routes",
      kind: "trie",
      title: "Route matching (a trie of URL parts)",
      where: `${ix.g.routes.length} routes, e.g. ${r.method} ${r.path}`,
      why: "Web frameworks match a URL by walking its parts (/todos → :id → complete) down a prefix tree, so lookup cost depends on the URL length, not on how many routes exist.",
      ref: r.handler ?? r.fileId,
      basis: "routes",
    });
  }
  const tables = ix.g.dataModels.filter((m) => m.table);
  if (tables.length) {
    out.push({
      key: "h:btree",
      kind: "binaryTree",
      title: "B-tree index in the database",
      where: tables
        .slice(0, 3)
        .map((t) => t.table)
        .join(", "),
      why: "Primary keys and indexes are B-trees: wide, shallow search trees kept sorted on disk, so finding one row among millions takes 3 or 4 page reads.",
      ref: tables[0].id,
      basis: "dataModels",
    });
  }
  // One card per idea: the same structure found twice lists both places.
  const merged = new Map<string, Hidden>();
  for (const h of out) {
    const m = merged.get(h.title);
    if (m) m.where = `${m.where}; ${h.where}`;
    else merged.set(h.title, { ...h });
  }
  return [...merged.values()];
}

export interface Problem {
  id: string;
  title: string;
  life: string;
  kinds: DsaKind[];
  how: string;
  names: RegExp;
}

/** Everyday problems and the structure that solves each one. */
export const PROBLEMS: Problem[] = [
  {
    id: "undo",
    title: "Undo / back button",
    life: "A pile of plates: the last one you put down is the first one you pick up.",
    kinds: ["stack"],
    how: "Push every change on a stack; undo pops the last one. A second stack gives redo.",
    names: /undo|redo|history|back/i,
  },
  {
    id: "jobs",
    title: "Background jobs in order",
    life: "A queue at a ticket counter: first come, first served.",
    kinds: ["queue", "deque"],
    how: "Producers enqueue at the back, workers dequeue from the front. Celery, RabbitMQ and Redis lists are queues.",
    names: /queue|enqueue|task|job|worker|consume|process_next/i,
  },
  {
    id: "top",
    title: "Top results / leaderboard",
    life: "Keeping the best 10 runners without sorting the whole race.",
    kinds: ["heap", "sorting"],
    how: "A min-heap of size k: push each score, pop when it grows past k. O(N log k).",
    names: /top|rank|best|score|leader|rerank|k_smallest|nlargest/i,
  },
  {
    id: "seen",
    title: "Duplicates / have I seen this?",
    life: "A guest list at the door: one glance tells you if a name is on it.",
    kinds: ["hashSet", "hashMap"],
    how: "Put every key in a set; membership is O(1). Dedupe, visited nodes, idempotency keys.",
    names: /dedup|unique|seen|visited|exists|contains/i,
  },
  {
    id: "cache",
    title: "Remember recent answers",
    life: "Keeping the most used tools on the desk and the rest in a drawer.",
    kinds: ["hashMap", "memoization"],
    how: "A hash map from question to answer, with LRU eviction. Memoization is the same idea inside one function.",
    names: /cache|memo|lru|remember|store_answer|cached/i,
  },
  {
    id: "autocomplete",
    title: "Search as you type",
    life: "A dictionary's thumb index: open at the first letters and you are close.",
    kinds: ["trie", "binarySearch"],
    how: "A trie walks one character per level to all words with that prefix; on a sorted list, binary search finds the first match.",
    names: /search|suggest|complete|prefix|lookup|find/i,
  },
  {
    id: "route",
    title: "Shortest route / connections",
    life: "A metro map: stations are nodes, lines are edges.",
    kinds: ["graph", "queue", "heap"],
    how: "BFS with a queue for fewest hops, Dijkstra with a heap for least distance.",
    names: /path|route|graph|neighbo|bfs|dfs|friend|follow|connect/i,
  },
  {
    id: "order",
    title: "What must happen first",
    life: "Getting dressed: socks before shoes.",
    kinds: ["graph", "recursion"],
    how: "Topological sort of a dependency graph (DFS post-order, or repeatedly take nodes with no incoming edges).",
    names: /depend|order|schedule|plan|migrat|build/i,
  },
  {
    id: "folders",
    title: "Folders, comments, menus",
    life: "A family tree: every item has one parent and any number of children.",
    kinds: ["binaryTree", "recursion"],
    how: "A tree walked with recursion: render a node, then its children.",
    names: /tree|node|child|parent|nest|walk|render_menu|thread/i,
  },
  {
    id: "groups",
    title: "Which group is this in?",
    life: "Friend circles merging when two people meet.",
    kinds: ["unionFind"],
    how: "Union-find joins groups and answers same-group questions in near O(1).",
    names: /group|cluster|merge|union|component/i,
  },
];

export interface Candidate {
  id: string;
  name: string;
  path: string;
  found: boolean;
}

/** Functions whose names suggest the problem, and whether a matching structure was found. */
export function candidatesFor(ix: Indexed, p: Problem): Candidate[] {
  const found = new Set(
    (ix.g.insights?.dataStructures ?? [])
      .filter((f) => p.kinds.includes(f.kind))
      .map((f) => f.functionId),
  );
  const out: Candidate[] = [];
  for (const [id, fe] of ix.fns) {
    if (found.has(id) || p.names.test(fe.fn.name))
      out.push({ id, name: fe.fn.name, path: fe.file.path, found: found.has(id) });
  }
  return out
    .sort((a, b) => Number(b.found) - Number(a.found) || a.path.localeCompare(b.path))
    .slice(0, 8);
}

export interface HotSpot {
  fn: FunctionNode;
  path: string;
  why: string;
}

const SLOW = new Set(["O(N^2)", "O(N^k)", "O(2^N)"]);

/** Functions whose loops nest deeply or whose estimated cost grows fast. */
export function hotSpots(ix: Indexed): HotSpot[] {
  const out: HotSpot[] = [];
  for (const fe of ix.fns.values()) {
    const c = fe.fn.complexity;
    if (SLOW.has(c.timeBigO) || c.maxNesting >= 4)
      out.push({
        fn: fe.fn,
        path: fe.file.path,
        why: SLOW.has(c.timeBigO)
          ? `estimated ${c.timeBigO}${c.basis.length ? ` (${c.basis.slice(0, 2).join(", ")})` : ""}`
          : `${c.maxNesting} levels of nesting`,
      });
  }
  return out.sort((a, b) => b.fn.complexity.cognitive - a.fn.complexity.cognitive).slice(0, 12);
}
