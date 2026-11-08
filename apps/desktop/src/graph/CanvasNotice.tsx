import type { ReactNode } from "react";
import { FolderOpen } from "lucide-react";
import { useEngine } from "../engine/EngineContext";
import { useRunView } from "./RunViewContext";

export function CanvasNotice() {
  const { available, project, projectError, chooseProject, runs, subjects, runsError, graph, detailError } = useEngine();
  const { previewing, previewError, graph: shown } = useRunView();
  if (previewing) {
    if (shown) return null;
    return <Notice title={previewError ? `Surt could not read ${previewing}` : `Reading ${previewing}`} error={previewError} />;
  }
  if (!available) {
    return (
      <Notice title="Surt reads runs in the desktop app">
        <code className="mono">npm run tauri dev</code>
      </Notice>
    );
  }
  if (!project) {
    return (
      <Notice title="Open a Surt project" error={projectError}>
        <button className="btn btn-primary" onClick={chooseProject}>
          <FolderOpen size={14} /> Open project
        </button>
      </Notice>
    );
  }
  const error = runsError ?? detailError;
  if (error && !graph) return <Notice title="Surt could not read this project" error={error} />;
  if (runs && subjects.length === 0) {
    return (
      <Notice title="No runs yet">
        <code className="mono">surt run feature "what to build"</code>
      </Notice>
    );
  }
  if (!graph) return <Notice title="Reading runs" />;
  return null;
}

function Notice({ title, error, children }: { title: string; error?: string | null; children?: ReactNode }) {
  return (
    <div className="canvas-notice">
      <div className="canvas-notice__card">
        <span className="canvas-notice__title">{title}</span>
        {children}
        {error ? <span className="canvas-notice__error mono">{error}</span> : null}
      </div>
    </div>
  );
}
