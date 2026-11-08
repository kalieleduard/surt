import { useCallback, useRef, useState } from "react";
import { ChevronDown, Play, Square, SquareTerminal } from "lucide-react";
import { useEngine } from "../engine/EngineContext";
import { useDismiss } from "../ui/useDismiss";
import { scriptState, useScripts } from "./ScriptsContext";
import "./scripts.css";

const CHOSEN_KEY = "surt-script";

export function RunScript() {
  const { available, project } = useEngine();
  const { scripts, states, start, stop, show, shown } = useScripts();
  const [chosen, setChosen] = useState<string | null>(() => recall());
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLSpanElement>(null);
  const close = useCallback(() => setOpen(false), []);
  useDismiss(ref, open, close);
  if (!available || !project) return null;
  if (!scripts.length) {
    return (
      <span className="run-script" title='Add a [[scripts]] entry to .surt/config.toml to run the project from here'>
        <button className="btn btn-ghost run-script__main" disabled>
          <Play size={13} strokeWidth={1.75} /> Run
        </button>
      </span>
    );
  }
  const current = scripts.find((script) => script.name === chosen) ?? scripts[0];
  const running = scriptState(states, current.name).running;
  const choose = (name: string) => {
    setChosen(name);
    remember(name);
    close();
  };

  return (
    <span className="run-script" ref={ref}>
      <button
        className={`btn run-script__main${running ? " is-running" : " btn-ghost"}`}
        onClick={() => (running ? stop(current.name) : start(current.name))}
        title={running ? `Stop ${current.name}` : `Run ${current.command}`}
      >
        {running ? <Square size={11} strokeWidth={2} fill="currentColor" /> : <Play size={13} strokeWidth={1.75} />}
        <span>{running ? "Stop" : "Run"}</span>
        <span className="run-script__name mono">{current.name}</span>
      </button>
      <button className="btn btn-ghost run-script__more" aria-haspopup="menu" aria-expanded={open} onClick={() => setOpen(!open)} title="Choose a script">
        <ChevronDown size={13} strokeWidth={1.75} />
      </button>
      <button
        className="icon-btn run-script__console"
        aria-pressed={shown !== null}
        onClick={() => show(shown ? null : current.name)}
        title="Script output"
      >
        <SquareTerminal size={16} strokeWidth={1.5} />
      </button>
      {open ? (
        <div className="run-script__menu" role="menu">
          <span className="caption run-script__title">Scripts · .surt/config.toml</span>
          {scripts.map((script) => {
            const live = scriptState(states, script.name).running;
            return (
              <button
                key={script.name}
                role="menuitemradio"
                aria-checked={script.name === current.name}
                className={`run-script__item${script.name === current.name ? " is-active" : ""}`}
                onClick={() => choose(script.name)}
              >
                <span className="run-script__item-head">
                  <span className="run-script__item-name">{script.name}</span>
                  {live ? <span className="run-script__live">running</span> : null}
                </span>
                {script.description ? <span className="run-script__item-text">{script.description}</span> : null}
                <code className="run-script__item-command mono">{script.command}</code>
              </button>
            );
          })}
        </div>
      ) : null}
    </span>
  );
}

function recall() {
  try {
    return localStorage.getItem(CHOSEN_KEY);
  } catch {
    return null;
  }
}

function remember(name: string) {
  try {
    localStorage.setItem(CHOSEN_KEY, name);
  } catch {
    return;
  }
}
