import { useState } from "react";
import { ChevronDown, ChevronRight, RefreshCw, Search } from "lucide-react";
import type { StepState } from "../data/types";
import { useEngine } from "../engine/EngineContext";
import type { ItemView, RunStateName, SubjectView } from "../engine/types";
import { StateDot } from "../ui/status";
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

const DATED_NAME = /^(\d{8})-(.+)$/;
const ENDED_ITEM: ItemView["state"][] = ["succeeded", "cancelled"];

export function RunTree() {
  const { subjects, refresh, project } = useEngine();
  const [query, setQuery] = useState("");
  const shown = subjects.filter((subject) => subject.name.includes(query.trim()));
  return (
    <aside className="tree">
      <label className="search">
        <Search size={13} />
        <input placeholder="Search subjects" value={query} onChange={(event) => setQuery(event.target.value)} />
      </label>
      <div className="tree__header">
        <span className="caption">{project ? `Subjects · ${subjects.length}` : "Subjects"}</span>
        <span className="tree__header-actions">
          <button className="icon-btn icon-btn--small" onClick={refresh} aria-label="Refresh runs" title="Refresh">
            <RefreshCw size={12} />
          </button>
        </span>
      </div>
      <ul className="tree__list">
        {shown.map((subject) => (
          <SubjectRow key={subject.name} subject={subject} />
        ))}
      </ul>
    </aside>
  );
}

function SubjectRow({ subject }: { subject: SubjectView }) {
  const { selection, show } = useEngine();
  const isActive = selection?.subject === subject.name;
  const [open, setOpen] = useState(isActive);
  const expanded = open || (isActive && selection?.item !== null);
  const Chevron = expanded ? ChevronDown : ChevronRight;
  const [date, slug] = splitName(subject.name);
  return (
    <li>
      <button
        className={`tree__row${isActive && selection?.item === null ? " is-selected" : ""}`}
        onClick={() => {
          show(subject.name, null);
          setOpen(true);
        }}
      >
        <Chevron
          size={12}
          className="tree__chevron"
          onClick={(event) => {
            event.stopPropagation();
            setOpen(!expanded);
          }}
        />
        <StateDot state={RUN_STATE[subject.states[0]] ?? "pending"} />
        <span className="tree__label">
          <span className="tree__id mono">{date}</span>
          <span className="tree__slug">{slug}</span>
        </span>
      </button>
      {expanded ? <div className="tree__status">{statusLine(subject)}</div> : null}
      {expanded && subject.items.length ? (
        <ul className="tree__children">
          {subject.items.map((item) => (
            <li key={item.key}>
              <button
                className={`tree__row tree__row--child${isActive && selection?.item === item.key ? " is-selected" : ""}`}
                onClick={() => show(subject.name, item.key)}
              >
                <StateDot state={ITEM_STATE[item.state]} />
                <span className="tree__label">
                  <span className="mono">{item.key}</span>
                </span>
                <span className="tree__meta">{ENDED_ITEM.includes(item.state) ? item.state : (item.step ?? item.state)}</span>
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </li>
  );
}

function splitName(name: string): [string, string] {
  const match = DATED_NAME.exec(name);
  return match ? [match[1], match[2]] : [name, ""];
}

function statusLine(subject: SubjectView) {
  const states = subject.states.map((state) => state.replace("_", " ")).join(" · ");
  const run = subject.runs > 1 ? ` · run ${subject.run}/${subject.runs}` : "";
  const ended = subject.endedAt ? ` · ${ago(subject.endedAt)}` : "";
  return `${states}${run}${ended}`;
}
