import { Fragment } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Play, X } from "lucide-react";
import { SurtWordmark } from "../brand/SurtMark";
import { AGENT_SLOTS } from "../data/subjects";
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
        <button className="btn btn-primary">
          <Play size={13} fill="currentColor" /> New run
        </button>
      </div>
      <WindowControls />
    </div>
  );
}

function RunPath() {
  const { graph, back } = useRunView();
  return (
    <span className="run-path">
      {graph.path.map((segment, index) => (
        <Fragment key={segment}>
          {index > 0 ? <span className="run-path__sep">/</span> : null}
          <button
            className={`run-path__segment${index === graph.path.length - 1 ? " is-current" : ""}`}
            onClick={index > 0 && index < graph.path.length - 1 ? back : undefined}
          >
            {segment}
          </button>
        </Fragment>
      ))}
    </span>
  );
}

function SlotMeter() {
  const { used, total } = AGENT_SLOTS;
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
