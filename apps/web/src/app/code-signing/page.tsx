import type { Metadata } from "next";
import { Footer } from "@/components/Footer";
import { Header } from "@/components/Header";

export const metadata: Metadata = {
  title: "Code signing policy · FolderKnower",
  description: "How FolderKnower releases are built, signed and verified.",
};

const rows: [string, string][] = [
  ["Updater packages (all OS)", "Minisign signature checked by the app before every install"],
  [
    "Windows installers",
    "Authenticode via SignPath Foundation (once the OSS application is approved)",
  ],
  ["Linux AppImage", "Embedded GPG signature (key fingerprint below once published)"],
  [
    "Every file",
    "SHA256SUMS.txt (GPG-signed when the key is configured) + Sigstore build provenance",
  ],
  ["macOS", "Not notarized yet (needs a paid Apple Developer account)"],
];

export default function CodeSigningPage() {
  return (
    <main className="min-h-screen flex flex-col">
      <Header />
      <section className="flex-1 py-16">
        <article className="max-w-3xl mx-auto px-6 text-[var(--ink-2)] leading-relaxed">
          <div className="ak-badge bg-[var(--tier-ui)] text-[#1f1d36] mb-3 font-bold">Trust</div>
          <h1 className="text-3xl sm:text-4xl font-extrabold text-[var(--ink)]">
            Code signing policy
          </h1>
          <p className="mt-4">
            Windows release binaries are intended to be signed with free code signing provided by{" "}
            <a className="underline" href="https://signpath.io">
              SignPath.io
            </a>
            , certificate by{" "}
            <a className="underline" href="https://signpath.org">
              SignPath Foundation
            </a>
            . Until that is approved, the table below shows exactly what is and is not signed.
          </p>

          <div className="ak-card mt-8 overflow-hidden">
            <table className="w-full text-sm">
              <tbody>
                {rows.map(([k, v]) => (
                  <tr key={k} className="border-b-2 border-[var(--line)] last:border-0">
                    <th className="text-left align-top p-3 font-extrabold text-[var(--ink)] w-1/3">
                      {k}
                    </th>
                    <td className="p-3">{v}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <h2 className="mt-10 text-xl font-extrabold text-[var(--ink)]">How a release is made</h2>
          <p className="mt-2">
            Only tagged commits on the public repository are built, by GitHub Actions, from source.
            Nothing is built or signed on a personal machine. Each release is a draft until every
            build, signature and checksum step passes, then it is published.
          </p>

          <h2 className="mt-8 text-xl font-extrabold text-[var(--ink)]">Team roles</h2>
          <ul className="mt-2 list-disc pl-5">
            <li>
              Committers and reviewers:{" "}
              <a className="underline" href="https://github.com/IamSamk">
                IamSamk
              </a>
            </li>
            <li>
              Approvers:{" "}
              <a className="underline" href="https://github.com/IamSamk">
                IamSamk
              </a>
            </li>
          </ul>

          <h2 className="mt-8 text-xl font-extrabold text-[var(--ink)]">Verify a download</h2>
          <pre className="mt-2 p-4 rounded-lg border-2 border-[var(--line)] bg-[var(--code-bg)] text-xs font-mono overflow-x-auto">
            {`# checksum
sha256sum -c SHA256SUMS.txt --ignore-missing

# built by this repo's CI (Sigstore)
gh attestation verify FolderKnower_*_amd64.AppImage -R IamSamk/FolderKnower

# Windows signature
Get-AuthenticodeSignature .\\FolderKnower_*_x64-setup.exe`}
          </pre>

          <h2 className="mt-8 text-xl font-extrabold text-[var(--ink)]">Privacy</h2>
          <p className="mt-2">
            The app makes no network calls except the update check to GitHub Releases and optional
            model downloads you start yourself. See{" "}
            <a
              className="underline"
              href="https://github.com/IamSamk/FolderKnower/blob/main/PRIVACY.md"
            >
              PRIVACY.md
            </a>
            .
          </p>
        </article>
      </section>
      <Footer />
    </main>
  );
}
