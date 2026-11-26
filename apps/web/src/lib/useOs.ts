"use client";

import { useEffect, useState } from "react";
import { type Os, osFromUserAgent } from "./os";

export const OS_LABEL: Record<Os, string> = {
  windows: "Windows",
  macos: "macOS",
  linux: "Linux",
};

/** The visitor's desktop OS, detected after mount (null on the server, phones and unknowns). */
export function useOs(): Os | null {
  const [os, setOs] = useState<Os | null>(null);
  useEffect(() => {
    setOs(osFromUserAgent(navigator.userAgent));
  }, []);
  return os;
}
