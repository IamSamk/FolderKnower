"use client";

import { Layers, Moon, Sun } from "lucide-react";
import { useEffect, useState } from "react";
import { GithubIcon } from "./Icons";

const NAV = [
  { href: "/#explorer", label: "How it works" },
  { href: "/#features", label: "Features" },
  { href: "/releases", label: "Releases" },
];

export function Header() {
  const [theme, setTheme] = useState<"dark" | "light">("dark");

  useEffect(() => {
    const current = document.documentElement.getAttribute("data-theme");
    if (current === "light" || current === "dark") setTheme(current);
  }, []);

  const toggleTheme = () => {
    const next = theme === "dark" ? "light" : "dark";
    setTheme(next);
    try {
      localStorage.setItem("ak-theme", next);
    } catch {
      // Storage can be blocked; the theme still changes for this visit.
    }
    document.documentElement.setAttribute("data-theme", next);
  };

  return (
    <header className="sticky top-0 z-50 border-b-[2.5px] border-[var(--line)] bg-[var(--paper)]/90 backdrop-blur-md">
      <div className="max-w-6xl mx-auto px-6 h-16 flex items-center justify-between gap-4">
        <a
          href="/"
          className="flex items-center gap-2.5 font-black text-xl tracking-tight text-[var(--ink)]"
        >
          <span className="w-8 h-8 rounded-[var(--r-sm)] bg-[var(--accent)] border-2 border-[var(--line)] flex items-center justify-center">
            <Layers size={18} strokeWidth={2.4} className="text-[var(--on-accent)]" />
          </span>
          <span>FolderKnower</span>
        </a>

        <nav className="hidden md:flex items-center gap-8 font-bold text-sm text-[var(--ink-2)]">
          {NAV.map((n) => (
            <a key={n.href} href={n.href} className="hover:text-[var(--ink)] transition-colors">
              {n.label}
            </a>
          ))}
          <a
            href="https://github.com/IamSamk/FolderKnower"
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-2 hover:text-[var(--ink)] transition-colors"
          >
            <GithubIcon size={16} />
            GitHub
          </a>
        </nav>

        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={toggleTheme}
            className="ak-btn w-10 h-10 p-0"
            aria-label={theme === "dark" ? "Use light theme" : "Use dark theme"}
          >
            <span className="ak-swap">
              <Sun size={18} data-on={theme === "dark"} />
              <Moon size={18} data-on={theme === "light"} />
            </span>
          </button>
          <a href="/download" className="ak-btn ak-btn-accent hidden sm:inline-flex">
            Download
          </a>
        </div>
      </div>
    </header>
  );
}
