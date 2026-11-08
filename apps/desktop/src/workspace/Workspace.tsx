import { useState } from "react";
import { PanelLeft } from "lucide-react";
import { useEngine } from "../engine/EngineContext";
import { RunCanvas } from "../graph/RunCanvas";
import { RunViewProvider, useRunView } from "../graph/RunViewContext";
import { CanvasToolbar } from "../panels/CanvasToolbar";
import { TitleBar } from "../panels/chrome";
import { Inspector } from "../panels/Inspector";
import { RunTree } from "../panels/RunTree";
import { SourceOverlay } from "../panels/SourceOverlay";

const CANVAS_PADDING = { top: "80px", right: "330px", bottom: "96px", left: "280px" } as const;

export function Workspace() {
  return (
    <RunViewProvider>
      <div className="workspace">
        <RunCanvas padding={CANVAS_PADDING} />
        <TitleBar />
        <FloatingRuns />
        <FloatingInspector />
        <div className="dock">
          <CanvasToolbar />
        </div>
        <SourceOverlay />
      </div>
    </RunViewProvider>
  );
}

function FloatingRuns() {
  const { subjects, project } = useEngine();
  const [open, setOpen] = useState(true);
  if (!project) return null;
  if (!open) {
    return (
      <button className="floating-pill" onClick={() => setOpen(true)}>
        <PanelLeft size={14} /> Subjects <span className="mono">{subjects.length}</span>
      </button>
    );
  }
  return (
    <div className="floating-panel floating-panel--left">
      <button className="icon-btn floating-panel__collapse" onClick={() => setOpen(false)} aria-label="Collapse subjects">
        <PanelLeft size={14} />
      </button>
      <RunTree />
    </div>
  );
}

function FloatingInspector() {
  const { graph, selectedId } = useRunView();
  const [dismissed, setDismissed] = useState<string | null>(null);
  if (!graph || dismissed === selectedId) return null;
  return (
    <div className="floating-panel floating-panel--right">
      <Inspector onClose={() => setDismissed(selectedId)} />
    </div>
  );
}
