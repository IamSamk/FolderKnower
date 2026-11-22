"use client";

import { useEffect, useState } from "react";
import { Sun, Moon } from "lucide-react";
import { GithubIcon } from "./Icons";

export function Header() {
  const [theme, setTheme] = useState<"dark" | "light">("dark");

  useEffect(() => {
    const saved = localStorage.getItem("ak-theme") as "dark" | "light" | null;
    if (saved) {
      setTheme(saved);
      document.documentElement.setAttribute("data-theme", saved);
    }
  }, []);

  const toggleTheme = () => {
    const next = theme === "dark" ? "light" : "dark";
    setTheme(next);
    localStorage.setItem("ak-theme", next);
    document.documentElement.setAttribute("data-theme", next);
  };

  return (
    <header className="sticky top-0 z-50 border-b-[2.5px] border-[var(--line)] bg-[var(--paper)]/90 backdrop-blur-md">
      <div className="max-w-6xl mx-auto px-6 h-18 flex items-center justify-between">
        <a href="#" className="flex items-center gap-2.5 font-black text-xl tracking-tight text-[var(--ink)] no-underline">
          <span className="w-8 h-8 rounded-lg bg-[var(--mark)] border-2 border-[var(--line)] flex items-center justify-center font-black text-sm shadow-[2px_2px_0_var(--shade)]">
            ⬡
          </span>
          <span>Architekton</span>
        </a>

        <nav className="hidden md:flex items-center gap-7 font-bold text-sm text-[var(--ink-2)]">
          <a href="#explorer" className="hover:text-[var(--ink)] transition-colors">Explorer</a>
          <a href="#features" className="hover:text-[var(--ink)] transition-colors">Features</a>
          <a href="#download" className="hover:text-[var(--ink)] transition-colors">Downloads</a>
          <a
            href="https://github.com/IamSamk/Architekton"
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-1.5 hover:text-[var(--ink)] transition-colors"
          >
            <GithubIcon size={16} />
            GitHub
          </a>
        </nav>

        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={toggleTheme}
            className="ak-btn p-2.5 bg-[var(--surface)] text-[var(--ink)]"
            aria-label="Toggle theme"
          >
            {theme === "dark" ? <Sun size={17} /> : <Moon size={17} />}
          </button>
          <a
            href="#download"
            className="ak-btn ak-btn-mark hidden sm:inline-flex"
          >
            Get App ↓
          </a>
        </div>
      </div>
    </header>
  );
}
