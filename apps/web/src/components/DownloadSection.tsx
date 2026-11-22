import { Download, Monitor, Terminal, Apple, Laptop } from "lucide-react";

export function DownloadSection() {
  const releases = [
    {
      os: "Windows",
      icon: <Monitor size={28} />,
      desc: "Windows 10 / 11 (x64)",
      primaryBtn: "Download .exe installer",
      primaryUrl: "https://github.com/IamSamk/Architekton/releases/latest/download/Architekton_0.3.0_x64-setup.exe",
      secondaryText: "Download .msi package",
      secondaryUrl: "https://github.com/IamSamk/Architekton/releases/latest/download/Architekton_0.3.0_x64_en-US.msi",
      badge: "In-App Updater enabled",
    },
    {
      os: "macOS",
      icon: <Apple size={28} />,
      desc: "Universal (Apple Silicon & Intel)",
      primaryBtn: "Download .dmg",
      primaryUrl: "https://github.com/IamSamk/Architekton/releases/latest/download/Architekton_0.3.0_universal.dmg",
      secondaryText: "Right click → Open on first run",
      badge: "Universal Binary",
    },
    {
      os: "Linux",
      icon: <Laptop size={28} />,
      desc: "x86-64 AppImage, deb, rpm",
      primaryBtn: "View Linux Packages",
      primaryUrl: "https://github.com/IamSamk/Architekton/releases/latest",
      secondaryText: "Available in .AppImage, .deb, .rpm",
      badge: "Desktop Standard",
    },
  ];

  return (
    <section id="download" className="py-24 border-t-[2.5px] border-[var(--line)] bg-[var(--surface-2)]">
      <div className="max-w-6xl mx-auto px-6">
        <div className="text-center max-w-2xl mx-auto mb-16">
          <div className="ak-badge bg-[var(--tier-ui)] text-[#1f1d36] mb-3 font-bold">
            Releases & Installers
          </div>
          <h2 className="text-3xl sm:text-4xl font-extrabold text-[var(--ink)]">
            Get Architekton Desktop
          </h2>
          <p className="text-[var(--ink-2)] text-base font-medium mt-3">
            Available as a native desktop application with in-app self updates and as a standalone Rust CLI.
          </p>
        </div>

        {/* OS Cards */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-12">
          {releases.map((rel, idx) => (
            <div
              key={idx}
              className="ak-card p-6 flex flex-col justify-between"
            >
              <div>
                <div className="flex items-center justify-between mb-4">
                  <div className="w-12 h-12 rounded-lg bg-[var(--surface)] border-2 border-[var(--line)] flex items-center justify-center shadow-[2px_2px_0_var(--shade)]">
                    {rel.icon}
                  </div>
                  <span className="ak-badge bg-[var(--surface-2)] text-[11px]">
                    {rel.badge}
                  </span>
                </div>
                <h3 className="text-xl font-extrabold text-[var(--ink)] mb-1">
                  {rel.os}
                </h3>
                <p className="text-xs text-[var(--ink-3)] font-mono mb-6">
                  {rel.desc}
                </p>
              </div>

              <div className="flex flex-col gap-3">
                <a
                  href={rel.primaryUrl}
                  className="ak-btn ak-btn-accent w-full text-center py-2.5 text-sm"
                >
                  <Download size={16} />
                  <span>{rel.primaryBtn}</span>
                </a>
                {rel.secondaryUrl ? (
                  <a
                    href={rel.secondaryUrl}
                    className="text-xs text-center font-bold text-[var(--ink-2)] hover:text-[var(--ink)] underline"
                  >
                    {rel.secondaryText}
                  </a>
                ) : (
                  <span className="text-xs text-center font-medium text-[var(--ink-3)]">
                    {rel.secondaryText}
                  </span>
                )}
              </div>
            </div>
          ))}
        </div>

        {/* CLI Section */}
        <div className="ak-card p-6 sm:p-8 bg-[var(--surface)] flex flex-col md:flex-row items-center justify-between gap-6">
          <div className="flex items-center gap-4">
            <div className="w-12 h-12 rounded-lg bg-[var(--mark)] border-2 border-[var(--line)] flex items-center justify-center font-black text-xl shadow-[2px_2px_0_var(--shade)]">
              <Terminal size={24} />
            </div>
            <div>
              <h4 className="font-extrabold text-base text-[var(--ink)]">
                Prefer the Command Line?
              </h4>
              <p className="text-xs sm:text-sm text-[var(--ink-2)] font-medium mt-0.5">
                Run the indexer directly in CI/CD or your favorite terminal.
              </p>
            </div>
          </div>

          <div className="bg-[var(--code-bg)] border-2 border-[var(--line)] rounded-lg px-4 py-2.5 font-mono text-xs sm:text-sm text-[var(--ink)] font-bold shadow-[2px_2px_0_var(--shade)]">
            cargo install ark-cli
          </div>
        </div>
      </div>
    </section>
  );
}
