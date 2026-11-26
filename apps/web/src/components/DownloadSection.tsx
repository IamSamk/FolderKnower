import { Apple, Download, Laptop, Monitor, Terminal } from "lucide-react";
import { fmtDate, fmtSize, getLatestRelease, pick, REPO } from "@/lib/releases";

export async function DownloadSection() {
  const latest = await getLatestRelease();
  const fallback = `https://github.com/${REPO}/releases/latest`;
  const exe = pick(latest, /_x64-setup\.exe$/);
  const msi = pick(latest, /_x64_en-US\.msi$/);
  const dmg = pick(latest, /_universal\.dmg$/);
  const appimage = pick(latest, /\.AppImage$/);
  const deb = pick(latest, /\.deb$/);
  const rpm = pick(latest, /\.rpm$/);

  const releases = [
    {
      os: "Windows",
      icon: <Monitor size={28} />,
      desc: "Windows 10 / 11 (x64)",
      primaryBtn: exe ? `Download .exe · ${fmtSize(exe.size)}` : "Download for Windows",
      primaryUrl: exe?.url ?? fallback,
      secondaryText: "Download .msi package",
      secondaryUrl: msi?.url ?? fallback,
      badge: "Auto-updates",
    },
    {
      os: "macOS",
      icon: <Apple size={28} />,
      desc: "Universal (Apple Silicon & Intel)",
      primaryBtn: dmg ? `Download .dmg · ${fmtSize(dmg.size)}` : "Download for macOS",
      primaryUrl: dmg?.url ?? fallback,
      secondaryText: "Not notarized: right click, Open on first run",
      badge: "Universal Binary",
    },
    {
      os: "Linux",
      icon: <Laptop size={28} />,
      desc: "x86-64 AppImage, deb, rpm",
      primaryBtn: appimage
        ? `Download AppImage · ${fmtSize(appimage.size)}`
        : "View Linux packages",
      primaryUrl: appimage?.url ?? fallback,
      secondaryText: [deb && ".deb", rpm && ".rpm"].filter(Boolean).join(" · ") || "deb and rpm",
      secondaryUrl: deb?.url ?? fallback,
      badge: "Auto-updates (AppImage)",
    },
  ];

  return (
    <section
      id="download"
      className="py-24 border-t-[2.5px] border-[var(--line)] bg-[var(--surface-2)]"
    >
      <div className="max-w-6xl mx-auto px-6">
        <div className="text-center max-w-2xl mx-auto mb-16">
          <div className="ak-badge bg-[var(--tier-ui)] text-[#1f1d36] mb-3 font-bold">
            Releases & Installers
          </div>
          <h2 className="text-3xl sm:text-4xl font-extrabold text-[var(--ink)]">
            Get FolderKnower Desktop
          </h2>
          <p className="text-[var(--ink-2)] text-base font-medium mt-3">
            Native desktop app that updates itself with a pop-up, plus a standalone Rust CLI.
          </p>
          {latest && (
            <p className="mt-4 text-sm font-mono text-[var(--ink-3)]">
              Latest: <span className="font-bold text-[var(--ink)]">{latest.tag}</span> ·{" "}
              {fmtDate(latest.publishedAt)} ·{" "}
              <a href="/releases" className="underline hover:text-[var(--ink)]">
                what&apos;s new
              </a>{" "}
              ·{" "}
              <a href="/code-signing" className="underline hover:text-[var(--ink)]">
                code signing policy
              </a>
            </p>
          )}
          <p className="mt-3 text-sm text-[var(--ink-3)]">
            Windows builds are signed through the{" "}
            <a
              href="https://signpath.org"
              target="_blank"
              rel="noopener noreferrer"
              className="underline hover:text-[var(--ink)]"
            >
              SignPath Foundation
            </a>
            . Every download also has a checksum and a build attestation.
          </p>
        </div>

        {/* OS Cards */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-12">
          {releases.map((rel) => (
            <div key={rel.os} className="ak-card p-6 flex flex-col justify-between">
              <div>
                <div className="flex items-center justify-between mb-4">
                  <div className="w-12 h-12 rounded-lg bg-[var(--surface)] border-2 border-[var(--line)] flex items-center justify-center shadow-[2px_2px_0_var(--shade)]">
                    {rel.icon}
                  </div>
                  <span className="ak-badge bg-[var(--surface-2)] text-[11px]">{rel.badge}</span>
                </div>
                <h3 className="text-xl font-extrabold text-[var(--ink)] mb-1">{rel.os}</h3>
                <p className="text-xs text-[var(--ink-3)] font-mono mb-6">{rel.desc}</p>
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
                Run the indexer as <code>fknow</code> in CI or any terminal. On Windows:{" "}
                <code>irm folderknower.vercel.app/i.ps1 | iex</code>
              </p>
            </div>
          </div>

          <div className="bg-[var(--code-bg)] border-2 border-[var(--line)] rounded-lg px-4 py-2.5 font-mono text-xs sm:text-sm text-[var(--ink)] font-bold shadow-[2px_2px_0_var(--shade)]">
            curl -fsSL folderknower.vercel.app/i | sh
          </div>
        </div>
      </div>
    </section>
  );
}
