import { Pencil, X } from "lucide-react";
import { Highlighted } from "../code/Code";
import { useDefinitions } from "../editor/DefinitionsContext";
import { useEngine } from "../engine/EngineContext";
import { useRunView } from "../graph/RunViewContext";

export function SourceOverlay() {
  const { graph, sourceOpen, toggleSource } = useRunView();
  const { edit } = useDefinitions();
  const { available } = useEngine();
  if (!sourceOpen || !graph) return null;
  return (
    <div className="source-overlay">
      <header className="source-overlay__head">
        <span className="caption">{graph.workflow}.toml</span>
        <span className="source-overlay__frozen mono">{graph.preview ? "current definition" : `frozen with run ${graph.run}`}</span>
        {available ? (
          <button className="source-overlay__edit" onClick={() => edit({ kind: "workflow", name: graph.workflow })} title="Edit the workflow definition new runs start from">
            <Pencil size={12} /> Edit
          </button>
        ) : null}
        <button className="icon-btn" onClick={toggleSource} aria-label="Close source">
          <X size={14} />
        </button>
      </header>
      <pre className="source-overlay__code mono" data-surface="obsidiana">
        <Highlighted text={graph.source} language="toml" />
      </pre>
    </div>
  );
}
