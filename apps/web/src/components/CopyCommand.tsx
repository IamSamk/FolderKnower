"use client";

import { Check, Copy } from "lucide-react";
import { useState } from "react";

export function CopyCommand({ label, command }: { label: string; command: string }) {
  const [copied, setCopied] = useState(false);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard can be blocked; the command stays visible to select by hand.
    }
  };

  return (
    <div className="min-w-0">
      <p className="text-xs font-bold text-[var(--ink-3)] mb-1.5">{label}</p>
      <div className="flex items-center gap-2 bg-[var(--code-bg)] border-2 border-[var(--line)] rounded-[var(--r-sm)] pl-4 pr-1 py-1">
        <code className="font-mono text-sm font-bold text-[var(--ink)] overflow-x-auto whitespace-nowrap flex-1 py-1.5">
          {command}
        </code>
        <button
          type="button"
          onClick={copy}
          aria-label={copied ? "Copied" : `Copy ${label} command`}
          className="shrink-0 w-9 h-9 rounded-[4px] flex items-center justify-center text-[var(--ink-2)] hover:text-[var(--ink)] hover:bg-[var(--hair)] transition-colors"
        >
          <span className="ak-swap">
            <Copy size={16} strokeWidth={2.5} data-on={!copied} />
            <Check size={16} strokeWidth={2.5} data-on={copied} className="text-[var(--ok)]" />
          </span>
        </button>
      </div>
    </div>
  );
}
