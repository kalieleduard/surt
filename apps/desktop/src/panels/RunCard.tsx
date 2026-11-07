import { useState, type ReactNode } from "react";
import { ChevronDown, Gauge, ListTodo, ScrollText, Settings2, Square, Waypoints, X } from "lucide-react";
import { RUN_SUMMARY } from "../data/subjects";
import { TASKS } from "../data/tasks";
import { useRunView } from "../graph/RunViewContext";
import { StateMark } from "../ui/status";

export function RunCard() {
  const [tab, setTab] = useState<"run" | "inbox">("run");
  const [collapsed, setCollapsed] = useState(false);
  const { drill } = useRunView();
  if (collapsed) {
    return (
      <button className="run-card-pill" onClick={() => setCollapsed(false)}>
        <span className="run-card__dot" />
        <span className="mono">{RUN_SUMMARY.subject}</span>
        <b>{RUN_SUMMARY.stepsDone}/{RUN_SUMMARY.stepsTotal}</b>
        <span className="caption">steps</span>
        <ChevronDown size={14} />
      </button>
    );
  }
  return (
    <div className="run-card">
      <div className="run-card__top">
        <span className="run-card__dot" />
        <div className="tabs tabs--pill">
          <button className={tab === "run" ? "is-active" : undefined} onClick={() => setTab("run")}>Run</button>
          <button className={tab === "inbox" ? "is-active" : undefined} onClick={() => setTab("inbox")}>Inbox · 2</button>
        </div>
        <Settings2 size={15} />
      </div>
      <div className="run-card__hero">
        <span className="caption">{RUN_SUMMARY.subject} · {RUN_SUMMARY.slug}</span>
        <span className="run-card__value">
          {RUN_SUMMARY.stepsDone}
          <small>/{RUN_SUMMARY.stepsTotal} steps</small>
        </span>
        <span className="run-card__delta">{RUN_SUMMARY.delta}</span>
      </div>
      <div className="run-card__actions">
        <RoundAction icon={<Square size={15} />} label="Stop" />
        <RoundAction icon={<ScrollText size={16} />} label="Follow" />
        <RoundAction icon={<X size={17} />} label="Cancel" />
      </div>
      <div className="run-card__sheet">
        <span className="run-card__grip" />
        <header className="run-card__sheet-head">
          <span>Tasks</span>
          <button className="run-card__close" aria-label="Collapse" onClick={() => setCollapsed(true)}>
            <X size={13} />
          </button>
        </header>
        {TASKS.map((task) => (
          <button key={task.key} className={`run-card__task state-${task.state}`} onClick={() => drill(task.key)}>
            <span className="run-card__key">{task.key}</span>
            <span className="run-card__task-text">
              <span>{task.title}</span>
              <small>{task.status}</small>
            </span>
            <span className="run-card__task-end">
              <span className="mono">{task.diff ?? "—"}</span>
              <StateMark state={task.state} />
            </span>
          </button>
        ))}
      </div>
      <nav className="run-card__nav">
        <span className="is-active"><Waypoints size={16} />Run</span>
        <span><ListTodo size={16} />Tasks</span>
        <span><Gauge size={16} />Slots</span>
      </nav>
    </div>
  );
}

function RoundAction({ icon, label }: { icon: ReactNode; label: string }) {
  return (
    <button className="run-card__action">
      <span>{icon}</span>
      {label}
    </button>
  );
}
