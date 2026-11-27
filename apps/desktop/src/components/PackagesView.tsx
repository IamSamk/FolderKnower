// "Packages" view: the packages of a monorepo as cards, with what each one is made of
// (files per tier) and which other packages it imports or declares as dependencies.
// Every number comes from manifests and resolved imports; nothing is guessed.

import { useMemo, useState } from "react";
import type { TierLabel, WorkspacePackage } from "../gen";
import { TIER_COLOR, TIER_ORDER } from "../lib/theme";
import { useStore } from "../store";

const ECO: Record<string, string> = {
  npm: "npm",
  cargo: "Cargo",
  python: "Python",
  go: "Go",
  maven: "Maven",
  gradle: "Gradle",
  sbt: "sbt",
  dotnet: ".NET",
};

function compact(n: number): string {
  return n >= 1000 ? `${(n / 1000).toFixed(n >= 10000 ? 0 : 1)}k` : String(n);
}

export function PackagesView() {
  const ix = useStore((s) => s.ix);
  const packages = ix?.g.insights?.packages;
  const [picked, setPicked] = useState<string | null>(null);

  const byId = useMemo(() => new Map((packages ?? []).map((p) => [p.id, p])), [packages]);
  const usedBy = useMemo(() => {
    const m = new Map<string, WorkspacePackage[]>();
    for (const p of packages ?? [])
      for (const l of p.dependsOn) m.set(l.target, [...(m.get(l.target) ?? []), p]);
    return m;
  }, [packages]);

  if (!packages?.length) return null;
  const max = Math.max(...packages.map((p) => p.files));
  const related = new Set<string>();
  if (picked) {
    related.add(picked);
    for (const l of byId.get(picked)?.dependsOn ?? []) related.add(l.target);
    for (const p of usedBy.get(picked) ?? []) related.add(p.id);
  }

  return (
    <section className="pk-view" aria-label="Packages">
      <header className="pk-head">
        <h2>{packages.length} packages</h2>
        <p>
          Each card is a folder with its own manifest. Click one to see what it uses and what uses
          it.
        </p>
      </header>
      <div className="pk-grid">
        {packages.map((p) => {
          const dim = picked !== null && !related.has(p.id);
          const tiers = TIER_ORDER.filter((t) => p.tiers[t]) as TierLabel[];
          return (
            <button
              type="button"
              key={p.id}
              className={`pk-card${picked === p.id ? " on" : ""}${dim ? " dim" : ""}`}
              onClick={() => setPicked(picked === p.id ? null : p.id)}
              aria-pressed={picked === p.id}
            >
              <span className="pk-top">
                <strong className="pk-name">{p.name}</strong>
                <span className="pk-eco">{ECO[p.ecosystem] ?? p.ecosystem}</span>
              </span>
              <span className="pk-root">{p.root || "(repository root)"}</span>
              <span className="pk-bar" style={{ width: `${Math.max(8, (p.files / max) * 100)}%` }}>
                {tiers.map((t) => (
                  <i
                    key={t}
                    title={`${t}: ${p.tiers[t]} files`}
                    style={{ flexGrow: p.tiers[t], background: TIER_COLOR[t] }}
                  />
                ))}
              </span>
              <span className="pk-stats">
                {compact(p.files)} files · {compact(p.lines)} lines
              </span>
              {p.dependsOn.length > 0 && (
                <span className="pk-uses">
                  uses{" "}
                  {p.dependsOn.slice(0, 4).map((l) => (
                    <em key={l.target}>
                      {byId.get(l.target)?.name ?? l.target}
                      {l.imports > 0 ? ` ${l.imports}` : ""}
                    </em>
                  ))}
                  {p.dependsOn.length > 4 && <em>+{p.dependsOn.length - 4}</em>}
                </span>
              )}
            </button>
          );
        })}
      </div>
      <footer className="pk-legend">
        {TIER_ORDER.map((t) => (
          <span key={t}>
            <i style={{ background: TIER_COLOR[t] }} />
            {t}
          </span>
        ))}
        <span className="pk-note">
          A number after a name counts import edges; none means declared in the manifest only.
        </span>
      </footer>
    </section>
  );
}
