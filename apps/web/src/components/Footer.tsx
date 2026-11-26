import { GithubIcon } from "./Icons";

export function Footer() {
  return (
    <footer className="border-t-[2.5px] border-[var(--line)] bg-[var(--paper)] py-12">
      <div className="max-w-6xl mx-auto px-6 flex flex-col sm:flex-row items-center justify-between gap-6 text-sm font-bold text-[var(--ink-2)]">
        <div className="flex items-center gap-2">
          <span>© 2026 FolderKnower by</span>
          <a
            href="https://github.com/IamSamk"
            target="_blank"
            rel="noopener noreferrer"
            className="text-[var(--ink)] underline hover:text-[var(--accent)]"
          >
            IamSamk
          </a>
          <span>· MIT OR Apache-2.0</span>
        </div>

        <div className="flex items-center gap-6">
          <a
            href="https://github.com/IamSamk/FolderKnower"
            target="_blank"
            rel="noopener noreferrer"
            className="hover:text-[var(--ink)] flex items-center gap-1.5"
          >
            <GithubIcon size={16} />
            Repository
          </a>
          <a href="/releases" className="hover:text-[var(--ink)]">
            Releases
          </a>
          <a href="/code-signing" className="hover:text-[var(--ink)]">
            Code signing policy
          </a>
          <a
            href="https://github.com/IamSamk/FolderKnower/issues"
            target="_blank"
            rel="noopener noreferrer"
            className="hover:text-[var(--ink)]"
          >
            Issues
          </a>
        </div>
      </div>
    </footer>
  );
}
