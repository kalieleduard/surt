import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { engine } from "../engine/client";
import { useEngine } from "../engine/EngineContext";
import type { ScriptEnded, ScriptOutput, ScriptView } from "../engine/types";
import { appendOutput, type OutputLine } from "./output";

const OUTPUT_EVENT = "surt://script-output";
const ENDED_EVENT = "surt://script-ended";

export type ScriptState = { running: boolean; ended: ScriptEnded | null; lines: OutputLine[] };

type Scripts = {
  scripts: ScriptView[];
  error: string | null;
  states: Record<string, ScriptState>;
  shown: string | null;
  show: (name: string | null) => void;
  start: (name: string) => void;
  stop: (name: string) => void;
  clear: (name: string) => void;
  reload: () => void;
};

const ScriptsContext = createContext<Scripts | null>(null);
const EMPTY: ScriptState = { running: false, ended: null, lines: [] };

export function useScripts() {
  const value = useContext(ScriptsContext);
  if (!value) throw new Error("useScripts needs a ScriptsProvider");
  return value;
}

export function scriptState(states: Record<string, ScriptState>, name: string): ScriptState {
  return states[name] ?? EMPTY;
}

export function ScriptsProvider({ children }: { children: ReactNode }) {
  const { available, project } = useEngine();
  const [scripts, setScripts] = useState<ScriptView[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [states, setStates] = useState<Record<string, ScriptState>>({});
  const [shown, setShown] = useState<string | null>(null);

  const patch = useCallback((name: string, change: (state: ScriptState) => ScriptState) => {
    setStates((all) => ({ ...all, [name]: change(all[name] ?? EMPTY) }));
  }, []);

  const reload = useCallback(() => {
    if (!available || !project) return setScripts([]);
    engine
      .projectScripts()
      .then((next) => {
        setScripts(next);
        setError(null);
        setStates((all) => {
          const merged = { ...all };
          for (const script of next) merged[script.name] = { ...(all[script.name] ?? EMPTY), running: script.running };
          return merged;
        });
      })
      .catch((failure) => setError(String(failure)));
  }, [available, project]);

  useEffect(reload, [reload]);

  useEffect(() => {
    if (!available) return;
    const output = listen<ScriptOutput>(OUTPUT_EVENT, ({ payload }) =>
      patch(payload.script, (state) => ({ ...state, lines: appendOutput(state.lines, payload) })),
    );
    const ended = listen<ScriptEnded>(ENDED_EVENT, ({ payload }) => patch(payload.script, (state) => ({ ...state, running: false, ended: payload })));
    return () => {
      output.then((stop) => stop());
      ended.then((stop) => stop());
    };
  }, [available, patch]);

  const value = useMemo<Scripts>(
    () => ({
      scripts,
      error,
      states,
      shown,
      show: setShown,
      start: (name) => {
        patch(name, () => ({ running: true, ended: null, lines: [] }));
        setShown(name);
        engine.startScript(name).catch((failure) =>
          patch(name, (state) => ({ ...state, running: false, ended: { script: name, code: null, stopped: false, error: String(failure) } })),
        );
      },
      stop: (name) => {
        engine.stopScript(name).catch((failure) => setError(String(failure)));
      },
      clear: (name) => patch(name, (state) => ({ ...state, lines: [] })),
      reload,
    }),
    [scripts, error, states, shown, patch, reload],
  );

  return <ScriptsContext.Provider value={value}>{children}</ScriptsContext.Provider>;
}
