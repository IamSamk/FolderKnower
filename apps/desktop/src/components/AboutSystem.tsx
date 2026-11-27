// "Your computer": facts about this machine and what it can run. Separate from Live,
// which only follows the project's own processes while it runs.

import { Cpu, HardDrive, MemoryStick } from "lucide-react";
import { useEffect, useState } from "react";
import { fmtBytes } from "../lib/insights";
import { type SystemSnapshot, systemSnapshot } from "../lib/telemetry";
import { KvCalculator } from "./insights/KvCache";

export function AboutSystem() {
  const [snap, setSnap] = useState<SystemSnapshot | null>(null);
  useEffect(() => {
    void systemSnapshot()
      .then(setSnap)
      .catch(() => undefined);
  }, []);
  return (
    <div className="ak-insights-body ak-about">
      <section className="ak-card">
        <div className="ak-section-title">This computer</div>
        {snap ? (
          <div className="ak-about-facts">
            <div>
              <Cpu size={15} /> <b>{snap.cpuBrand || "CPU"}</b>
              <div className="ak-muted text-[12px]">
                {snap.physicalCores ? `${snap.physicalCores} cores, ` : ""}
                {snap.cpus.length} threads
              </div>
            </div>
            <div>
              <MemoryStick size={15} /> <b>{fmtBytes(snap.totalMemory)} memory</b>
              <div className="ak-muted text-[12px]">
                {fmtBytes(snap.availableMemory)} free right now
              </div>
            </div>
            <div>
              <HardDrive size={15} /> <b>{snap.os}</b>
              <div className="ak-muted text-[12px]">
                {fmtBytes(snap.totalSwap)} swap (disk used as spare memory)
              </div>
            </div>
          </div>
        ) : (
          <p className="ak-muted text-[13px]">Reading this computer's specs…</p>
        )}
        <p className="ak-muted mt-2 text-[12px]">
          Read once, locally. Nothing leaves your computer. To watch your project's own processes
          while it runs, use the Live tab.
        </p>
      </section>
      <section className="ak-card">
        <div className="ak-section-title">Will a model fit on this computer?</div>
        <KvCalculator ramBytes={snap?.totalMemory} />
      </section>
    </div>
  );
}
