import { X } from "lucide-react";
import { WORKFLOW_SOURCE } from "../data/workflowSource";
import { useRunView } from "../graph/RunViewContext";

export function SourceOverlay() {
  const { graph, sourceOpen, toggleSource } = useRunView();
  if (!sourceOpen) return null;
  return (
    <div className="source-overlay">
      <header className="source-overlay__head">
        <span className="caption">.surt/workflows/{graph.workflow}.toml</span>
        <span className="source-overlay__frozen mono">frozen with run 1</span>
        <button className="icon-btn" onClick={toggleSource} aria-label="Close source">
          <X size={14} />
        </button>
      </header>
      <pre className="source-overlay__code mono">{WORKFLOW_SOURCE[graph.workflow]}</pre>
    </div>
  );
}
