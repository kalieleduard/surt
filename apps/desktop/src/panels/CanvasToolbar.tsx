import { useReactFlow } from "@xyflow/react";
import { ArrowLeft, CodeXml, Hand, MousePointer2, Scan, ZoomIn, ZoomOut } from "lucide-react";
import { useRunView } from "../graph/RunViewContext";

const FIT_PADDING = 0.08;
const ZOOM_DURATION_MS = 150;
const FIT_DURATION_MS = 300;

export function CanvasToolbar() {
  const { graph, back, sourceOpen, toggleSource, tool, setTool } = useRunView();
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
      <div className="canvas-toolbar__group">
        <button className="icon-btn" aria-pressed={tool === "select"} onClick={() => setTool("select")} title="Select (drag to box-select)">
          <MousePointer2 size={15} />
        </button>
        <button className="icon-btn" aria-pressed={tool === "pan"} onClick={() => setTool("pan")} title="Pan">
          <Hand size={15} />
        </button>
      </div>
      <div className="canvas-toolbar__group">
        <button className="icon-btn" title="Zoom out" onClick={() => flow.zoomOut({ duration: ZOOM_DURATION_MS })}>
          <ZoomOut size={15} />
        </button>
        <button className="icon-btn" title="Zoom in" onClick={() => flow.zoomIn({ duration: ZOOM_DURATION_MS })}>
          <ZoomIn size={15} />
        </button>
        <button className="icon-btn" title="Fit" onClick={() => flow.fitView({ padding: FIT_PADDING, duration: FIT_DURATION_MS })}>
          <Scan size={15} />
        </button>
      </div>
      <div className="canvas-toolbar__group">
        <button className="icon-btn" aria-pressed={sourceOpen} onClick={toggleSource} title="Workflow source" disabled={!graph}>
          <CodeXml size={15} />
        </button>
      </div>
    </div>
  );
}
