import { useEffect, useMemo, useState } from "react";
import { table, text, type Specs } from "../engine/contracts";
import { engine } from "../engine/client";
import { parseToml, type TomlTable } from "../engine/toml";
import { useDefinitions } from "./DefinitionsContext";

type Loaded = { key: string; live: string | null; agents: Record<string, TomlTable> };

export type WorkflowSpecs = {
  specs: Specs;
  file: string;
  live: string | null;
  problem: string | null;
};

export function useWorkflowSpecs(workflow: string, frozen: string, child: boolean): WorkflowSpecs {
  const { catalog } = useDefinitions();
  const file = catalog?.workflows.find((entry) => entry.name === workflow)?.file ?? `workflows/${workflow}.toml`;
  const parsed = useMemo(() => safeParse(frozen), [frozen]);
  const agentIds = useMemo(() => agentsOf(parsed.table), [parsed.table]);
  const key = `${file}|${agentIds.join(",")}`;
  const [loaded, setLoaded] = useState<Loaded | null>(null);

  useEffect(() => {
    if (!catalog) return;
    let cancelled = false;
    const known = new Set(catalog.agents.map((agent) => agent.id));
    const live = catalog.workflows.some((entry) => entry.file === file) ? engine.readDefinition(file) : Promise.resolve(null);
    const agents = Promise.all(
      agentIds.filter((id) => known.has(id)).map((id) => engine.readDefinition(`agents/${id}/agent.toml`).then((source) => [id, safeParse(source).table] as const)),
    );
    Promise.all([live, agents])
      .then(([source, entries]) => {
        if (!cancelled) setLoaded({ key, live: source, agents: Object.fromEntries(entries) });
      })
      .catch(() => {
        if (!cancelled) setLoaded({ key, live: null, agents: {} });
      });
    return () => {
      cancelled = true;
    };
  }, [catalog, key]);

  const current = loaded?.key === key ? loaded : null;
  return {
    specs: { workflow: parsed.table, agents: current?.agents ?? {}, child },
    file,
    live: current?.live ?? null,
    problem: parsed.problem,
  };
}

function agentsOf(workflow: TomlTable): string[] {
  const ids = Object.values(table(workflow.steps)).map((step) => text(table(step).agent));
  return [...new Set(ids.filter((id): id is string => Boolean(id)))].sort();
}

function safeParse(source: string): { table: TomlTable; problem: string | null } {
  try {
    return { table: parseToml(source), problem: null };
  } catch (failure) {
    return { table: {}, problem: String(failure) };
  }
}
