# 08. Static DSA detection

**Status: done.** `crates/ark-patterns/src/dsa.rs`, ruleset `patterns 0.1.0`.

## Inputs

From `ark-parse` (enrich pass, extractor version 3):

- per-function calls with receiver, arity and keyword arguments;
- the identifiers used in the function;
- decorators and the control-flow outline.

## Rules

**Container use grouped by (owner, receiver).** A stack, for example, needs `append`
and `pop()` on the same receiver.

| Kind | Fires when |
|---|---|
| Stack | push/append and pop() without index on one receiver; `[-1]` peeks count |
| Queue | `deque` plus `popleft`/`appendleft`, or `queue.Queue` get/put, or `shift`/`push` in JS |
| Deque | both ends used |
| Heap | `heapq.heappush`/`heappop`/`heapify` (receiver is the first argument), `BinaryHeap`, `PriorityQueue` |
| Union-find | `find`/`union` with a `parent` array |

**Shape rules per function.**

| Kind | Fires when |
|---|---|
| Recursion | the function calls itself |
| Memoization | `@lru_cache`/`@cache`, or a memo/dp table with recursion |
| Binary search | `bisect`, or `mid` with `lo`/`hi` (or `left`/`right`) |
| Binary tree | `.left`/`.right` plus root/node/val |
| Trie | children map plus an end-of-word flag |
| Linked list | `.next` walking |
| Graph | an adjacency structure plus a visited/seen set. The algorithm label (BFS with a queue, DFS with a stack or recursion, Dijkstra with a heap, topological sort with in-degree) goes into the evidence |
| Hash map / set | typed `HashMap`/`dict`/`Map`/`set` construction |
| Sorting | `sort`/`sorted` with a key, or hand-rolled swap loops |

Each finding carries: id `dsa:{fn}:{kind}:{subject}`, rule id, confidence, evidence
strings ("stack.append (line 12)") and the ordered operations for the animation.

## Precision notes

- Measured on this repository: visitor stacks and recursive walkers are found correctly.
- Known noise: typed HashMap/HashSet from enum paths, and a recursion false positive on a
  same-named method call. The **next** fix scores typed map/set at 0.5 and drops
  recursion when the receiver is not `self`/`this`.
- Confidence shows in the UI ("85% sure") so students learn that this is inference.
