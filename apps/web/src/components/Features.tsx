import { Binary, Brain, Cpu, Search, Shield, Terminal } from "lucide-react";

export function Features() {
  const features = [
    {
      icon: <Shield size={24} className="text-[#3dd66e]" />,
      title: "100% Offline Static Analysis",
      desc: "Your source code never leaves your workstation. No external API calls, no cloud indexers, zero telemetry leaks.",
    },
    {
      icon: <Cpu size={24} className="text-[var(--accent)]" />,
      title: "Sub-Second Rust Engine",
      desc: "Built in Rust with tree-sitter. Parses and classifies thousands of source files in under 200 milliseconds.",
    },
    {
      icon: <Binary size={24} className="text-[var(--mark)]" />,
      title: "Zero Hallucinated Edges",
      desc: "Every arrow in the graph is backed by verifiable AST evidence — exact import paths, REST endpoints, and schema definitions.",
    },
    {
      icon: <Search size={24} className="text-[#ff7633]" />,
      title: "Systems Ontology",
      desc: "Categorizes systems into OS, Network, Cloud, and Data layers using a codified taxonomy of architectural patterns.",
    },
    {
      icon: <Brain size={24} className="text-[var(--tier-ui)]" />,
      title: "Local AI Explanations",
      desc: "Optional llama.cpp & Ollama integration to explain architectural choices in plain English without sending data out.",
    },
    {
      icon: <Terminal size={24} className="text-[#2dd4bf]" />,
      title: "Deterministic CLI Export",
      desc: "Run `fknow index . --out architecture.json` in CI or terminals. Same input repository always yields identical JSON.",
    },
  ];

  return (
    <section
      id="features"
      className="py-24 border-t-[2.5px] border-[var(--line)] bg-[var(--paper)]"
    >
      <div className="max-w-6xl mx-auto px-6">
        <div className="text-center max-w-2xl mx-auto mb-16">
          <div className="ak-badge bg-[var(--mark)] text-[#1f1d36] mb-3 font-bold">
            Core Engineering Principles
          </div>
          <h2 className="text-3xl sm:text-4xl font-extrabold text-[var(--ink)]">
            Built for Architects & Engineers
          </h2>
          <p className="text-[var(--ink-2)] text-base font-medium mt-3">
            Traditional tools guess architectures with flaky AI prompts. FolderKnower analyzes real
            AST code links.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {features.map((feat, idx) => (
            <div
              key={idx}
              className="ak-card ak-card-interactive p-6 flex flex-col items-start justify-between"
            >
              <div>
                <div className="w-12 h-12 rounded-lg bg-[var(--surface-2)] border-2 border-[var(--line)] flex items-center justify-center mb-5 shadow-[2px_2px_0_var(--shade)]">
                  {feat.icon}
                </div>
                <h3 className="text-lg font-extrabold text-[var(--ink)] mb-2.5">{feat.title}</h3>
                <p className="text-sm text-[var(--ink-2)] leading-relaxed font-medium">
                  {feat.desc}
                </p>
              </div>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
