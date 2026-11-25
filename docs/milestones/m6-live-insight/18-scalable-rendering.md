# 18. Scalable rendering for huge trees

**Status: next.** Targets: a 200k-file monorepo with `node_modules`, or a `bin/` with
thousands of DLLs, stays at 60 fps on a 2018 laptop with integrated graphics.

## Principles

1. **Never draw what is not visible.** Semantic zoom already limits each level to one
   parent's children. Within a level:
   - lists use `@tanstack/react-virtual`;
   - canvases cull by viewport using a static R-tree (`flatbush`).
2. **Level of detail.** Below a zoom threshold a node is a coloured rectangle with no
   text; labels appear only when ≥ 9 px tall; edges bundle into one thick edge per
   cluster pair.
3. **Aggregate before rendering.** Folders over N children collapse to a "+1,240 files"
   card with a size treemap. `node_modules` is one node per package (name, version,
   size, license), never per file.
4. **GPU when the count is truly large.** Above about 5k visible nodes, switch the
   canvas from DOM/SVG to WebGL (`@cosmos.gl/graph` 3.6 for force layout, or a simple
   instanced-quad renderer for fixed layouts). DOM handles hover cards only.
5. **Page data, not just pixels.** The Rust side keeps the graph in SQLite (already used
   for the cache) and the UI asks for one level at a time. A zoom-in loads its children
   on demand.

## Budgets

| Metric | Budget |
|---|---|
| First paint after analysis | < 300 ms for any level |
| Zoom transition | 16 ms per frame; work deferred with `requestIdleCallback` |
| UI process memory | < 400 MB at 200k files |
| JSON over IPC per level | < 2 MB (binary MessagePack above that) |
