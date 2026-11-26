import { ArrowDown, CheckCircle2, ShieldCheck, Zap } from "lucide-react";

export function Hero() {
  return (
    <section className="relative pt-16 pb-24 overflow-hidden bg-dot-pattern">
      <div className="max-w-6xl mx-auto px-6 grid grid-cols-1 lg:grid-cols-12 gap-12 items-center">
        {/* Left Column */}
        <div className="lg:col-span-7 flex flex-col items-start">
          <div className="ak-badge bg-[var(--mark)] text-[#1f1d36] mb-6 font-bold">
            v0.3.0 · Soft Neo-Brutalism · 100% Offline Core
          </div>

          <h1 className="text-4xl sm:text-6xl font-extrabold tracking-tight leading-[1.08] text-[var(--ink)] mb-6">
            See how any
            <br />
            codebase is{" "}
            <span className="underline decoration-[var(--mark)] decoration-wavy decoration-3">
              really built
            </span>
            .
          </h1>

          <p className="text-lg sm:text-xl text-[var(--ink-2)] font-medium leading-relaxed mb-8 max-w-xl">
            Drop in a local repository. FolderKnower walks it in seconds — completely offline — and
            draws the full architecture: system tiers, subsystems, API routes, call flows, and
            database schemas. Every edge backed by static evidence.
          </p>

          <div className="flex flex-wrap items-center gap-4 mb-10">
            <a href="#explorer" className="ak-btn ak-btn-accent text-base px-6 py-3.5">
              <span>Try Live Explorer</span>
              <ArrowDown size={18} />
            </a>
            <a href="#download" className="ak-btn text-base px-6 py-3.5">
              Download Desktop App
            </a>
          </div>

          <div className="flex items-center gap-6 text-xs sm:text-sm font-semibold text-[var(--ink-3)]">
            <div className="flex items-center gap-1.5">
              <ShieldCheck size={16} className="text-[#3dd66e]" />
              <span>Zero cloud uploads</span>
            </div>
            <div className="flex items-center gap-1.5">
              <Zap size={16} className="text-[var(--mark)]" />
              <span>Sub-second Rust engine</span>
            </div>
          </div>
        </div>

        {/* Right Column: 3D Stack Building Visualization */}
        <div className="lg:col-span-5 flex justify-center">
          <div className="relative w-full max-w-sm flex flex-col items-center">
            {/* Slabs */}
            <div className="w-full flex flex-col gap-3 relative z-10">
              {/* Frontend Slab */}
              <div className="ak-card bg-[var(--tier-ui)] text-[#1f1d36] p-4 flex items-center justify-between font-extrabold">
                <span className="text-sm uppercase tracking-wider">Frontend Tier</span>
                <span className="ak-badge bg-white text-xs">React 19 / TS</span>
              </div>

              {/* Connector Pin */}
              <div className="w-1 h-3 bg-[var(--line)] mx-auto rounded-full" />

              {/* Gateway / API Slab */}
              <div className="ak-card bg-[var(--tier-gateway)] text-[#1f1d36] p-4 flex items-center justify-between font-extrabold">
                <span className="text-sm uppercase tracking-wider">API Gateway</span>
                <span className="ak-badge bg-white text-xs">REST / RPC</span>
              </div>

              {/* Connector Pin */}
              <div className="w-1 h-3 bg-[var(--line)] mx-auto rounded-full" />

              {/* Backend Slab */}
              <div className="ak-card bg-[var(--tier-backend)] text-[#ffffff] p-4 flex items-center justify-between font-extrabold">
                <span className="text-sm uppercase tracking-wider">Services Tier</span>
                <span className="ak-badge bg-[#1f1d36] text-[#ffffff] text-xs">Node / Rust</span>
              </div>

              {/* Connector Pin */}
              <div className="w-1 h-3 bg-[var(--line)] mx-auto rounded-full" />

              {/* Storage Slab */}
              <div className="ak-card bg-[var(--tier-storage)] text-[#1f1d36] p-4 flex items-center justify-between font-extrabold">
                <span className="text-sm uppercase tracking-wider">Storage Tier</span>
                <span className="ak-badge bg-white text-xs">Postgres / SQLite</span>
              </div>
            </div>

            {/* Neo-brutalist stamp */}
            <div className="mt-6 ak-badge bg-[var(--surface)] text-[var(--ink)] text-xs flex items-center gap-1.5 font-bold">
              <CheckCircle2 size={14} className="text-[#3dd66e]" />
              <span>Exact static linkage • Zero hallucinated edges</span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
