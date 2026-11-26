"use client";

import { Download } from "lucide-react";
import { OS_LABEL, useOs } from "@/lib/useOs";

/** Main call to action: downloads the installer for the visitor's OS in one click. */
export function DownloadButton({ className = "" }: { className?: string }) {
  const os = useOs();
  return (
    <a href={os ? `/download/${os}` : "/#download"} className={`ak-btn ak-btn-accent ${className}`}>
      <Download size={18} />
      <span>{os ? `Download for ${OS_LABEL[os]}` : "Download"}</span>
    </a>
  );
}
