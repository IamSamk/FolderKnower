# 09. Dynamic DSA detection

**Status: later.**

Static rules see names; runtime sees shape. Following MemPick and Howard, a heap
snapshot can classify structures by how objects point to each other:

- each node has one `next` → linked list; also `prev` → doubly linked;
- two child pointers and no cycles → binary tree; balanced height → likely a search tree;
- arbitrary out-degree with cycles → graph.

## For Python

`gc.get_objects()` limited to instances of classes defined in the repo, sampled once at
a breakpoint the user picks. Edges come from `__dict__` and `__slots__`. Cost is
proportional to the live objects of those classes only.

## For native code

Out of scope until `ark-probe` ships; we would need DWARF types plus heap walking.

## Payoff

- Confirms or corrects static findings (confidence goes up or the finding is removed).
- Real values in the animation: the actual tree that was built, not a textbook one.
