# 11. Workflow: the ideal (clean) path

**Status: done.**

- Outline: `ark-parse/src/enrich.rs` (`FlowBuilder`), stored on `FunctionNode.flow`.
- Marking: `lib/insights.ts` `idealPath()` and `idealSteps()`.
- UI: `components/insights/Workflow.tsx`, opened from the **Workflow** button in the top
  bar, or from "Show the clean path through this function" in the Inspector.

## The outline

One `FlowNode` per step: `branch`, `else`, `loop`, `try`, `catch`, `finally`, `switch`,
`case`, `call` (a statement that calls something), `return` and `raise`. Each node has a
line, a clipped label and children. It is capped at 120 nodes and depth 6 per function.

Error exits are flagged (`error: true`) by syntax:

- `raise` and `throw`;
- `panic!`, `os.Exit`, `process.exit`, `sys.exit`;
- `return Err(..)`, `return nil, err`, `return errors.New(..)`;
- the Rust `?` operator, annotated "may return early with ?".

## The clean path

`idealPath` walks the outline:

- in an if/elif/else chain, take the first arm that cannot raise; a guard clause
  (`if bad: raise`) is simply not taken;
- enter `try` and skip `catch`;
- stop at the first normal `return`.

Marks: **ideal** (green), **branch** (amber, the decision taken), **error** (red) and
**skipped** (faded, dashed).

## At every zoom level

The drawer lists the functions at the current focus, the most complex first:

| Focus | Functions listed |
|---|---|
| System | all functions |
| Tier | functions in that tier |
| Subsystem | functions in that subsystem |
| File | functions in that file |
| Function | just that one |

"Walk the clean path" steps through it with the current step highlighted and scrolled
into view.

## What we can't know statically

The footer says it plainly: red steps are where a run *can* stop early. Whether it does
depends on the input at run time, which reading the code cannot know. Doc 05 replaces
"can" with "did" when a trace is connected.
