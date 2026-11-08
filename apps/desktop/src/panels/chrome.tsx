import { Fragment } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, FolderOpen, Minus, X } from "lucide-react";
import { SurtWordmark } from "../brand/SurtMark";
import { useEngine } from "../engine/EngineContext";
import { useRunView } from "../graph/RunViewContext";
import { ThemePicker } from "../theme/ThemePicker";

export function TitleBar() {
  return (
    <div className="titlebar" data-tauri-drag-region>
      <span className="titlebar__brand" data-tauri-drag-region>
        <SurtWordmark size={15} />
      </span>
      <div className="titlebar__center" data-tauri-drag-region>
        <RunPath />
      </div>
      <div className="titlebar__actions">
        <SlotMeter />
        <ThemePicker />
        <OpenProject />
      </div>
      <WindowControls />
    </div>
  );
}

function RunPath() {
  const { graph, back } = useRunView();
  const { project } = useEngine();
  const path = graph?.path ?? (project ? [project.name] : []);
  return (
    <span className="run-path" title={project?.root}>
      {path.map((segment, index) => (
        <Fragment key={`${index}:${segment}`}>
          {index > 0 ? <span className="run-path__sep">/</span> : null}
          <button
            className={`run-path__segment${index === path.length - 1 ? " is-current" : ""}`}
            onClick={index === 1 && path.length > 2 ? back : undefined}
          >
            {segment}
          </button>
        </Fragment>
      ))}
    </span>
  );
}

function OpenProject() {
  const { available, project, chooseProject } = useEngine();
  if (!available) return null;
  return (
    <button className={`btn ${project ? "btn-ghost" : "btn-primary"}`} onClick={chooseProject} title="Open a Surt project">
      <FolderOpen size={13} /> {project ? "Project" : "Open project"}
    </button>
  );
}

function SlotMeter() {
  const { runs } = useEngine();
  if (!runs) return null;
  const { inUse: used, total } = runs.slots;
  return (
    <span className="slot-meter" title="Agent Slots in use">
      <span className="caption">slots</span>
      {Array.from({ length: total }, (_, index) => (
        <i key={index} className={index < used ? "is-used" : undefined} />
      ))}
      <span className="slot-meter__count mono">{used}/{total}</span>
    </span>
  );
}

function WindowControls() {
  const native = isTauri();
  const run = (action: "minimize" | "toggleMaximize" | "close") => {
    if (native) getCurrentWindow()[action]();
  };
  return (
    <span className="window-controls">
      <button aria-label="Minimize" onClick={() => run("minimize")}>
        <Minus size={14} />
      </button>
      <button aria-label="Maximize" onClick={() => run("toggleMaximize")}>
        <Copy size={12} />
      </button>
      <button aria-label="Close" className="is-close" onClick={() => run("close")}>
        <X size={15} />
      </button>
    </span>
  );
}
