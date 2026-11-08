import { useEffect, useRef, useState } from "react";
import { Eraser, Play, RotateCcw, Square, X } from "lucide-react";
import type { ScriptEnded } from "../engine/types";
import { scriptState, useScripts } from "./ScriptsContext";

const STICK_DISTANCE = 24;

export function ScriptConsole() {
  const { scripts, states, shown, show, start, stop, clear } = useScripts();
  const body = useRef<HTMLDivElement>(null);
  const [stuck, setStuck] = useState(true);
  const script = scripts.find((entry) => entry.name === shown);
  const state = shown ? scriptState(states, shown) : null;
  const count = state?.lines.length ?? 0;

  useEffect(() => {
    if (stuck && body.current) body.current.scrollTop = body.current.scrollHeight;
  }, [count, stuck, shown]);

  if (!script || !state) return null;

  return (
    <section className="script-console" aria-label={`${script.name} output`}>
      <header className="script-console__head">
        <span className={`script-console__dot${state.running ? " is-running" : ""}`} />
        <span className="script-console__name">{script.name}</span>
        <code className="script-console__command mono" title={script.command}>
          {script.command}
        </code>
        <span className="script-console__status mono">{status(state.running, state.ended)}</span>
        {state.running ? (
          <button className="icon-btn icon-btn--small" onClick={() => stop(script.name)} title="Stop" aria-label="Stop">
            <Square size={11} strokeWidth={2} fill="currentColor" />
          </button>
        ) : (
          <button className="icon-btn icon-btn--small" onClick={() => start(script.name)} title={state.ended ? "Run again" : "Run"} aria-label="Run">
            {state.ended ? <RotateCcw size={13} strokeWidth={1.75} /> : <Play size={13} strokeWidth={1.75} />}
          </button>
        )}
        <button className="icon-btn icon-btn--small" onClick={() => clear(script.name)} title="Clear output" aria-label="Clear output">
          <Eraser size={13} strokeWidth={1.75} />
        </button>
        <button className="icon-btn icon-btn--small" onClick={() => show(null)} title="Close" aria-label="Close output">
          <X size={14} strokeWidth={1.75} />
        </button>
      </header>
      <div
        className="script-console__body mono"
        ref={body}
        onScroll={(event) => {
          const element = event.currentTarget;
          setStuck(element.scrollHeight - element.scrollTop - element.clientHeight < STICK_DISTANCE);
        }}
      >
        {state.lines.length ? (
          state.lines.map((line) => (
            <div key={line.id} className={`script-console__line is-${line.stream}`}>
              {line.text || " "}
            </div>
          ))
        ) : (
          <div className="script-console__empty">{state.running ? "Waiting for output…" : "No output yet."}</div>
        )}
        {state.ended?.error ? <div className="script-console__line is-error">{state.ended.error}</div> : null}
      </div>
    </section>
  );
}

function status(running: boolean, ended: ScriptEnded | null) {
  if (running) return "running";
  if (!ended) return "idle";
  if (ended.error) return "failed to start";
  if (ended.stopped) return "stopped";
  return ended.code === 0 ? "exited 0" : `exited ${ended.code ?? "?"}`;
}
