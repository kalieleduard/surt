import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import { ReactFlowProvider } from "@xyflow/react";
import type { GraphNode, RunGraph } from "../data/types";
import { EngineProvider, useEngine } from "../engine/EngineContext";

export type CanvasTool = "select" | "pan";

type RunView = {
  graph: RunGraph | null;
  selectedId: string;
  selected: GraphNode | undefined;
  select: (id: string) => void;
  drill: (itemKey: string) => void;
  back: () => void;
  sourceOpen: boolean;
  toggleSource: () => void;
  tool: CanvasTool;
  setTool: (tool: CanvasTool) => void;
};

const RunViewContext = createContext<RunView | null>(null);

export function useRunView() {
  const view = useContext(RunViewContext);
  if (!view) throw new Error("useRunView needs a RunViewProvider");
  return view;
}

export function RunViewProvider({ children }: { children: ReactNode }) {
  return (
    <EngineProvider>
      <ReactFlowProvider>
        <RunViewState>{children}</RunViewState>
      </ReactFlowProvider>
    </EngineProvider>
  );
}

function RunViewState({ children }: { children: ReactNode }) {
  const { graph, drill, back } = useEngine();
  const [chosen, setChosen] = useState<{ graph: string; node: string } | null>(null);
  const [sourceOpen, setSourceOpen] = useState(false);
  const [tool, setTool] = useState<CanvasTool>("select");

  const selectedId = useMemo(() => {
    if (!graph) return "";
    const kept = chosen?.graph === graph.id && graph.nodes.some((node) => node.id === chosen.node);
    return kept && chosen ? chosen.node : graph.defaultSelection;
  }, [graph, chosen]);

  const view = useMemo<RunView>(
    () => ({
      graph,
      selectedId,
      selected: graph?.nodes.find((node) => node.id === selectedId),
      select: (node) => graph && setChosen({ graph: graph.id, node }),
      drill,
      back,
      sourceOpen,
      toggleSource: () => setSourceOpen((open) => !open),
      tool,
      setTool,
    }),
    [graph, selectedId, sourceOpen, tool, drill, back],
  );

  return <RunViewContext.Provider value={view}>{children}</RunViewContext.Provider>;
}
