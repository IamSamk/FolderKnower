import type { Node } from "@xyflow/react";
import { useMemo } from "react";
import { buildEr, buildFocus } from "../canvas/build";
import { Canvas } from "../canvas/Canvas";
import type { TierLabel } from "../gen";
import { type Focus, useStore } from "../store";

function focusKey(f: Focus): string {
  switch (f.level) {
    case "system":
      return "system";
    case "tier":
      return `tier:${f.tier}`;
    case "cluster":
      return f.clusterId;
    case "file":
      return f.fileId;
    case "function":
      return `focus:${f.fnId}`;
  }
}

export function ArchitectureView() {
  const ix = useStore((s) => s.ix);
  const stack = useStore((s) => s.stack);
  const drill = useStore((s) => s.drill);
  const reveal = useStore((s) => s.reveal);
  const select = useStore((s) => s.select);
  const focus = stack[stack.length - 1];
  const built = useMemo(() => (ix ? buildFocus(ix, focus) : null), [ix, focus]);
  if (!ix || !built) return null;

  const open = (n: Node) => {
    switch (n.type) {
      case "tier":
        drill({ level: "tier", tier: (n.data as { tier: TierLabel }).tier });
        break;
      case "cluster":
        drill({ level: "cluster", clusterId: n.id });
        break;
      case "file":
        if (!(n.data as { ghost?: boolean }).ghost) drill({ level: "file", fileId: n.id });
        else reveal(n.id);
        break;
      case "fn":
        if (focus.level !== "function" || focus.fnId !== n.id)
          drill({ level: "function", fnId: n.id });
        break;
      case "model":
        reveal(n.id);
        break;
    }
  };

  const navigate = (target: string) => {
    if (target.startsWith("tier:")) {
      const tier = target.slice(5) as TierLabel;
      useStore.setState({ stack: [{ level: "system" }, { level: "tier", tier }] });
      select(null);
      return;
    }
    if (ix.files.has(target)) {
      const fe = ix.files.get(target);
      if (fe)
        useStore.setState({
          stack: [
            { level: "system" },
            { level: "tier", tier: fe.tier },
            { level: "cluster", clusterId: fe.cluster.id },
          ],
        });
      select(target);
      return;
    }
    if (ix.fns.has(target) && focus.level === "function") {
      const fe = ix.fns.get(target);
      if (fe)
        useStore.setState({
          stack: [
            { level: "system" },
            { level: "tier", tier: fe.tier },
            { level: "cluster", clusterId: fe.clusterId },
            { level: "file", fileId: fe.file.id },
            { level: "function", fnId: target },
          ],
        });
      select(target);
      return;
    }
    reveal(target);
  };

  return <Canvas built={built} sceneKey={focusKey(focus)} onOpen={open} onNavigate={navigate} />;
}

export function ErView() {
  const ix = useStore((s) => s.ix);
  const reveal = useStore((s) => s.reveal);
  const built = useMemo(() => (ix ? buildEr(ix) : null), [ix]);
  if (!ix || !built) return null;
  if (built.nodes.length === 0)
    return (
      <div className="ak-empty">
        <div className="text-lg font-semibold">No data models found</div>
        <div className="ak-muted mt-1 max-w-md text-sm">
          Architekton reads SQLAlchemy, Django, SQLModel, Pydantic, Prisma, SQL DDL, TypeORM,
          Sequelize, zod, sqlx, Diesel and GORM models.
        </div>
      </div>
    );
  return (
    <Canvas
      built={built}
      sceneKey="er"
      onOpen={(n) => {
        const m = ix.models.get(n.id);
        if (m) reveal(m.fileId);
      }}
      onNavigate={(t) => useStore.getState().select(t)}
    />
  );
}
