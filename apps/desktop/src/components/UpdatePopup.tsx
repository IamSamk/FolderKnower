// In-app update pop-up. Checks the signed GitHub Releases feed shortly after launch
// and every six hours, then shows a modal with the release notes and a real download
// progress bar. Signature verification happens in the Rust updater plugin; a tampered
// or unsigned package is rejected before install.

import { Download, Sparkles, X } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  checkForUpdate,
  installUpdate,
  type UpdateInfo,
  type UpdateProgress,
} from "../lib/backend";

const FIRST_CHECK_MS = 4_000;
const RECHECK_MS = 6 * 60 * 60 * 1000;
const SKIP_KEY = "ak.update.skip";
const CHECK_EVENT = "ak:check-update";

/** Ask the pop-up to check now (manual "Check for updates"); ignores Skip. */
export function requestUpdateCheck() {
  window.dispatchEvent(new Event(CHECK_EVENT));
}

function readSkip(): string | null {
  try {
    return localStorage.getItem(SKIP_KEY);
  } catch {
    return null;
  }
}

function writeSkip(version: string) {
  try {
    localStorage.setItem(SKIP_KEY, version);
  } catch {
    // storage unavailable: skipping just lasts for this session
  }
}

type Note = { id: string; kind: "h" | "li" | "p"; text: string };

/** Tiny, safe renderer for release-note markdown: headings, bullets, paragraphs. */
function parseNotes(body: string | null): Note[] {
  if (!body) return [];
  return body
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0)
    .slice(0, 60)
    .map((l, i) => {
      const id = `n${i}`;
      const clean = (s: string) => s.replace(/\*\*|__|`/g, "");
      if (/^#{1,6}\s/.test(l)) return { id, kind: "h", text: clean(l.replace(/^#+\s*/, "")) };
      if (/^[-*]\s/.test(l)) return { id, kind: "li", text: clean(l.slice(2)) };
      return { id, kind: "p", text: clean(l) };
    });
}

function fmtMB(bytes: number) {
  return `${(bytes / 1_048_576).toFixed(1)} MB`;
}

export function UpdatePopup() {
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [open, setOpen] = useState(false);
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);

  const check = useCallback(async (manual: boolean) => {
    try {
      const info = await checkForUpdate();
      if (info && (manual || info.version !== readSkip())) {
        setUpdate(info);
        setOpen(true);
      } else if (manual) {
        setToast("You have the latest version.");
      }
    } catch (e) {
      // offline or feed unreachable: the app keeps working, we try again later
      if (manual) setToast(`Could not check for updates: ${String(e)}`);
    }
  }, []);

  useEffect(() => {
    const first = setTimeout(() => check(false), FIRST_CHECK_MS);
    const every = setInterval(() => check(false), RECHECK_MS);
    const manual = () => check(true);
    window.addEventListener(CHECK_EVENT, manual);
    return () => {
      clearTimeout(first);
      clearInterval(every);
      window.removeEventListener(CHECK_EVENT, manual);
    };
  }, [check]);

  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(null), 3_500);
    return () => clearTimeout(t);
  }, [toast]);

  const notes = useMemo(() => parseNotes(update?.body ?? null), [update]);

  const pct =
    progress?.total && progress.total > 0
      ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100))
      : null;

  async function onInstall() {
    setInstalling(true);
    setError(null);
    try {
      await installUpdate(setProgress);
    } catch (e) {
      setInstalling(false);
      setProgress(null);
      setError(String(e));
    }
  }

  return (
    <AnimatePresence>
      {toast && (
        <motion.div
          key="update-toast"
          role="status"
          className="update-toast"
          initial={{ y: -20, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
          exit={{ y: -20, opacity: 0 }}
        >
          {toast}
        </motion.div>
      )}
      {open && update && (
        <motion.div
          key="update-scrim"
          className="update-scrim"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
        >
          <motion.div
            role="dialog"
            aria-modal="true"
            aria-labelledby="update-title"
            className="update-card"
            initial={{ y: 28, scale: 0.96, opacity: 0 }}
            animate={{ y: 0, scale: 1, opacity: 1 }}
            exit={{ y: 16, scale: 0.98, opacity: 0 }}
            transition={{ type: "spring", stiffness: 360, damping: 28 }}
          >
            <div className="update-card__head">
              <div className="update-card__icon">
                <Sparkles size={20} />
              </div>
              <div className="flex-1">
                <div id="update-title" className="update-card__title">
                  FolderKnower {update.version} is ready
                </div>
                <div className="update-card__sub">
                  you have {update.currentVersion}
                  {update.date ? ` · released ${update.date.slice(0, 10)}` : ""}
                </div>
              </div>
              {!installing && (
                <button
                  type="button"
                  aria-label="Close"
                  className="update-card__icon"
                  onClick={() => setOpen(false)}
                >
                  <X size={18} />
                </button>
              )}
            </div>

            <div className="update-card__notes">
              {notes.length === 0 && <p>Bug fixes and improvements.</p>}
              {notes.map((n) =>
                n.kind === "h" ? (
                  <h4 key={n.id}>{n.text}</h4>
                ) : n.kind === "li" ? (
                  <li key={n.id}>{n.text}</li>
                ) : (
                  <p key={n.id}>{n.text}</p>
                ),
              )}
            </div>

            {installing && (
              <>
                <div className="update-card__bar">
                  <div style={{ width: `${pct ?? 100}%`, opacity: pct === null ? 0.45 : 1 }} />
                </div>
                <div className="update-card__status">
                  {progress?.done
                    ? "Verified. Installing and restarting…"
                    : progress
                      ? `${fmtMB(progress.downloaded)}${progress.total ? ` of ${fmtMB(progress.total)}` : ""}${pct !== null ? ` · ${pct}%` : ""}`
                      : "Connecting…"}
                </div>
              </>
            )}
            {error && <div className="update-card__error">Update failed: {error}</div>}

            <div className="update-card__actions">
              {!installing && (
                <button
                  type="button"
                  className="update-card__skip"
                  onClick={() => {
                    writeSkip(update.version);
                    setOpen(false);
                  }}
                >
                  Skip this version
                </button>
              )}
              {!installing && (
                <button type="button" className="ak-btn" onClick={() => setOpen(false)}>
                  Later
                </button>
              )}
              <button
                type="button"
                className="ak-btn-primary"
                disabled={installing}
                onClick={onInstall}
              >
                <Download size={16} />
                {installing ? "Updating…" : "Update and restart"}
              </button>
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
