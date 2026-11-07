import { useState } from "react";
import { PanelLeft } from "lucide-react";
import { SUBJECTS } from "../data/subjects";
import { RunCanvas } from "../graph/RunCanvas";
import { RunViewProvider, useRunView } from "../graph/RunViewContext";
import { CanvasToolbar } from "../panels/CanvasToolbar";
import { TitleBar } from "../panels/chrome";
import { Inspector } from "../panels/Inspector";
import { RunTree } from "../panels/RunTree";
import { SourceOverlay } from "../panels/SourceOverlay";

const FOCUS_PADDING = { top: "76px", right: "330px", bottom: "90px", left: "300px" } as const;

export function FocusLayout() {
  return (
    <RunViewProvider>
      <div className="app">
        <div className="stage stage--full">
          <RunCanvas padding={FOCUS_PADDING} minimap />
          <TitleBar floating />
          <FloatingRuns />
          <FloatingInspector />
          <div className="dock">
            <CanvasToolbar />
          </div>
          <SourceOverlay />
        </div>
      </div>
    </RunViewProvider>
  );
}

function FloatingRuns() {
  const [open, setOpen] = useState(true);
  if (!open) {
    return (
      <button className="floating-pill" onClick={() => setOpen(true)}>
        <PanelLeft size={14} /> Subjects <span className="mono">{SUBJECTS.length}</span>
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
  const { selectedId } = useRunView();
  const [dismissed, setDismissed] = useState<string | null>(null);
  if (dismissed === selectedId) return null;
  return (
    <div className="floating-panel floating-panel--right">
      <Inspector onClose={() => setDismissed(selectedId)} />
    </div>
  );
}
