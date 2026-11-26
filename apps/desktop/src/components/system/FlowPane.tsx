// The Flow map: pick a kind of workflow diagram and draw it.

import { useMemo, useState } from "react";
import type { ExampleRun, Feature } from "../../gen";
import type { Depth } from "../../lib/system";
import {
  aiWorkflow,
  availableKinds,
  contextWorkflow,
  requestWorkflow,
  WORKFLOW_KINDS,
  type WorkflowKind,
} from "../../lib/workflow";
import { useStore } from "../../store";
import { WorkflowView } from "./WorkflowView";

export function FlowPane({
  feature,
  run,
  depth,
}: {
  feature?: Feature;
  run?: ExampleRun;
  depth: Depth;
}) {
  const ix = useStore((s) => s.ix);
  const kinds = useMemo(() => (ix ? availableKinds(ix, run) : []), [ix, run]);
  const [want, setWant] = useState<WorkflowKind | null>(null);
  const kind = want && kinds.includes(want) ? want : kinds[0];
  const wf = useMemo(() => {
    if (!ix || !kind) return null;
    if (kind === "request") return requestWorkflow(ix, run);
    if (kind === "ai") return aiWorkflow(ix, feature, run);
    return contextWorkflow(ix);
  }, [ix, kind, run, feature]);

  if (!ix || !kind || !wf)
    return (
      <div className="ak-empty">
        <h2>Nothing to draw yet</h2>
        <p>
          A flow needs something traced in the code: an example run, AI stages, or memory the
          project keeps. None was found for this view.
        </p>
      </div>
    );
  return (
    <div className="ak-wf-pane">
      <div className="ak-wf-bar">
        <div className="ak-seg" role="radiogroup" aria-label="Flow kind">
          {WORKFLOW_KINDS.filter((k) => kinds.includes(k.id)).map((k) => (
            <button
              key={k.id}
              type="button"
              role="radio"
              aria-checked={kind === k.id}
              className={kind === k.id ? "on" : ""}
              title={k.hint}
              onClick={() => setWant(k.id)}
            >
              {k.label}
            </button>
          ))}
        </div>
        <span className="ak-muted ak-wf-headline">{wf.title}</span>
      </div>
      <div className="ak-wf-scroll">
        <WorkflowView wf={wf} depth={depth} />
      </div>
    </div>
  );
}
