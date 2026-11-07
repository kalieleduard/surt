import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import { ReactFlowProvider } from "@xyflow/react";
import { FEATURE_GRAPH } from "../data/featureGraph";
import { taskCycleGraph } from "../data/taskCycleGraph";
import type { GraphNode, RunGraph } from "../data/types";

type RunView = {
  graph: RunGraph;
  selectedId: string;
  selected: GraphNode | undefined;
  select: (id: string) => void;
  drill: (itemKey: string) => void;
  back: () => void;
  sourceOpen: boolean;
  toggleSource: () => void;
};

const RunViewContext = createContext<RunView | null>(null);

export function useRunView() {
  const view = useContext(RunViewContext);
  if (!view) throw new Error("useRunView needs a RunViewProvider");
  return view;
}

export function RunViewProvider({ children }: { children: ReactNode }) {
  const [graph, setGraph] = useState<RunGraph>(FEATURE_GRAPH);
  const [selectedId, setSelectedId] = useState(FEATURE_GRAPH.defaultSelection);
  const [sourceOpen, setSourceOpen] = useState(false);

  const show = useCallback((next: RunGraph) => {
    setGraph(next);
    setSelectedId(next.defaultSelection);
  }, []);

  const view = useMemo<RunView>(
    () => ({
      graph,
      selectedId,
      selected: graph.nodes.find((node) => node.id === selectedId),
      select: setSelectedId,
      drill: (itemKey) => show(taskCycleGraph(itemKey)),
      back: () => show(FEATURE_GRAPH),
      sourceOpen,
      toggleSource: () => setSourceOpen((open) => !open),
    }),
    [graph, selectedId, sourceOpen, show],
  );

  return (
    <ReactFlowProvider>
      <RunViewContext.Provider value={view}>{children}</RunViewContext.Provider>
    </ReactFlowProvider>
  );
}
