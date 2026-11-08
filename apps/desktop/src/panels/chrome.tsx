import { Fragment, useState } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, FolderOpen, Minus, Plus, X } from "lucide-react";
import { SurtWordmark } from "../brand/SurtMark";
import { CheckoutChips } from "../checkout/CheckoutChips";
import { useEngine } from "../engine/EngineContext";
import { useRunView } from "../graph/RunViewContext";
import { ThemePicker } from "../theme/ThemePicker";
import { NewRunDialog } from "./NewRunDialog";

export function TitleBar() {
  return (
    <div className="titlebar" data-tauri-drag-region>
      <span className="titlebar__brand" data-tauri-drag-region>
        <SurtWordmark size={15} />
      </span>
      <div className="titlebar__center" data-tauri-drag-region>
        <CheckoutChips />
        <RunPath />
      </div>
      <div className="titlebar__actions">
        <SlotMeter />
        <ThemePicker />
        <OpenProject />
        <NewRun />
      </div>
      <WindowControls />
    </div>
  );
}

const DATED_SEGMENT = /^(\d{4})(\d{2})(\d{2})-(.+)$/;

function RunPath() {
  const { graph, back } = useRunView();
  const { project } = useEngine();
  const path = graph?.path ?? (project ? [project.name] : []);
  if (path.length < 2) return null;
  return (
    <nav className="run-path" title={path.join(" / ")} data-tauri-drag-region>
      <span className="run-path__sep">/</span>
      {path.map((segment, index) => {
        if (index === 0) return null;
        const current = index === path.length - 1;
        const link = index === 1 && path.length > 2;
        const dated = DATED_SEGMENT.exec(segment);
        return (
          <Fragment key={`${index}:${segment}`}>
            {index > 1 ? <span className="run-path__sep">/</span> : null}
            {dated ? <span className="run-path__date">{`${dated[1]}-${dated[2]}-${dated[3]}`}</span> : null}
            <button
              className={`run-path__segment${current ? " is-current" : ""}${link ? " is-link" : ""}`}
              onClick={link ? back : undefined}
              title={segment}
            >
              {dated ? dated[4] : segment}
            </button>
          </Fragment>
        );
      })}
    </nav>
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

function NewRun() {
  const { project } = useEngine();
  const [open, setOpen] = useState(false);
  if (!project) return null;
  return (
    <>
      <button className="btn btn-primary" onClick={() => setOpen(true)} title="Start a new Workflow Run">
        <Plus size={13} /> New run
      </button>
      {open ? <NewRunDialog onClose={() => setOpen(false)} /> : null}
    </>
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
