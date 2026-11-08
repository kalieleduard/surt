import { useReactFlow } from "@xyflow/react";
import { ArrowLeft, CodeXml, Hand, LayoutDashboard, Magnet, MousePointer2, Scan, ZoomIn, ZoomOut } from "lucide-react";
import { useRunView } from "../graph/RunViewContext";
import { RunControls } from "./RunControls";

const FIT_PADDING = 0.08;
const ICON = { size: 18, strokeWidth: 1.35 } as const;
const ZOOM_DURATION_MS = 150;
const FIT_DURATION_MS = 300;

export function CanvasToolbar() {
  const { graph, back, sourceOpen, toggleSource, tool, setTool, snap, toggleSnap, resetLayout } = useRunView();
  const flow = useReactFlow();
  const drilled = Boolean(graph?.item);
  return (
    <div className="canvas-toolbar">
      {drilled ? (
        <div className="canvas-toolbar__group">
          <button className="btn btn-ghost" onClick={back}>
            <ArrowLeft size={14} /> {graph?.subject}
          </button>
        </div>
      ) : null}
      {graph?.preview ? null : <RunControls />}
      <div className="canvas-toolbar__group">
        <button className="icon-btn" aria-pressed={tool === "select"} onClick={() => setTool("select")} title="Select (drag to box-select, Ctrl/Shift+click to add)">
          <MousePointer2 {...ICON} />
        </button>
        <button className="icon-btn" aria-pressed={tool === "pan"} onClick={() => setTool("pan")} title="Pan">
          <Hand {...ICON} />
        </button>
        <button className="icon-btn" aria-pressed={snap} onClick={toggleSnap} title="Snap to grid">
          <Magnet {...ICON} />
        </button>
        <button className="icon-btn" onClick={resetLayout} title="Reset layout to the automatic arrangement" disabled={!graph}>
          <LayoutDashboard {...ICON} />
        </button>
      </div>
      <div className="canvas-toolbar__group">
        <button className="icon-btn" title="Zoom out" onClick={() => flow.zoomOut({ duration: ZOOM_DURATION_MS })}>
          <ZoomOut {...ICON} />
        </button>
        <button className="icon-btn" title="Zoom in" onClick={() => flow.zoomIn({ duration: ZOOM_DURATION_MS })}>
          <ZoomIn {...ICON} />
        </button>
        <button className="icon-btn" title="Fit" onClick={() => flow.fitView({ padding: FIT_PADDING, duration: FIT_DURATION_MS })}>
          <Scan {...ICON} />
        </button>
      </div>
      <div className="canvas-toolbar__group">
        <button className="icon-btn" aria-pressed={sourceOpen} onClick={toggleSource} title="Workflow source" disabled={!graph}>
          <CodeXml {...ICON} />
        </button>
      </div>
    </div>
  );
}
