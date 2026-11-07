import { useState } from "react";
import { ChevronDown, ChevronUp } from "lucide-react";
import { LOGS } from "../data/activity";

export function Console({ initiallyOpen = true }: { initiallyOpen?: boolean }) {
  const [open, setOpen] = useState(initiallyOpen);
  const [mode, setMode] = useState<"run" | "debug">("run");
  const Chevron = open ? ChevronDown : ChevronUp;
  const lines = mode === "run" ? LOGS.filter((line) => line.state !== "info") : LOGS;
  return (
    <section className={`console${open ? " is-open" : ""}`}>
      <header className="console__head">
        <div className="tabs tabs--mini">
          <button className={mode === "run" ? "is-active" : undefined} onClick={() => setMode("run")}>Run</button>
          <button className={mode === "debug" ? "is-active" : undefined} onClick={() => setMode("debug")}>Debug</button>
        </div>
        <button className="console__toggle" onClick={() => setOpen(!open)}>
          <Chevron size={13} />
          Logs
          <span className="console__count">{lines.length}</span>
        </button>
        <span className="console__hint mono">surt logs 20261107-3 --follow</span>
      </header>
      {open ? (
        <div className="console__lines mono">
          {lines.map((line) => (
            <div key={line.time + line.text} className={`console__line log-${line.state}`}>
              <span className="console__time">{line.time}</span>
              <span className="console__symbol">{line.symbol}</span>
              <span>{line.text}</span>
            </div>
          ))}
        </div>
      ) : null}
    </section>
  );
}
