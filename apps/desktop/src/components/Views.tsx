import type { Node } from "@xyflow/react";
import { useMemo } from "react";
import { buildEr, buildFocus } from "../canvas/build";
import { Canvas, type NavOrigin } from "../canvas/Canvas";
import type { TierLabel } from "../gen";
import { TIER_COLOR } from "../lib/theme";
import { type Focus, useStore } from "../store";
import { originOf, PortalScene } from "./Portal";

export function focusKey(f: Focus): string {
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

function nodeLabel(n: Node): string {
  const d = n.data as Record<string, unknown>;
  if (n.type === "tier") return String(d.tier);
  if (n.type === "cluster") return (d.cluster as { name: string }).name;
  if (n.type === "file")
    return ((d.file as { path: string }).path.split("/").pop() ?? "") as string;
  if (n.type === "fn") return `${(d.fn as { name: string }).name}()`;
  if (n.type === "model") return (d.model as { name: string }).name;
  return "";
}

export function ArchitectureView() {
  const ix = useStore((s) => s.ix);
  const stack = useStore((s) => s.stack);
  const select = useStore((s) => s.select);
  const go = useStore((s) => s.go);
  const focus = stack[stack.length - 1];
  const built = useMemo(() => (ix ? buildFocus(ix, focus) : null), [ix, focus]);
  if (!ix || !built) return null;

  const push = (f: Focus) => useStore.setState((s) => ({ stack: [...s.stack, f] }));

  const open = (n: Node, el: Element | null) => {
    const tier = (n.data as { tier?: TierLabel }).tier;
    const origin = originOf(el, nodeLabel(n), tier ? TIER_COLOR[tier] : undefined);
    switch (n.type) {
      case "tier":
        go(origin, "in", () => push({ level: "tier", tier: (n.data as { tier: TierLabel }).tier }));
        break;
      case "cluster":
        go(origin, "in", () => push({ level: "cluster", clusterId: n.id }));
        break;
      case "file":
        if (!(n.data as { ghost?: boolean }).ghost)
          go(origin, "in", () => push({ level: "file", fileId: n.id }));
        break;
      case "fn":
        if (focus.level !== "function" || focus.fnId !== n.id)
          go(origin, "in", () => push({ level: "function", fnId: n.id }));
        break;
      case "model":
        go(origin, "tab", () => useStore.getState().reveal(n.id));
        break;
    }
  };

  const navigate = (target: string, at?: NavOrigin) => {
    const origin = {
      x: at?.x ?? 400,
      y: at?.y ?? 300,
      label: ix.label(target),
      color: TIER_COLOR[ix.tierOf(target) ?? "Shared"],
    };
    if (target.startsWith("tier:")) {
      const tier = target.slice(5) as TierLabel;
      go({ ...origin, label: tier, color: TIER_COLOR[tier] }, "in", () => {
        useStore.setState({ stack: [{ level: "system" }, { level: "tier", tier }] });
        select(null);
      });
      return;
    }
    const fe = ix.files.get(target);
    if (fe) {
      go(origin, "in", () => {
        useStore.setState({
          stack: [
            { level: "system" },
            { level: "tier", tier: fe.tier },
            { level: "cluster", clusterId: fe.cluster.id },
          ],
        });
        select(target);
      });
      return;
    }
    const fn = ix.fns.get(target);
    if (fn) {
      go(origin, "in", () => {
        const base: Focus[] = [
          { level: "system" },
          { level: "tier", tier: fn.tier },
          { level: "cluster", clusterId: fn.clusterId },
          { level: "file", fileId: fn.file.id },
        ];
        if (focus.level === "function") base.push({ level: "function", fnId: target });
        useStore.setState({ stack: base });
        select(target);
      });
      return;
    }
    go(origin, "tab", () => useStore.getState().reveal(target));
  };

  return (
    <PortalScene sceneKey={focusKey(focus)}>
      <Canvas built={built} onOpen={open} onNavigate={navigate} />
    </PortalScene>
  );
}

export function ErView() {
  const ix = useStore((s) => s.ix);
  const reveal = useStore((s) => s.reveal);
  const go = useStore((s) => s.go);
  const built = useMemo(() => (ix ? buildEr(ix) : null), [ix]);
  if (!ix || !built) return null;
  if (built.nodes.length === 0)
    return (
      <div className="ak-empty">
        <h2>No data models found</h2>
        <p>
          FolderKnower reads SQLAlchemy, Django, SQLModel, Pydantic, Prisma, SQL DDL, TypeORM,
          Sequelize, zod, sqlx, Diesel and GORM models. Open a repository that uses one of them.
        </p>
      </div>
    );
  return (
    <Canvas
      built={built}
      onOpen={(n, el) => {
        const m = ix.models.get(n.id);
        if (m) go(originOf(el, m.name, TIER_COLOR.Storage), "in", () => reveal(m.fileId));
      }}
      onNavigate={(t) => useStore.getState().select(t)}
    />
  );
}
