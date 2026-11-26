import type { Metadata } from "next";
import { Footer } from "@/components/Footer";
import { Header } from "@/components/Header";
import { fmtDate, fmtSize, getReleases, noteLines } from "@/lib/releases";

export const metadata: Metadata = {
  title: "Releases · FolderKnower",
  description: "Every FolderKnower release with notes, downloads and checksums.",
};

export const revalidate = 600;

export default async function ReleasesPage() {
  const releases = await getReleases();
  return (
    <main className="min-h-dvh flex flex-col">
      <Header />
      <section className="flex-1 py-16">
        <div className="max-w-4xl mx-auto px-6">
          <p className="ak-eyebrow mb-3">Changelog</p>
          <h1 className="text-3xl sm:text-4xl font-extrabold text-[var(--ink)]">Releases</h1>
          <p className="text-[var(--ink-2)] mt-3 font-medium">
            Installed apps pick these up on their own and show an update pop-up. Every file is
            listed in <code className="font-mono">SHA256SUMS.txt</code> and carries a build
            provenance attestation from GitHub Actions.
          </p>

          {releases.length === 0 && (
            <p className="ak-card mt-10 text-[var(--ink-2)] font-medium">
              No release is published yet. The first one is building now and shows here when it is
              ready.
            </p>
          )}

          <ol className="mt-10 flex flex-col gap-6">
            {releases.map((r, i) => {
              const notes = noteLines(r.body.split(/^## Install/m)[0] ?? "");
              const installers = r.assets.filter((a) =>
                /(setup\.exe|\.msi|\.dmg|\.AppImage|\.deb|\.rpm|SHA256SUMS\.txt)$/.test(a.name),
              );
              return (
                <li key={r.tag} className="ak-card p-6">
                  <div className="flex flex-wrap items-baseline gap-3">
                    <h2 className="text-2xl font-extrabold text-[var(--ink)]">{r.tag}</h2>
                    {i === 0 && (
                      <span className="font-mono text-xs font-bold text-[var(--accent)]">
                        latest
                      </span>
                    )}
                    <span className="text-xs font-mono text-[var(--ink-3)]">
                      {fmtDate(r.publishedAt)}
                    </span>
                    <a href={r.url} className="ak-link ml-auto text-xs">
                      on GitHub
                    </a>
                  </div>
                  <div className="mt-4 text-sm leading-relaxed text-[var(--ink-2)]">
                    {notes.map((n) =>
                      n.kind === "h" ? (
                        <h3 key={n.id} className="mt-3 mb-1 font-extrabold text-[var(--ink)]">
                          {n.text}
                        </h3>
                      ) : n.kind === "li" ? (
                        <p key={n.id} className="pl-4 -indent-3">
                          • {n.text}
                        </p>
                      ) : (
                        <p key={n.id}>{n.text}</p>
                      ),
                    )}
                  </div>
                  {installers.length > 0 && (
                    <div className="mt-5 flex flex-wrap gap-2">
                      {installers.map((a) => (
                        <a
                          key={a.name}
                          href={a.url}
                          className="ak-badge bg-[var(--surface-2)] text-[11px] font-mono hover:bg-[var(--accent)] hover:text-[var(--on-accent)] transition-colors"
                        >
                          {a.name.replace(/^FolderKnower[_-]/, "")} · {fmtSize(a.size)}
                        </a>
                      ))}
                    </div>
                  )}
                </li>
              );
            })}
          </ol>
        </div>
      </section>
      <Footer />
    </main>
  );
}
