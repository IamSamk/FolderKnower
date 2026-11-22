// Soft neo-brutalist update notification banner.
// Mounts invisibly; polls once on mount, then shows a dismissible top banner
// if a newer GitHub release is available.

import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import { checkForUpdate, installUpdate, type UpdateInfo } from "../lib/backend";

export function UpdateBanner() {
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [dismissed, setDismissed] = useState(false);
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    // Check once after a short delay so it doesn't block startup rendering.
    const t = setTimeout(async () => {
      try {
        const info = await checkForUpdate();
        if (info) setUpdate(info);
      } catch {
        // silently ignore — updater failures must never crash the app
      }
    }, 4_000);
    return () => clearTimeout(t);
  }, []);

  const visible = !!update && !dismissed;

  if (!visible || !update) return null;

  return (
    <AnimatePresence>
      <motion.div
        key="update-banner"
        className="update-banner"
        initial={{ y: -72, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        exit={{ y: -72, opacity: 0 }}
        transition={{ type: "spring", stiffness: 340, damping: 30 }}
      >
        <span className="update-banner__label">Architekton {update.version} is out</span>

        {update.body && (
          <span className="update-banner__note ak-mono">{update.body.split("\n")[0]}</span>
        )}

        <div className="update-banner__actions">
          <button
            type="button"
            className="ak-btn ak-btn--accent update-banner__btn-install"
            disabled={installing}
            onClick={async () => {
              setInstalling(true);
              try {
                await installUpdate();
              } catch {
                setInstalling(false);
              }
            }}
          >
            {installing ? "Downloading…" : "Update now"}
          </button>

          <button
            type="button"
            className="ak-btn update-banner__btn-later"
            onClick={() => setDismissed(true)}
          >
            Later
          </button>
        </div>
      </motion.div>
    </AnimatePresence>
  );
}
