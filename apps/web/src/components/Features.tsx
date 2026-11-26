import { Database, Layers, Lock, MousePointerClick, Sparkles, Terminal } from "lucide-react";

const FEATURES = [
  {
    icon: Layers,
    title: "Zoom from system to function",
    desc: "Five levels: tiers, subsystems, files, calls, then one function with its Big-O. Double-click to go in, use the breadcrumb to come out.",
  },
  {
    icon: MousePointerClick,
    title: "Follow a button to its table",
    desc: "Point at any button in the UI map. See the path: handler, API call, route, service, then the database table it writes.",
  },
  {
    icon: Database,
    title: "Database tables, drawn for you",
    desc: "Tables, columns, keys and relations from SQL, Prisma, SQLAlchemy, Django, GORM and more, with the schemas that mirror them.",
  },
  {
    icon: Lock,
    title: "Offline by design",
    desc: "Parsing runs on your computer with tree-sitter. No upload, no account, no telemetry.",
  },
  {
    icon: Sparkles,
    title: "Optional local AI",
    desc: "A small model on your computer explains what you selected. It never draws arrows. Use the built-in one or your own Ollama or LM Studio.",
  },
  {
    icon: Terminal,
    title: "Same result every time",
    desc: "Run fknow index . in CI. The same code always gives the same architecture.json, so you can diff it in a pull request.",
  },
];

export function Features() {
  return (
    <section
      id="features"
      className="scroll-mt-16 py-24 border-t-[2.5px] border-[var(--line)] bg-[var(--paper)]"
    >
      <div className="max-w-6xl mx-auto px-6">
        <div className="max-w-2xl mb-12">
          <p className="ak-eyebrow mb-3">Features</p>
          <h2 className="text-3xl sm:text-4xl font-extrabold tracking-tight text-[var(--ink)]">
            Understand a new codebase in minutes
          </h2>
          <p className="text-[var(--ink-2)] text-base font-medium mt-3">
            FolderKnower finds each part by what its code does. It does not guess from folder names
            or ask a cloud model.
          </p>
        </div>

        <ul className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {FEATURES.map(({ icon: Icon, title, desc }) => (
            <li key={title} className="ak-card p-6">
              <Icon size={24} strokeWidth={2.25} className="text-[var(--accent)] mb-4" />
              <h3 className="text-lg font-extrabold text-[var(--ink)] mb-2">{title}</h3>
              <p className="text-sm text-[var(--ink-2)] leading-relaxed font-medium">{desc}</p>
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
