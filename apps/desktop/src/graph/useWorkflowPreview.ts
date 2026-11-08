import { useEffect, useMemo, useState } from "react";
import type { RunGraph } from "../data/types";
import { useDefinitions } from "../editor/DefinitionsContext";
import { engine } from "../engine/client";
import { useEngine } from "../engine/EngineContext";
import { previewGraph } from "../engine/runGraph";

type Loaded = { name: string; source: string | null; error: string | null };

export type WorkflowPreview = { name: string | null; graph: RunGraph | null; error: string | null; open: (name: string | null) => void };

export function useWorkflowPreview(): WorkflowPreview {
  const { project } = useEngine();
  const { catalog } = useDefinitions();
  const [name, setName] = useState<string | null>(null);
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const file = name ? (catalog?.workflows.find((entry) => entry.name === name)?.file ?? `workflows/${name}.toml`) : null;

  useEffect(() => {
    if (!name || !file) return;
    let cancelled = false;
    engine
      .readDefinition(file)
      .then((source) => !cancelled && setLoaded({ name, source, error: null }))
      .catch((failure) => !cancelled && setLoaded({ name, source: null, error: String(failure) }));
    return () => {
      cancelled = true;
    };
  }, [name, file, catalog]);

  useEffect(() => {
    setName(null);
  }, [project?.root]);

  const current = loaded?.name === name ? loaded : null;
  const built = useMemo(() => {
    if (!project || !name || !current?.source) return { graph: null, error: current?.error ?? null };
    try {
      return { graph: previewGraph(project, name, current.source), error: null };
    } catch (failure) {
      return { graph: null, error: `${file} is not valid TOML: ${String(failure)}` };
    }
  }, [project, name, current, file]);

  return { name, graph: built.graph, error: built.error, open: setName };
}
