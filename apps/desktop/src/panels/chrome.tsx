import { Fragment } from "react";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Bot, Copy, Minus, Play, ScrollText, Settings, Workflow, X, Waypoints } from "lucide-react";
import { SurtWordmark } from "../brand/SurtMark";
import { RUN_SUMMARY } from "../data/subjects";
import { useRunView } from "../graph/RunViewContext";

const MENUS = ["File", "Edit", "View", "Run", "Window", "Help"];

export function TitleBar({ floating = false }: { floating?: boolean }) {
  return (
    <div className={`titlebar${floating ? " titlebar--floating" : ""}`} data-tauri-drag-region>
      <span className="titlebar__brand" data-tauri-drag-region>
        <SurtWordmark size={15} />
      </span>
      {floating ? null : (
        <nav className="titlebar__menu">
          {MENUS.map((menu) => (
            <button key={menu} className="titlebar__menu-item">{menu}</button>
          ))}
        </nav>
      )}
      <div className="titlebar__center" data-tauri-drag-region>
        <RunPath />
      </div>
      <div className="titlebar__actions">
        <SlotMeter />
        <button className="btn btn-primary">
          <Play size={13} fill="currentColor" /> New run
        </button>
      </div>
      <WindowControls />
    </div>
  );
}

export function RunPath() {
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

export function SlotMeter() {
  const { used, total } = RUN_SUMMARY.slots;
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

export function WindowControls() {
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

const RAIL = [
  { id: "runs", label: "Runs", icon: Waypoints },
  { id: "workflows", label: "Workflows", icon: Workflow },
  { id: "agents", label: "Agents", icon: Bot },
  { id: "logs", label: "Logs", icon: ScrollText },
];

export function IconRail() {
  return (
    <nav className="rail">
      {RAIL.map(({ id, label, icon: Icon }) => (
        <button key={id} className={`rail__item${id === "runs" ? " is-active" : ""}`}>
          <Icon size={17} />
          <span>{label}</span>
        </button>
      ))}
      <span className="rail__spacer" />
      <button className="rail__item">
        <Settings size={17} />
        <span>Settings</span>
      </button>
    </nav>
  );
}
