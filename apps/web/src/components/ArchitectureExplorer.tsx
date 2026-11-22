"use client";

import { useState } from "react";
import demoData from "@/data/demo-architecture.json";
import { Upload, FileCode, Route, Database, CheckCircle2 } from "lucide-react";

interface FileNode {
  path: string;
  role?: string;
  tierConfidence?: number;
}

interface Cluster {
  id: string;
  name?: string;
  label?: string;
  files: FileNode[];
}

interface Tier {
  id: string;
  label: string;
  clusters: Cluster[];
}

export function ArchitectureExplorer() {
  const [data, setData] = useState<any>(demoData);
  const [selectedTier, setSelectedTier] = useState<string>("all");
  const [selectedFile, setSelectedFile] = useState<FileNode | null>(null);

  const tiers: Tier[] = data.tiers || [];
  const routes = data.routes || [];
  const crossEdges = data.crossTierEdges || [];
  const dbModels = data.dataModels || data.database?.models || [];

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (event) => {
      try {
        const json = JSON.parse(event.target?.result as string);
        if (json.tiers) {
          setData(json);
          setSelectedTier("all");
          setSelectedFile(null);
        }
      } catch (err) {
        alert("Invalid architecture.json file format");
      }
    };
    reader.readAsText(file);
  };

  const filteredTiers = selectedTier === "all"
    ? tiers
    : tiers.filter((t) => t.id === selectedTier || t.label.toLowerCase() === selectedTier.toLowerCase());

  return (
    <section id="explorer" className="py-20 border-t-[2.5px] border-[var(--line)] bg-[var(--surface-2)]">
      <div className="max-w-6xl mx-auto px-6">
        {/* Section Header */}
        <div className="flex flex-col md:flex-row md:items-end justify-between gap-6 mb-12">
          <div>
            <div className="ak-badge bg-[var(--tier-ui)] text-[#1f1d36] mb-3 font-bold">
              Live Interactive Architecture Viewer
            </div>
            <h2 className="text-3xl sm:text-4xl font-extrabold text-[var(--ink)]">
              Interactive Codebase Explorer
            </h2>
            <p className="text-[var(--ink-2)] text-base font-medium mt-2 max-w-xl">
              Exploring: <code className="bg-[var(--surface)] px-2 py-0.5 border border-[var(--line)] rounded font-bold text-xs">{data.summary?.headline || data.repoName || "fullstack-todo"}</code>.
              Load your own <code>architecture.json</code> from <code>architekton index</code> to inspect any repo.
            </p>
          </div>

          {/* Upload Button */}
          <label className="ak-btn ak-btn-mark cursor-pointer flex items-center gap-2">
            <Upload size={16} />
            <span>Upload architecture.json</span>
            <input
              type="file"
              accept=".json"
              className="hidden"
              onChange={handleFileUpload}
            />
          </label>
        </div>

        {/* Tier filter pills */}
        <div className="flex flex-wrap items-center gap-2.5 mb-8">
          <button
            type="button"
            onClick={() => setSelectedTier("all")}
            className={`ak-btn text-xs px-3.5 py-1.5 ${selectedTier === "all" ? "ak-btn-accent" : "bg-[var(--surface)]"}`}
          >
            All Tiers ({tiers.length})
          </button>
          {tiers.map((t) => (
            <button
              key={t.id || t.label}
              type="button"
              onClick={() => setSelectedTier(t.id)}
              className={`ak-btn text-xs px-3.5 py-1.5 ${
                selectedTier === t.id ? "ak-btn-accent" : "bg-[var(--surface)]"
              }`}
            >
              {t.label || t.id}
            </button>
          ))}
        </div>

        {/* Explorer Workspace */}
        <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
          {/* Left: Tiers & Clusters */}
          <div className="lg:col-span-8 flex flex-col gap-6">
            {filteredTiers.map((tier) => (
              <div key={tier.id || tier.label} className="ak-card p-5">
                <div className="flex items-center justify-between pb-3 mb-4 border-b-2 border-[var(--line)]">
                  <span className="font-extrabold text-sm uppercase tracking-wider text-[var(--ink)]">
                    {tier.label || tier.id} Tier
                  </span>
                  <span className="ak-badge bg-[var(--surface-2)] text-xs">
                    {tier.clusters?.reduce((acc, c) => acc + (c.files?.length || 0), 0) || 0} files
                  </span>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  {tier.clusters?.map((cluster) => (
                    <div
                      key={cluster.id || cluster.name}
                      className="p-3.5 rounded-lg border-2 border-[var(--line)] bg-[var(--surface-2)] flex flex-col gap-2"
                    >
                      <div className="font-bold text-xs text-[var(--ink-2)] uppercase tracking-wide">
                        {cluster.label || cluster.name || cluster.id}
                      </div>
                      <div className="flex flex-wrap gap-1.5">
                        {cluster.files?.map((file) => (
                          <button
                            key={file.path}
                            type="button"
                            onClick={() => setSelectedFile(file)}
                            className={`px-2 py-1 rounded text-xs font-mono font-medium border border-[var(--line)] transition-all ${
                              selectedFile?.path === file.path
                                ? "bg-[var(--mark)] text-[#1f1d36] font-bold shadow-[2px_2px_0_var(--shade)]"
                                : "bg-[var(--surface)] hover:bg-[var(--mark)] text-[var(--ink)]"
                            }`}
                          >
                            {file.path.split("/").pop()}
                          </button>
                        ))}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            ))}

            {/* Cross-Tier Connections */}
            <div className="ak-card p-5">
              <div className="font-extrabold text-sm uppercase tracking-wider text-[var(--ink)] mb-4 pb-2 border-b-2 border-[var(--line)] flex items-center gap-2">
                <Route size={16} />
                <span>Static Call Flows & API Routes ({routes.length} routes, {crossEdges.length} cross-tier edges)</span>
              </div>
              <div className="flex flex-col gap-2.5 max-h-56 overflow-y-auto pr-2">
                {routes.slice(0, 5).map((r: any, idx: number) => (
                  <div
                    key={r.id || `${r.method || 'ROUTE'}-${r.path || ''}-${idx}`}
                    className="p-2.5 rounded border-2 border-[var(--line)] bg-[var(--surface)] text-xs flex items-center justify-between font-mono"
                  >
                    <div className="flex items-center gap-2">
                      <span className="ak-badge bg-[var(--tier-gateway)] text-[#1f1d36] px-1.5 py-0.5 text-[10px]">
                        {r.method || "GET"}
                      </span>
                      <span className="font-bold text-[var(--ink)]">{r.path}</span>
                    </div>
                    <span className="text-[var(--ink-3)] text-[11px] truncate max-w-[200px]">
                      {r.handlerFile || r.handler || "handler"}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* Right: Inspector & Details */}
          <div className="lg:col-span-4 flex flex-col gap-6">
            <div className="ak-card p-5 sticky top-24">
              <div className="font-extrabold text-sm uppercase tracking-wider text-[var(--ink)] pb-3 mb-4 border-b-2 border-[var(--line)] flex items-center gap-2">
                <FileCode size={16} />
                <span>Inspector</span>
              </div>

              {selectedFile ? (
                <div className="flex flex-col gap-4">
                  <div>
                    <span className="text-xs font-bold text-[var(--ink-3)] uppercase tracking-wider">File Path</span>
                    <div className="font-mono font-bold text-sm text-[var(--ink)] break-all mt-1 bg-[var(--surface-2)] p-2 rounded border border-[var(--line)]">
                      {selectedFile.path}
                    </div>
                  </div>

                  <div className="grid grid-cols-2 gap-3">
                    <div className="p-2.5 rounded border border-[var(--line)] bg-[var(--surface-2)]">
                      <span className="text-[11px] font-bold text-[var(--ink-3)] uppercase">Confidence</span>
                      <div className="text-base font-extrabold text-[var(--accent)] mt-0.5">
                        {Math.round((selectedFile.tierConfidence ?? 1) * 100)}%
                      </div>
                    </div>
                    <div className="p-2.5 rounded border border-[var(--line)] bg-[var(--surface-2)]">
                      <span className="text-[11px] font-bold text-[var(--ink-3)] uppercase">Role</span>
                      <div className="text-xs font-bold text-[var(--ink)] mt-0.5 truncate">
                        {selectedFile.role || "Component"}
                      </div>
                    </div>
                  </div>

                  <div className="ak-badge bg-[#5fe38a] text-[#004d1a] justify-center py-2 font-bold">
                    <CheckCircle2 size={15} />
                    <span>Evidence-backed AST link</span>
                  </div>
                </div>
              ) : (
                <div className="text-center py-10 text-[var(--ink-3)] text-sm font-medium">
                  Click any file node on the left to inspect its extracted architecture metadata.
                </div>
              )}

              {/* Database Entities Preview */}
              {dbModels.length > 0 && (
                <div className="mt-6 pt-4 border-t-2 border-[var(--line)]">
                  <div className="flex items-center gap-1.5 font-bold text-xs uppercase text-[var(--ink-2)] mb-3">
                    <Database size={14} />
                    <span>Database Entities ({dbModels.length})</span>
                  </div>
                  <div className="flex flex-wrap gap-1.5">
                    {dbModels.map((m: any) => (
                      <span
                        key={m.id || m.name}
                        className="ak-badge bg-[var(--tier-storage)] text-[#1f1d36] text-[11px]"
                      >
                        {m.name}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
