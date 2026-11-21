// Light / dark / system theme. The choice is a per-viewer convenience kept in
// localStorage; the effective theme is written to <html data-theme>.

import { create } from "zustand";

export type ThemePref = "light" | "dark" | "system";

const KEY = "architekton.theme";

function readPref(): ThemePref {
  try {
    const v = localStorage.getItem(KEY);
    if (v === "light" || v === "dark" || v === "system") return v;
  } catch {
    // storage can be unavailable; fall back to the system setting
  }
  return "system";
}

const media =
  typeof window !== "undefined" ? window.matchMedia("(prefers-color-scheme: dark)") : null;

function effective(pref: ThemePref): "light" | "dark" {
  if (pref === "system") return media?.matches ? "dark" : "light";
  return pref;
}

function apply(pref: ThemePref) {
  const eff = effective(pref);
  document.documentElement.dataset.theme = eff;
  document.documentElement.style.colorScheme = eff;
}

interface ThemeState {
  pref: ThemePref;
  resolved: "light" | "dark";
  setPref: (p: ThemePref) => void;
}

export const useTheme = create<ThemeState>((set) => ({
  pref: readPref(),
  resolved: effective(readPref()),
  setPref(pref) {
    try {
      localStorage.setItem(KEY, pref);
    } catch {
      // ignore: the theme still applies for this session
    }
    apply(pref);
    set({ pref, resolved: effective(pref) });
  },
}));

export function initTheme() {
  apply(useTheme.getState().pref);
  media?.addEventListener("change", () => {
    const { pref } = useTheme.getState();
    if (pref === "system") {
      apply(pref);
      useTheme.setState({ resolved: effective(pref) });
    }
  });
}
