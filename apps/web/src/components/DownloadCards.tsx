"use client";

import { Apple, Download, Laptop, Monitor } from "lucide-react";
import type { Os } from "@/lib/os";
import { useOs } from "@/lib/useOs";

type File = { label: string; href?: string; size?: string };

export interface Platform {
  os: Os;
  name: string;
  req: string;
  main: File;
  extra: File[];
  note: string;
}

const ICON: Record<Os, typeof Monitor> = { windows: Monitor, macos: Apple, linux: Laptop };

export function DownloadCards({ platforms, ready }: { platforms: Platform[]; ready: boolean }) {
  const mine = useOs();

  return (
    <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
      {platforms.map((p) => {
        const Icon = ICON[p.os];
        const isMine = p.os === mine;
        return (
          <div
            key={p.os}
            className={`ak-card p-6 flex flex-col gap-6 ${isMine ? "outline outline-[3px] outline-offset-4 outline-[var(--accent)]" : ""}`}
          >
            <div>
              <div className="flex items-center justify-between mb-4">
                <Icon size={28} strokeWidth={2.25} className="text-[var(--ink)]" />
                {isMine && (
                  <span className="font-mono text-xs font-bold text-[var(--accent)]">
                    Your system
                  </span>
                )}
              </div>
              <h3 className="text-xl font-extrabold text-[var(--ink)]">{p.name}</h3>
              <p className="text-sm text-[var(--ink-2)] font-medium mt-1">{p.req}</p>
              <p className="text-xs text-[var(--ink-3)] font-medium mt-3">{p.note}</p>
            </div>

            <div className="mt-auto flex flex-col gap-3">
              {p.main.href ? (
                <a
                  href={p.main.href}
                  className={`ak-btn w-full py-3 ${isMine || !mine ? "ak-btn-accent" : ""}`}
                >
                  <Download size={16} />
                  <span>
                    Download {p.main.label}
                    <span className="font-mono font-medium opacity-80"> · {p.main.size}</span>
                  </span>
                </a>
              ) : (
                <span aria-disabled="true" className="ak-btn w-full py-3">
                  <Download size={16} />
                  <span>{ready ? `${p.main.label} not in this release` : "Building"}</span>
                </span>
              )}
              {p.extra.some((f) => f.href) && (
                <p className="text-sm text-center text-[var(--ink-2)]">
                  Or{" "}
                  {p.extra
                    .filter((f) => f.href)
                    .map((f, i, all) => (
                      <span key={f.label}>
                        <a href={f.href} className="ak-link">
                          {f.label}
                        </a>
                        <span className="text-[var(--ink-3)]"> {f.size}</span>
                        {i < all.length - 1 ? ", " : ""}
                      </span>
                    ))}
                </p>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
