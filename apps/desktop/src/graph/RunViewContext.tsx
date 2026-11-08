import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import { ReactFlowProvider } from "@xyflow/react";
import type { GraphNode, RunGraph } from "../data/types";
import { CheckoutProvider } from "../checkout/CheckoutContext";
import { DefinitionsProvider } from "../editor/DefinitionsContext";
import { EngineProvider, useEngine } from "../engine/EngineContext";
import { ScriptsProvider } from "../scripts/ScriptsContext";
import { useWorkflowPreview } from "./useWorkflowPreview";

export type CanvasTool = "select" | "pan";

type RunView = {
  graph: RunGraph | null;
  selectedId: string;
  selected: GraphNode | undefined;
  select: (id: string) => void;
  selections: number;
  drill: (itemKey: string) => void;
  back: () => void;
  sourceOpen: boolean;
  toggleSource: () => void;
  tool: CanvasTool;
  setTool: (tool: CanvasTool) => void;
  snap: boolean;
  toggleSnap: () => void;
  resetToken: number;
  resetLayout: () => void;
  previewing: string | null;
  previewError: string | null;
  preview: (workflow: string | null) => void;
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
      <DefinitionsProvider>
        <ScriptsProvider>
          <CheckoutProvider>
            <ReactFlowProvider>
              <RunViewState>{children}</RunViewState>
            </ReactFlowProvider>
          </CheckoutProvider>
        </ScriptsProvider>
      </DefinitionsProvider>
    </EngineProvider>
  );
}

function RunViewState({ children }: { children: ReactNode }) {
  const engineView = useEngine();
  const workflowPreview = useWorkflowPreview();
  const graph = workflowPreview.name ? workflowPreview.graph : engineView.graph;
  const { drill, back } = engineView;
  const [chosen, setChosen] = useState<{ graph: string; node: string } | null>(null);
  const [sourceOpen, setSourceOpen] = useState(false);
  const [tool, setTool] = useState<CanvasTool>("select");
  const [snap, setSnap] = useState(true);
  const [resetToken, setResetToken] = useState(0);
  const [selections, setSelections] = useState(0);

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
      select: (node) => {
        if (!graph) return;
        setChosen({ graph: graph.id, node });
        setSelections((count) => count + 1);
      },
      selections,
      drill,
      back,
      sourceOpen,
      toggleSource: () => setSourceOpen((open) => !open),
      tool,
      setTool,
      snap,
      toggleSnap: () => setSnap((on) => !on),
      resetToken,
      resetLayout: () => setResetToken((token) => token + 1),
      previewing: workflowPreview.name,
      previewError: workflowPreview.error,
      preview: workflowPreview.open,
    }),
    [graph, selectedId, selections, sourceOpen, tool, drill, back, snap, resetToken, workflowPreview.name, workflowPreview.error, workflowPreview.open],
  );

  return <RunViewContext.Provider value={view}>{children}</RunViewContext.Provider>;
}
