import { X } from "lucide-react";
import { useRunView } from "../graph/RunViewContext";

export function SourceOverlay() {
  const { graph, sourceOpen, toggleSource } = useRunView();
  if (!sourceOpen || !graph) return null;
  return (
    <div className="source-overlay">
      <header className="source-overlay__head">
        <span className="caption">{graph.workflow}.toml</span>
        <span className="source-overlay__frozen mono">frozen with run {graph.run}</span>
        <button className="icon-btn" onClick={toggleSource} aria-label="Close source">
          <X size={14} />
        </button>
      </header>
      <pre className="source-overlay__code mono">{graph.source}</pre>
    </div>
  );
}
