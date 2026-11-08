import { useState } from "react";
import { AlertTriangle, ChevronRight, PanelLeftClose, Pencil, RefreshCw, Search, Workflow } from "lucide-react";
import type { StepState } from "../data/types";
import { useDefinitions } from "../editor/DefinitionsContext";
import { useEngine } from "../engine/EngineContext";
import { useRunView } from "../graph/RunViewContext";
import type { ItemView, RunStateName, SubjectView } from "../engine/types";
import { AgentAvatar, StateDot } from "../ui/status";
import { ago } from "../ui/time";

const RUN_STATE: Record<RunStateName, StepState> = {
  running: "running",
  awaiting_approval: "waiting",
  awaiting_answer: "waiting",
  blocked: "blocked",
  interrupted: "interrupted",
  succeeded: "succeeded",
  cancelled: "cancelled",
};

const ITEM_STATE: Record<ItemView["state"], StepState> = {
  pending: "pending",
  queued: "pending",
  running: "running",
  interrupted: "interrupted",
  blocked: "blocked",
  succeeded: "succeeded",
  cancelled: "cancelled",
};

const DATED_NAME = /^(\d{4})(\d{2})(\d{2})-(.+)$/;
const ENDED_ITEM: ItemView["state"][] = ["succeeded", "cancelled"];

type Tab = "runs" | "library";

export function RunTree({ onCollapse }: { onCollapse?: () => void }) {
  const { subjects, refresh, available } = useEngine();
  const { catalog, reload } = useDefinitions();
  const [tab, setTab] = useState<Tab>("runs");
  const [query, setQuery] = useState("");
  const needle = query.trim().toLowerCase();
  const libraryCount = catalog ? catalog.agents.length + catalog.workflows.length : 0;
  return (
    <aside className="tree">
      <header className="tree__top">
        <div className="tree__tabs" role="tablist">
          <button role="tab" aria-selected={tab === "runs"} onClick={() => setTab("runs")}>
            Runs <span className="mono">{subjects.length}</span>
          </button>
          {available ? (
            <button role="tab" aria-selected={tab === "library"} onClick={() => setTab("library")}>
              Library <span className="mono">{libraryCount}</span>
            </button>
          ) : null}
        </div>
        <button
          className="icon-btn icon-btn--small"
          onClick={tab === "runs" ? refresh : reload}
          aria-label="Refresh"
          title="Refresh"
        >
          <RefreshCw size={12} />
        </button>
        {onCollapse ? (
          <button className="icon-btn icon-btn--small" onClick={onCollapse} aria-label="Collapse sidebar" title="Collapse">
            <PanelLeftClose size={13} />
          </button>
        ) : null}
      </header>
      <label className="search">
        <Search size={13} />
        <input
          placeholder={tab === "runs" ? "Search runs" : "Search agents and flows"}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
      </label>
      {tab === "runs" ? <Runs needle={needle} /> : <Library needle={needle} />}
    </aside>
  );
}

function Runs({ needle }: { needle: string }) {
  const { subjects } = useEngine();
  const shown = subjects.filter((subject) => subject.name.toLowerCase().includes(needle));
  if (!shown.length) return <p className="tree__empty">{subjects.length ? "No run matches" : "No runs yet"}</p>;
  return (
    <ul className="tree__list">
      {shown.map((subject) => (
        <SubjectRow key={subject.name} subject={subject} />
      ))}
    </ul>
  );
}

function SubjectRow({ subject }: { subject: SubjectView }) {
  const { selection, show: showRun } = useEngine();
  const { previewing, preview } = useRunView();
  const show = (name: string, item: string | null) => {
    preview(null);
    showRun(name, item);
  };
  const isActive = !previewing && selection?.subject === subject.name;
  const [open, setOpen] = useState(isActive);
  const hasItems = subject.items.length > 0;
  const expanded = hasItems && (open || (isActive && selection?.item !== null));
  const { date, title } = splitName(subject.name);
  return (
    <li className={`tree__subject${isActive ? " is-active" : ""}`}>
      <div className={`tree__row${isActive && selection?.item === null ? " is-selected" : ""}`}>
        <button
          className={`tree__toggle${expanded ? " is-open" : ""}`}
          onClick={() => setOpen(!expanded)}
          aria-label={expanded ? "Collapse items" : "Expand items"}
          disabled={!hasItems}
        >
          {hasItems ? <ChevronRight size={12} /> : null}
        </button>
        <button
          className="tree__main"
          title={subject.name}
          onClick={() => {
            show(subject.name, null);
            setOpen(true);
          }}
        >
          <span className="tree__title">
            <StateDot state={RUN_STATE[subject.states[0]] ?? "pending"} />
            <span className="tree__slug">{title}</span>
          </span>
          <span className="tree__sub mono">
            {date ? <span>{date}</span> : null}
            <span>{statusLine(subject)}</span>
          </span>
        </button>
      </div>
      {expanded ? (
        <ul className="tree__children">
          {subject.items.map((item) => (
            <li key={item.key}>
              <button
                className={`tree__row tree__row--child${isActive && selection?.item === item.key ? " is-selected" : ""}`}
                onClick={() => show(subject.name, item.key)}
              >
                <StateDot state={ITEM_STATE[item.state]} />
                <span className="tree__item mono">{item.key}</span>
                <span className="tree__meta">{ENDED_ITEM.includes(item.state) ? item.state : (item.step ?? item.state)}</span>
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </li>
  );
}

function Library({ needle }: { needle: string }) {
  const { catalog, catalogError, edit, target } = useDefinitions();
  const { previewing, preview } = useRunView();
  if (catalogError) return <p className="tree__empty">{catalogError}</p>;
  if (!catalog) return <p className="tree__empty">Loading…</p>;
  const workflows = catalog.workflows.filter((workflow) => workflow.name.toLowerCase().includes(needle));
  const agents = catalog.agents.filter((agent) => agent.id.toLowerCase().includes(needle));
  return (
    <div className="tree__list">
      <span className="caption tree__section">Flows</span>
      <ul className="tree__group">
        {workflows.map((workflow) => (
          <li key={workflow.name} className="tree__flow">
            <button
              className={`tree__row tree__row--library${previewing === workflow.name ? " is-selected" : ""}`}
              onClick={() => preview(workflow.name)}
              title={workflow.problem ?? `Show ${workflow.file} on the canvas`}
            >
              <Workflow size={14} className="tree__icon" />
              <span className="tree__slug">{workflow.name}</span>
              {workflow.problem ? <AlertTriangle size={12} className="tree__warn" /> : null}
            </button>
            <button
              className={`icon-btn icon-btn--small tree__flow-edit${target?.kind === "workflow" && target.name === workflow.name ? " is-active" : ""}`}
              onClick={() => edit({ kind: "workflow", name: workflow.name })}
              title={`Edit ${workflow.file}`}
              aria-label={`Edit ${workflow.file}`}
            >
              <Pencil size={12} />
            </button>
          </li>
        ))}
      </ul>
      <span className="caption tree__section">Agents</span>
      <ul className="tree__group">
        {agents.map((agent) => (
          <li key={agent.id}>
            <button
              className={`tree__row tree__row--library${target?.kind === "agent" && target.id === agent.id ? " is-selected" : ""}`}
              onClick={() => edit({ kind: "agent", id: agent.id })}
            >
              <AgentAvatar agent={agent.id} size={18} />
              <span className="tree__slug">{agent.id}</span>
              <span className="tree__meta mono">{agent.files.length} files</span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}

function splitName(name: string): { date: string | null; title: string } {
  const match = DATED_NAME.exec(name);
  if (!match) return { date: null, title: name };
  const [, year, month, day, rest] = match;
  return { date: `${year}-${month}-${day}`, title: rest.replace(/-/g, " ") };
}

function statusLine(subject: SubjectView) {
  const state = subject.states[0]?.replace("_", " ") ?? "";
  const run = subject.runs > 1 ? ` · run ${subject.run}/${subject.runs}` : "";
  const ended = subject.endedAt ? ` · ${ago(subject.endedAt)}` : "";
  return `${state}${run}${ended}`;
}
