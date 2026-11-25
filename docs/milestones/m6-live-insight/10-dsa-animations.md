# 10. DSA animations

**Status: done.** `components/insights/DsaAnim.tsx`.

Each finding gets a small looping, step-able picture: play/pause, step and restart, a
caption per frame, and a counter ("3/7").

| Kind | Picture |
|---|---|
| Stack, recursion | vertical stack with a floor ("bottom" / "call stack"); frames push and pop |
| Queue, deque | horizontal lane labelled front and back |
| Heap | tree drawn from the array, with the array below it; sift-up and sift-down highlighted |
| Binary tree | in-order walk; visited nodes turn green and the output list grows |
| Graph | BFS or DFS: current node yellow, frontier blue, visited green |
| Binary search | sorted array with lo/hi/mid markers; discarded halves fade |
| Hash map / set | buckets; keys hash into them |
| Memoization | the memo table filling, so each sub-problem is computed once |
| Linked list | a pointer walking `next` |
| Sorting | bars swapping (insertion sort) |

## Real operations or a textbook example

If the finding has at least two kinds of operations (push and pop), the animation
replays them **in source order with their line numbers** ("pop → A  line 14:
stack.pop"). It repeats the sequence as a loop would, and seeds one item when the code
takes before it adds (BFS starts with the start node queued). Otherwise it plays a
textbook example and says "textbook example of the same structure". It never pretends.

## Motion

motion 14 layout animations with one spring (stiffness 420, damping 32). The idle
loop slows on the last frame so the end state can be read.

## Next

- Replay runtime traces (doc 05) with real values.
- Export a GIF or WebM per card for slides (render frames to canvas, encode with
  WebCodecs).
