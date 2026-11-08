import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { useEngine } from "../engine/EngineContext";
import { engine } from "../engine/client";
import type { Catalog } from "../engine/types";

export type EditTarget = { kind: "agent"; id: string; file?: string } | { kind: "workflow"; name: string };

type Definitions = {
  catalog: Catalog | null;
  catalogError: string | null;
  setCatalog: (catalog: Catalog) => void;
  reload: () => void;
  target: EditTarget | null;
  edit: (target: EditTarget) => void;
  close: () => void;
};

const DefinitionsContext = createContext<Definitions | null>(null);

export function useDefinitions() {
  const value = useContext(DefinitionsContext);
  if (!value) throw new Error("useDefinitions needs a DefinitionsProvider");
  return value;
}

export function DefinitionsProvider({ children }: { children: ReactNode }) {
  const { project, available } = useEngine();
  const [catalog, setCatalog] = useState<Catalog | null>(null);
  const [catalogError, setCatalogError] = useState<string | null>(null);
  const [target, setTarget] = useState<EditTarget | null>(null);

  const reload = useCallback(() => {
    if (!available || !project) return;
    engine
      .definitions()
      .then((next) => {
        setCatalog(next);
        setCatalogError(null);
      })
      .catch((failure) => setCatalogError(String(failure)));
  }, [available, project]);

  useEffect(() => {
    setCatalog(null);
    setTarget(null);
    reload();
  }, [reload]);

  const value = useMemo<Definitions>(
    () => ({
      catalog,
      catalogError,
      setCatalog,
      reload,
      target,
      edit: setTarget,
      close: () => setTarget(null),
    }),
    [catalog, catalogError, reload, target],
  );

  return <DefinitionsContext.Provider value={value}>{children}</DefinitionsContext.Provider>;
}

export function targetKey(target: EditTarget) {
  return target.kind === "agent" ? `agent:${target.id}` : `workflow:${target.name}`;
}

const AGENT_REFERENCE = /^\s*agent\s*=\s*"([^"]+)"/gm;
const WORKFLOW_REFERENCE = /^\s*workflow\s*=\s*"([^"]+)"/gm;

export function agentsIn(workflowSource: string): string[] {
  return unique([...workflowSource.matchAll(AGENT_REFERENCE)].map((match) => match[1]));
}

export function workflowsIn(workflowSource: string): string[] {
  return unique([...workflowSource.matchAll(WORKFLOW_REFERENCE)].map((match) => match[1]));
}

function unique(values: string[]) {
  return [...new Set(values)];
}
