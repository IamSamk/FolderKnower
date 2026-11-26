import { Terminal } from "lucide-react";
import {
  fmtDate,
  fmtSize,
  getLatestRelease,
  INSTALLERS,
  type InstallerKey,
  pick,
} from "@/lib/releases";
import { CopyCommand } from "./CopyCommand";
import { DownloadCards, type Platform } from "./DownloadCards";

function file(latest: Awaited<ReturnType<typeof getLatestRelease>>, key: InstallerKey) {
  const a = pick(latest, INSTALLERS[key]);
  return a ? { href: `/download/${key}`, size: fmtSize(a.size) } : null;
}

export async function DownloadSection() {
  const latest = await getLatestRelease();

  const platforms: Platform[] = [
    {
      os: "windows",
      name: "Windows",
      req: "Windows 10 or 11, x64",
      main: { label: ".exe installer", ...file(latest, "windows") },
      extra: [{ label: ".msi", ...file(latest, "windows-msi") }],
      note: "Updates itself.",
    },
    {
      os: "macos",
      name: "macOS",
      req: "Apple silicon and Intel",
      main: { label: ".dmg", ...file(latest, "macos") },
      extra: [],
      note: "Not notarized yet. The first time, right-click the app and choose Open.",
    },
    {
      os: "linux",
      name: "Linux",
      req: "x86-64",
      main: { label: "AppImage", ...file(latest, "linux") },
      extra: [
        { label: ".deb", ...file(latest, "linux-deb") },
        { label: ".rpm", ...file(latest, "linux-rpm") },
      ],
      note: "The AppImage updates itself.",
    },
  ];

  return (
    <section
      id="download"
      className="scroll-mt-16 py-24 border-t-[2.5px] border-[var(--line)] bg-[var(--surface-2)]"
    >
      <div className="max-w-6xl mx-auto px-6">
        <div className="max-w-2xl mb-12">
          <p className="ak-eyebrow mb-3">Download</p>
          <h2 className="text-3xl sm:text-4xl font-extrabold tracking-tight text-[var(--ink)]">
            Get FolderKnower for your desktop
          </h2>
          <p className="text-[var(--ink-2)] text-base font-medium mt-3">
            {latest ? (
              <>
                Version <span className="font-mono font-bold text-[var(--ink)]">{latest.tag}</span>,
                released {fmtDate(latest.publishedAt)}. The app shows a pop-up when a new version is
                ready.
              </>
            ) : (
              <>
                The first release is building now. The download buttons start working as soon as it
                is published.
              </>
            )}
          </p>
        </div>

        <DownloadCards platforms={platforms} ready={Boolean(latest)} />

        <p className="mt-8 text-sm text-[var(--ink-2)]">
          Each file has a SHA-256 checksum and a build attestation.{" "}
          <a href="/releases" className="ak-link">
            Release notes
          </a>{" "}
          ·{" "}
          <a href="/code-signing" className="ak-link">
            Code signing policy
          </a>
        </p>

        <div className="ak-card mt-12 p-6 sm:p-8 flex flex-col lg:flex-row lg:items-center justify-between gap-6">
          <div className="flex items-start gap-4">
            <div className="shrink-0 w-12 h-12 rounded-[var(--r-sm)] bg-[var(--surface-2)] border-2 border-[var(--line)] flex items-center justify-center">
              <Terminal size={22} strokeWidth={2.4} />
            </div>
            <div>
              <h3 className="font-extrabold text-lg text-[var(--ink)]">Use it from a terminal</h3>
              <p className="text-sm text-[var(--ink-2)] font-medium mt-1 max-w-md">
                The <code className="font-mono font-bold text-[var(--ink)]">fknow</code> command
                writes the same <code className="font-mono">architecture.json</code> in CI or any
                shell.
              </p>
            </div>
          </div>
          <div className="flex flex-col gap-3 min-w-0">
            <CopyCommand
              label="macOS and Linux"
              command="curl -fsSL folderknower.vercel.app/i | sh"
            />
            <CopyCommand
              label="Windows PowerShell"
              command="irm folderknower.vercel.app/i.ps1 | iex"
            />
          </div>
        </div>
      </div>
    </section>
  );
}
