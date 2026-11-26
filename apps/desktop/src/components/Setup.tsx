import { Check, Download, FileSearch, FolderOpen, Trash2, X } from "lucide-react";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import {
  type AiFound,
  type AiProgress,
  type AiStatus,
  aiFind,
  aiInstall,
  aiLink,
  aiRemove,
  aiStatus,
  checkIsTauri,
  isTauri,
  pickModelFile,
} from "../lib/backend";
import { useStore } from "../store";

const TERMS_VERSION = "2";
const KEY = "architekton.terms";

function accepted(): boolean {
  try {
    return localStorage.getItem(KEY) === TERMS_VERSION;
  } catch {
    return false;
  }
}

const mb = (n: number) => `${Math.round(n / 1_000_000)} MB`;

const STEPS = ["Terms", "Privacy", "Local AI"] as const;

function Terms() {
  return (
    <div className="ak-legal">
      <p>
        FolderKnower is free, open-source software under your choice of the <b>MIT</b> or{" "}
        <b>Apache 2.0</b> license. You can use it for personal and commercial work, change it and
        share it.
      </p>
      <p>
        It is provided as is, without warranty of any kind. It is a tool that explains code: its
        diagrams, complexity estimates and AI explanations can be incomplete or wrong, so check
        important conclusions against the source.
      </p>
      <p className="ak-dim">
        Full texts: LICENSE-MIT and LICENSE-APACHE in the repository. Fonts are under the SIL Open
        Font License; other open-source parts keep their own licenses.
      </p>
    </div>
  );
}

function Privacy() {
  return (
    <div className="ak-legal">
      <p>
        <b>Your code never leaves this computer.</b> FolderKnower has no account, no analytics and
        no telemetry. It does not upload, share or keep copies of your code anywhere else.
      </p>
      <p>
        To make the next run faster it caches parsed results in your user cache folder. Delete that
        folder any time; you only lose speed.
      </p>
      <p>
        If you add the optional AI models, they are downloaded once from their official sources,
        checked against fixed SHA-256 checksums, and run here on 127.0.0.1. Nothing you analyse is
        sent to those sites. If you already have them, you can point to your own files instead: they
        are checked, only read, and never moved or deleted.
      </p>
    </div>
  );
}

function LocalAi() {
  const [st, setSt] = useState<AiStatus | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [prog, setProg] = useState<AiProgress | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [found, setFound] = useState<AiFound[] | null>(null);
  useEffect(() => {
    void aiStatus().then(setSt);
  }, []);
  if (!isTauri && !checkIsTauri())
    return (
      <div className="ak-legal">
        <p>
          The local AI models install in the desktop app. In this browser preview they are not
          available.
        </p>
      </div>
    );
  if (!st) return <div className="ak-legal">Checking what is installed…</div>;
  const runtime = st.components.find((c) => c.id === "runtime");
  const install = async (id: string) => {
    setErr(null);
    setBusy(id);
    try {
      let s = st;
      if (id !== "runtime" && !runtime?.installed) {
        s = await aiInstall("runtime", setProg);
        setSt(s);
      }
      s = await aiInstall(id, setProg);
      setSt(s);
      void useStore.getState().refreshSlm();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(null);
      setProg(null);
    }
  };
  const link = async (id: string, path: string | null) => {
    if (!path) return;
    setErr(null);
    setBusy(id);
    try {
      const s = await aiLink(id, path, setProg);
      setSt(s);
      setFound((f) => {
        const rest = f?.filter((x) => x.id !== id) ?? [];
        return rest.some((x) => s.components.some((c) => c.id === x.id && !c.installed))
          ? rest
          : null;
      });
      void useStore.getState().refreshSlm();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(null);
      setProg(null);
    }
  };
  const look = async () => {
    setErr(null);
    setBusy("find");
    try {
      setFound(await aiFind());
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(null);
    }
  };
  const nameOf = (id: string) => st.components.find((c) => c.id === id)?.name ?? id;
  const missing = st.components.filter((c) => !c.installed).map((c) => c.id);
  const usable = (found ?? []).filter((f) => missing.includes(f.id));
  return (
    <div className="ak-legal">
      {missing.length > 0 && (
        <div className="ak-ai-have">
          <span>
            Already have llama.cpp or these models from LM Studio, Ollama or Hugging Face?
          </span>
          <button type="button" className="ak-btn ghost" disabled={!!busy} onClick={look}>
            <FileSearch size={14} /> {busy === "find" ? "Looking…" : "Look on this computer"}
          </button>
        </div>
      )}
      {found && (
        <ul className="ak-ai-found">
          {usable.length === 0 && (
            <li className="ak-dim">
              Nothing usable found in the usual folders. Use "Use a file I have" below to pick one.
            </li>
          )}
          {usable.map((f) => (
            <li key={`${f.id}:${f.path}`}>
              <div className="min-w-0">
                <b>{f.label}</b>
                <span className="ak-dim">
                  {" "}
                  for {nameOf(f.id)}
                  {f.size ? `, ${mb(f.size)}` : ""}
                  {f.likelyExact ? ", looks like the exact file" : ""}
                </span>
                <div className="ak-dim ak-mono truncate" title={f.path}>
                  {f.path}
                </div>
              </div>
              <button
                type="button"
                className="ak-btn"
                disabled={!!busy}
                onClick={() => void link(f.id, f.path)}
              >
                {busy === f.id ? "Checking…" : "Use"}
              </button>
            </li>
          ))}
        </ul>
      )}
      <p>
        Everything in FolderKnower works without these. Add them for a second opinion when the rules
        are unsure, and for plain-language explanations that cite the graph. They run on your CPU,
        here.
      </p>
      <ul className="ak-comps">
        {st.components.map((c) => (
          <li key={c.id} className={`ak-comp ${c.installed ? "on" : ""}`}>
            <div className="ak-comp-head">
              <b>{c.name}</b>
              <span className="ak-tag">{c.license}</span>
              <span className="ak-dim ml-auto">{mb(c.size)}</span>
            </div>
            <p className="ak-dim">{c.purpose}</p>
            {c.linked && (
              <p className="ak-comp-linked">
                <span className={`ak-tag ${c.linked.check === "exact" ? "ok" : ""}`}>
                  {c.linked.check === "exact"
                    ? "your file, checksum matches"
                    : c.id === "runtime"
                      ? "your llama-server"
                      : `your model${c.linked.modelName ? `: ${c.linked.modelName}` : ""}, not the tested one`}
                </span>
                <span className="ak-mono truncate" title={c.linked.path}>
                  {c.linked.path}
                </span>
              </p>
            )}
            <div className="ak-comp-foot">
              <a href={c.licenseUrl} target="_blank" rel="noreferrer">
                License and model card
              </a>
              {c.installed ? (
                <button
                  type="button"
                  className="ak-btn ghost"
                  disabled={!!busy}
                  onClick={() => void aiRemove(c.id).then(setSt)}
                  title={c.linked ? "Forget this file. It is not deleted." : undefined}
                >
                  <Trash2 size={14} /> {c.linked ? "Stop using" : "Remove"}
                </button>
              ) : (
                <span className="flex gap-1.5">
                  <button
                    type="button"
                    className="ak-btn ghost"
                    disabled={!!busy}
                    onClick={async () => link(c.id, await pickModelFile(c.id))}
                  >
                    <FolderOpen size={14} /> Use a file I have
                  </button>
                  <button
                    type="button"
                    className="ak-btn"
                    disabled={!!busy || !c.supported}
                    onClick={() => void install(c.id)}
                  >
                    <Download size={14} /> {busy === c.id ? "Working…" : "Install"}
                  </button>
                </span>
              )}
            </div>
            {busy && prog && (prog.id === c.id || (busy === c.id && prog.id === "runtime")) && (
              <div className="ak-progress" aria-live="polite">
                <motion.div
                  className="ak-progress-bar"
                  animate={{ width: `${Math.round((prog.done / Math.max(prog.total, 1)) * 100)}%` }}
                />
                <span>
                  {prog.id === "runtime" && busy !== "runtime" ? "Runtime: " : ""}
                  {prog.phase === "download"
                    ? `${mb(prog.done)} of ${mb(prog.total)}`
                    : prog.phase === "extract"
                      ? "Unpacking"
                      : prog.phase === "check"
                        ? `Checking your file: ${mb(prog.done)} of ${mb(prog.total)}`
                        : "Checksum verified"}
                </span>
              </div>
            )}
          </li>
        ))}
      </ul>
      {err && <p className="ak-err">{err}</p>}
      <p className="ak-dim ak-ai-dir">
        Downloads are saved in <span className="ak-mono">{st.installDir}</span>. Files you point to
        stay where they are and are only read.
      </p>
    </div>
  );
}

export function Setup() {
  const open = useStore((s) => s.setupOpen);
  const [step, setStep] = useState(0);
  const firstRun = !accepted();
  useEffect(() => {
    if (firstRun) useStore.setState({ setupOpen: true });
  }, [firstRun]);
  useEffect(() => {
    if (open) setStep(accepted() ? 2 : 0);
  }, [open]);
  const close = () => {
    if (!accepted()) return;
    useStore.setState({ setupOpen: false });
  };
  const accept = () => {
    try {
      localStorage.setItem(KEY, TERMS_VERSION);
    } catch {
      // still allow use this session
    }
    setStep(2);
  };
  return (
    <AnimatePresence>
      {open && (
        <motion.div
          className="ak-modal-wrap"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
        >
          <motion.div
            className="ak-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="setup-title"
            initial={{ y: 30, rotate: -1.5, opacity: 0 }}
            animate={{ y: 0, rotate: 0, opacity: 1 }}
            exit={{ y: 20, opacity: 0 }}
            transition={{ type: "spring", stiffness: 320, damping: 26 }}
          >
            <div className="ak-modal-head">
              <h2 id="setup-title">{firstRun ? "Before you start" : "Local AI"}</h2>
              {accepted() && (
                <button type="button" className="ak-iconbtn" aria-label="Close" onClick={close}>
                  <X size={16} strokeWidth={2.6} />
                </button>
              )}
            </div>
            <ol className="ak-steps">
              {STEPS.map((s, i) => (
                <li key={s} className={i === step ? "on" : i < step ? "done" : ""}>
                  <span>{i < step ? <Check size={12} strokeWidth={3} /> : i + 1}</span>
                  {s}
                </li>
              ))}
            </ol>
            <AnimatePresence mode="wait">
              <motion.div
                key={step}
                initial={{ opacity: 0, x: 24 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -24 }}
                transition={{ duration: 0.22 }}
              >
                {step === 0 && <Terms />}
                {step === 1 && <Privacy />}
                {step === 2 && <LocalAi />}
              </motion.div>
            </AnimatePresence>
            <div className="ak-modal-foot">
              {step === 0 && (
                <button type="button" className="ak-btn primary" onClick={() => setStep(1)}>
                  I agree
                </button>
              )}
              {step === 1 && (
                <>
                  <button type="button" className="ak-btn ghost" onClick={() => setStep(0)}>
                    Back
                  </button>
                  <button type="button" className="ak-btn primary" onClick={accept}>
                    Got it
                  </button>
                </>
              )}
              {step === 2 && (
                <button type="button" className="ak-btn primary" onClick={close}>
                  Done
                </button>
              )}
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
