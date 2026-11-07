import { useState } from "react";
import { useReactFlow } from "@xyflow/react";
import { ArrowLeft, CodeXml, GitBranch, Hand, LayoutGrid, MousePointer2, Plus, Scan, ZoomIn, ZoomOut } from "lucide-react";
import { useRunView } from "../graph/RunViewContext";
import { AddStepPalette } from "./AddStepPalette";
import { RunPath } from "./chrome";

export function CanvasToolbar() {
  const { graph, back, sourceOpen, toggleSource } = useRunView();
  const [tool, setTool] = useState<"select" | "pan">("select");
  const [paletteOpen, setPaletteOpen] = useState(false);
  const flow = useReactFlow();
  return (
    <div className="canvas-toolbar">
      <div className="canvas-toolbar__group">
        <button className="icon-btn" aria-pressed={tool === "select"} onClick={() => setTool("select")} title="Select">
          <MousePointer2 size={15} />
        </button>
        <button className="icon-btn" aria-pressed={tool === "pan"} onClick={() => setTool("pan")} title="Pan">
          <Hand size={15} />
        </button>
        <button className="icon-btn" aria-pressed={paletteOpen} onClick={() => setPaletteOpen(!paletteOpen)} title="Add step">
          <Plus size={16} />
        </button>
        <button className="icon-btn" title="Auto layout" onClick={() => flow.fitView({ padding: 0.08, duration: 300 })}>
          <LayoutGrid size={15} />
        </button>
        {paletteOpen ? <AddStepPalette onClose={() => setPaletteOpen(false)} /> : null}
      </div>
      <div className="canvas-toolbar__center">
        {graph.workflow === "task-cycle" ? (
          <button className="icon-btn" onClick={back} title="Back to feature">
            <ArrowLeft size={15} />
          </button>
        ) : null}
        <RunPath />
      </div>
      <div className="canvas-toolbar__group">
        <button className="icon-btn" title="Zoom out" onClick={() => flow.zoomOut({ duration: 150 })}>
          <ZoomOut size={15} />
        </button>
        <button className="icon-btn" title="Zoom in" onClick={() => flow.zoomIn({ duration: 150 })}>
          <ZoomIn size={15} />
        </button>
        <button className="icon-btn" title="Fit" onClick={() => flow.fitView({ padding: 0.08, duration: 300 })}>
          <Scan size={15} />
        </button>
        <button className="icon-btn" aria-pressed={sourceOpen} onClick={toggleSource} title="Workflow source">
          <CodeXml size={15} />
        </button>
        <button className="icon-btn is-lava" title="Branches">
          <GitBranch size={15} />
        </button>
      </div>
    </div>
  );
}
