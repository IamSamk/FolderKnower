import { Handle, type NodeProps, Position } from "@xyflow/react";
import { motion } from "motion/react";
import type { DataModel, FunctionNode, SourceFileNode, SubsystemCluster, TierLabel } from "../gen";
import { CONCEPT_BY_ID } from "../lib/backend";
import { BIGO_COLOR, LAYER_COLOR, ROLE_ICON, TIER_BLURB, TIER_COLOR } from "../lib/theme";

type Common = { _i?: number; dir?: "RIGHT" | "DOWN"; dim?: boolean; hot?: boolean };

function Handles({ dir }: { dir?: "RIGHT" | "DOWN" }) {
  const [t, s] = dir === "DOWN" ? [Position.Top, Position.Bottom] : [Position.Left, Position.Right];
  return (
    <>
      <Handle type="target" position={t} className="ak-handle" isConnectable={false} />
      <Handle type="source" position={s} className="ak-handle" isConnectable={false} />
    </>
  );
}

function Shell({
  color,
  children,
  data,
  selected,
  className = "",
  ghost = false,
}: {
  color: string;
  children: React.ReactNode;
  data: Common;
  selected?: boolean;
  className?: string;
  ghost?: boolean;
}) {
  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.92, y: 10 }}
      animate={{ opacity: data.dim ? 0.28 : 1, scale: 1, y: 0 }}
      transition={{
        type: "spring",
        stiffness: 260,
        damping: 26,
        delay: Math.min((data._i ?? 0) * 0.025, 0.5),
      }}
      className={`ak-node ${ghost ? "ak-ghost" : ""} ${selected || data.hot ? "ak-node-selected" : ""} ${className}`}
      style={{ "--c": color } as React.CSSProperties}
    >
      <Handles dir={data.dir} />
      {children}
    </motion.div>
  );
}

export function TierNodeView({ data, selected }: NodeProps) {
  const d = data as Common & {
    tier: TierLabel;
    files: number;
    fns: number;
    clusters: number;
    lines: number;
    top: string[];
    roles: string[];
  };
  const c = TIER_COLOR[d.tier];
  return (
    <Shell color={c} data={d} selected={selected} className="ak-tier">
      <div className="ak-tier-glow" />
      <div className="flex items-center justify-between">
        <span className="ak-tier-title">{d.tier}</span>
        <span className="ak-pill">{d.clusters} subsystems</span>
      </div>
      <div className="ak-muted mt-1 text-[12px]">{TIER_BLURB[d.tier]}</div>
      <div className="mt-3 grid grid-cols-3 gap-2 text-center">
        <Stat n={d.files} l="files" />
        <Stat n={d.fns} l="functions" />
        <Stat n={d.lines} l="lines" />
      </div>
      <div className="mt-3 flex flex-wrap gap-1">
        {d.top.map((t) => (
          <span key={t} className="ak-chip">
            {t}
          </span>
        ))}
      </div>
    </Shell>
  );
}

function Stat({ n, l }: { n: number; l: string }) {
  return (
    <div className="ak-stat">
      <div className="ak-stat-n">{n.toLocaleString()}</div>
      <div className="ak-stat-l">{l}</div>
    </div>
  );
}

export function ClusterNodeView({ data, selected }: NodeProps) {
  const d = data as Common & { cluster: SubsystemCluster; tier: TierLabel };
  const c = TIER_COLOR[d.tier];
  const files = d.cluster.files;
  return (
    <Shell color={c} data={d} selected={selected} className="ak-cluster">
      <div className="flex items-center justify-between">
        <span className="ak-title">{d.cluster.name}</span>
        <span className="ak-pill">{files.length} files</span>
      </div>
      <div className="ak-muted text-[11px]">
        {d.cluster.method.algo} · {files.reduce((a, f) => a + f.functions.length, 0)} functions
      </div>
      <ul className="mt-2 space-y-[2px]">
        {files.slice(0, 8).map((f) => (
          <li key={f.id} className="ak-row">
            <span className="ak-role" title={f.role ?? ""}>
              {ROLE_ICON[f.role ?? ""] ?? "·"}
            </span>
            <span className="truncate">{f.path.split("/").pop()}</span>
            <span className="ak-muted ml-auto text-[10px]">{f.functions.length}ƒ</span>
          </li>
        ))}
        {files.length > 8 && <li className="ak-muted text-[11px]">+{files.length - 8} more</li>}
      </ul>
    </Shell>
  );
}

export function FileNodeView({ data, selected }: NodeProps) {
  const d = data as Common & { file: SourceFileNode; tier: TierLabel; ghost?: boolean };
  const f = d.file;
  const c = TIER_COLOR[d.tier];
  const name = f.path.split("/").pop();
  const dir = f.path.split("/").slice(0, -1).join("/");
  return (
    <Shell color={c} data={d} selected={selected} ghost={d.ghost}>
      <div className="flex items-center gap-2">
        <span className="ak-role text-[14px]">{ROLE_ICON[f.role ?? ""] ?? "◇"}</span>
        <span className="ak-title truncate">{name}</span>
        <span className="ak-lang ml-auto">{f.language}</span>
      </div>
      <div className="ak-muted truncate text-[11px]">{dir || "."}</div>
      <div className="mt-2 flex items-center gap-2 text-[11px]">
        {f.role && <span className="ak-chip">{f.role}</span>}
        <span className="ak-muted">{f.functions.length} fn</span>
        <span className="ak-muted">{f.lines} loc</span>
      </div>
      <div
        className="ak-conf mt-2"
        title={`tier confidence ${Math.round(f.tierConfidence * 100)}%`}
      >
        <div style={{ width: `${Math.round(f.tierConfidence * 100)}%` }} />
      </div>
    </Shell>
  );
}

export function FnNodeView({ data, selected }: NodeProps) {
  const d = data as Common & {
    fn: FunctionNode;
    file: SourceFileNode;
    tier: TierLabel;
    ghost?: boolean;
    center?: boolean;
  };
  const f = d.fn;
  const c = TIER_COLOR[d.tier];
  if (d.ghost) {
    return (
      <Shell color={c} data={d} selected={selected} ghost>
        <div className="ak-mono truncate text-[12.5px] font-semibold">{f.name}()</div>
        <div className="ak-muted truncate text-[10.5px]">
          {d.tier} · {d.file.path.split("/").pop()}
        </div>
      </Shell>
    );
  }
  const params = f.signature.parameters.map((p) => p.name).join(", ");
  const layers = [...new Set(f.ontologyTags.map((t) => t.layer))];
  return (
    <Shell color={c} data={d} selected={selected} className={d.center ? "ak-center" : ""}>
      <div className="ak-mono truncate text-[13px] font-semibold">
        {f.name}
        <span className="ak-muted font-normal">({params})</span>
      </div>
      {f.signature.returnType && (
        <div className="ak-mono ak-muted truncate text-[11px]">→ {f.signature.returnType}</div>
      )}
      <div className="mt-2 flex flex-wrap items-center gap-1.5">
        <span
          className="ak-bigo"
          style={{ "--b": BIGO_COLOR[f.complexity.timeBigO] } as React.CSSProperties}
        >
          {f.complexity.timeBigO}
        </span>
        <span className="ak-chip">cc {f.complexity.cyclomatic}</span>
        {f.visibility !== "public" && <span className="ak-chip">{f.visibility}</span>}
        <span className="ml-auto flex gap-1">
          {layers.map((l) => (
            <span key={l} className="ak-dot" style={{ background: LAYER_COLOR[l] }} title={l} />
          ))}
        </span>
      </div>
      {d.center && f.ontologyTags.length > 0 && (
        <div className="mt-2 flex flex-wrap gap-1">
          {f.ontologyTags.map((t) => (
            <span
              key={t.ruleId}
              className="ak-chip"
              title={CONCEPT_BY_ID.get(t.ruleId)?.explain}
              style={{ borderColor: LAYER_COLOR[t.layer] }}
            >
              {t.concept}
            </span>
          ))}
        </div>
      )}
    </Shell>
  );
}

export function PortalNodeView({ data, selected }: NodeProps) {
  const d = data as Common & { tier: TierLabel; label: string; sub: string };
  return (
    <Shell color={TIER_COLOR[d.tier]} data={d} selected={selected} className="ak-portal">
      <div className="flex items-center gap-2">
        <span className="ak-portal-ring" />
        <span className="ak-title truncate">{d.label}</span>
      </div>
      <div className="ak-muted truncate text-[11px]">{d.sub}</div>
    </Shell>
  );
}

export function ModelNodeView({ data, selected }: NodeProps) {
  const d = data as Common & { model: DataModel };
  const m = d.model;
  const isSchema = m.kind === "schema";
  const color = isSchema ? "#A78BFA" : "#10B981";
  return (
    <Shell
      color={color}
      data={d}
      selected={selected}
      className={`ak-table ${isSchema ? "ak-schema" : ""}`}
    >
      <div className="ak-table-head">
        <span className="ak-title">{m.name}</span>
        <span className="ak-pill">{isSchema ? "schema" : m.kind}</span>
      </div>
      <div className="ak-muted px-3 pb-1 text-[10.5px]">
        {m.framework}
        {m.table ? ` · ${m.table}` : ""}
      </div>
      <ul>
        {m.fields.slice(0, 14).map((f) => (
          <li key={f.name} className="ak-field">
            <span className="ak-key">{f.primaryKey ? "PK" : f.references ? "FK" : ""}</span>
            <span className="ak-mono truncate">{f.name}</span>
            <span className="ak-mono ak-muted ml-auto truncate pl-2 text-[11px]">
              {f.type ?? ""}
              {f.nullable ? "?" : ""}
            </span>
          </li>
        ))}
        {m.fields.length > 14 && (
          <li className="ak-muted px-3 py-1 text-[11px]">+{m.fields.length - 14} more</li>
        )}
      </ul>
    </Shell>
  );
}

export const nodeTypes = {
  tier: TierNodeView,
  cluster: ClusterNodeView,
  file: FileNodeView,
  fn: FnNodeView,
  portal: PortalNodeView,
  model: ModelNodeView,
};
