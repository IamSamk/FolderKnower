import { ArrowDown, Lock, ShieldCheck } from "lucide-react";
import { DownloadButton } from "./DownloadButton";

const TIERS = [
  { name: "Frontend", eg: "React, Vue, Svelte", color: "var(--tier-ui)" },
  { name: "Gateway", eg: "routes, middleware", color: "var(--tier-gateway)" },
  { name: "Backend", eg: "services, handlers", color: "var(--tier-backend)" },
  { name: "Data", eg: "tables, schemas", color: "var(--tier-storage)" },
];

export function Hero() {
  return (
    <section className="relative pt-16 pb-24 sm:pt-24 bg-dot-pattern">
      <div className="max-w-6xl mx-auto px-6 grid grid-cols-1 lg:grid-cols-12 gap-12 items-center">
        <div className="lg:col-span-7 flex flex-col items-start">
          <h1 className="text-4xl sm:text-6xl font-extrabold tracking-tight leading-[1.05] text-[var(--ink)] mb-6">
            See how any codebase is{" "}
            <span className="underline decoration-[var(--accent)] decoration-wavy decoration-[3px] underline-offset-8">
              really built
            </span>
            .
          </h1>

          <p className="text-lg sm:text-xl text-[var(--ink-2)] font-medium leading-relaxed mb-8 max-w-xl">
            Drop a project folder on the window. FolderKnower reads the code on your computer and
            draws its tiers, routes, call paths and database tables. Each arrow links to the line of
            code that proves it.
          </p>

          <div className="flex flex-wrap items-center gap-4 mb-10">
            <DownloadButton className="text-base px-6 py-3.5" />
            <a href="#explorer" className="ak-btn text-base px-6 py-3.5">
              <span>See how it works</span>
              <ArrowDown size={18} />
            </a>
          </div>

          <ul className="flex flex-wrap items-center gap-x-6 gap-y-2 text-sm font-semibold text-[var(--ink-2)]">
            <li className="flex items-center gap-2">
              <Lock size={16} strokeWidth={2.25} className="text-[var(--ok)]" />
              Your code stays on your computer
            </li>
            <li className="flex items-center gap-2">
              <ShieldCheck size={16} strokeWidth={2.25} className="text-[var(--ok)]" />
              Free and open source
            </li>
          </ul>
        </div>

        <div className="lg:col-span-5 flex justify-center" aria-hidden="true">
          <div className="w-full max-w-sm flex flex-col">
            {TIERS.map((t, i) => (
              <div key={t.name} className="flex flex-col">
                {i > 0 && <div className="w-[2.5px] h-4 bg-[var(--line)] mx-auto" />}
                <div
                  className="ak-card p-4 flex items-center justify-between"
                  style={{ backgroundColor: t.color, color: "var(--on-tier)" }}
                >
                  <span className="text-sm font-extrabold uppercase tracking-wider">{t.name}</span>
                  <span className="font-mono text-xs font-bold opacity-70">{t.eg}</span>
                </div>
              </div>
            ))}
            <p className="mt-5 text-center font-mono text-xs font-bold text-[var(--ink-3)]">
              Tiers come from what the code does, not from folder names.
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
