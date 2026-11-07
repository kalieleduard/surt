import { useState } from "react";
import { ChevronDown, ChevronRight, GitBranch, Plus, RefreshCw, Search } from "lucide-react";
import { ACTIVE_SUBJECT, SUBJECTS } from "../data/subjects";
import type { Subject } from "../data/types";
import { useRunView } from "../graph/RunViewContext";
import { StateDot } from "../ui/status";

export function RunTree() {
  const [tab, setTab] = useState<"runs" | "definitions">("runs");
  return (
    <aside className="tree">
      <div className="tabs">
        <button className={tab === "runs" ? "is-active" : undefined} onClick={() => setTab("runs")}>Runs</button>
        <button className={tab === "definitions" ? "is-active" : undefined} onClick={() => setTab("definitions")}>Definitions</button>
      </div>
      <label className="search">
        <Search size={13} />
        <input placeholder="Search subjects" />
      </label>
      <div className="tree__header">
        <span className="caption">{tab === "runs" ? "Subjects" : ".surt/"}</span>
        <span className="tree__header-actions">
          <RefreshCw size={12} />
          <Plus size={13} />
        </span>
      </div>
      {tab === "runs" ? <SubjectList /> : <DefinitionList />}
    </aside>
  );
}

function SubjectList() {
  return (
    <ul className="tree__list">
      {SUBJECTS.map((subject) => (
        <SubjectRow key={subject.id} subject={subject} />
      ))}
    </ul>
  );
}

function SubjectRow({ subject }: { subject: Subject }) {
  const { graph, drill, back } = useRunView();
  const [open, setOpen] = useState(subject.id === ACTIVE_SUBJECT);
  const isActive = subject.id === ACTIVE_SUBJECT;
  const drilledKey = graph.workflow === "task-cycle" ? graph.path[graph.path.length - 1] : null;
  const Chevron = open ? ChevronDown : ChevronRight;
  return (
    <li>
      <button className={`tree__row${isActive && !drilledKey ? " is-selected" : ""}`} onClick={() => (isActive ? back() : setOpen(!open))}>
        <Chevron size={12} className="tree__chevron" onClick={(event) => { event.stopPropagation(); setOpen(!open); }} />
        <StateDot state={subject.state} />
        <span className="tree__label">
          <span className="tree__id mono">{subject.id}</span>
          <span className="tree__slug">{subject.slug}</span>
        </span>
      </button>
      {open ? <div className="tree__status">{subject.status}</div> : null}
      {open && subject.children ? (
        <ul className="tree__children">
          <li className="tree__row tree__row--static">
            <GitBranch size={12} />
            <span className="tree__label">feature</span>
            <span className="tree__meta mono">run 1</span>
          </li>
          {subject.children.map((child) => (
            <li key={child.key}>
              <button className={`tree__row tree__row--child${drilledKey === child.key ? " is-selected" : ""}`} onClick={() => drill(child.key)}>
                <StateDot state={child.state} />
                <span className="tree__label">
                  <span className="mono">{child.key}</span> {child.title}
                </span>
                <span className="tree__meta">{child.status}</span>
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </li>
  );
}

const DEFINITIONS = [
  { name: "workflows/feature.toml", detail: "12 steps" },
  { name: "workflows/task-cycle.toml", detail: "6 steps" },
  { name: "agents/spec", detail: "opus · loop" },
  { name: "agents/tasks", detail: "opus" },
  { name: "agents/builder", detail: "sonnet · loop" },
  { name: "agents/reviewer", detail: "opus" },
  { name: "agents/shipper", detail: "sonnet" },
  { name: "config.toml", detail: "3 slots" },
];

function DefinitionList() {
  return (
    <ul className="tree__list">
      {DEFINITIONS.map((definition) => (
        <li key={definition.name} className="tree__row tree__row--static">
          <span className="tree__label mono">{definition.name}</span>
          <span className="tree__meta">{definition.detail}</span>
        </li>
      ))}
    </ul>
  );
}
