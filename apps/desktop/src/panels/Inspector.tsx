import { useState } from "react";
import { X } from "lucide-react";
import type { FrameData, GraphNode, StepData, StepState } from "../data/types";
import { useRunView } from "../graph/RunViewContext";
import { STATE_LABEL, StateDot, kindIcon, kindLabel } from "../ui/status";
import { Row, Section, StepRunDetails } from "./RunDetails";
import { StepDefinition } from "./StepDefinition";

type Tab = "definition" | "run";

export function Inspector({ onClose }: { onClose?: () => void }) {
  const { selected } = useRunView();
  const [chosen, setChosen] = useState<Tab | null>(null);
  if (!selected) return <aside className="inspector inspector--empty caption">Select a step</aside>;
  const view = describe(selected);
  const tab = chosen ?? view.defaultTab;
  const Icon = view.icon;
  return (
    <aside className="inspector">
      <header className="inspector__head">
        <Icon size={14} />
        <span className="inspector__title">{view.title}</span>
        {onClose ? (
          <button className="icon-btn" onClick={onClose} aria-label="Close inspector">
            <X size={14} />
          </button>
        ) : null}
      </header>
      <div className="inspector__state">
        <StateDot state={view.state} />
        <span>{STATE_LABEL[view.state]}</span>
        <span className="inspector__kind mono">{view.kind}</span>
      </div>
      <div className="inspector__tabs" role="tablist">
        <TabButton tab="definition" current={tab} onSelect={setChosen} label="Definition" />
        <TabButton tab="run" current={tab} onSelect={setChosen} label={view.waiting ? "Run · action needed" : "Run"} attention={view.waiting} />
      </div>
      <div className="inspector__body">
        {tab === "definition" ? <StepDefinition stepKey={view.stepKey} /> : <RunTab node={selected} />}
      </div>
    </aside>
  );
}

function TabButton({ tab, current, onSelect, label, attention }: { tab: Tab; current: Tab; onSelect: (tab: Tab) => void; label: string; attention?: boolean }) {
  return (
    <button role="tab" aria-selected={tab === current} className={attention ? "is-attention" : undefined} onClick={() => onSelect(tab)}>
      {label}
    </button>
  );
}

function RunTab({ node }: { node: GraphNode }) {
  if (node.type === "step") return <StepRunDetails step={node.data as StepData} />;
  const frame = node.data as FrameData;
  return (
    <Section title={frame.kind === "loop" ? "Loop" : "Per item"}>
      <Row label="state" value={STATE_LABEL[frame.state]} mono={false} />
      <Row label="progress" value={frame.detail} />
    </Section>
  );
}

type View = { title: string; icon: ReturnType<typeof kindIcon>; state: StepState; kind: string; stepKey: string; defaultTab: Tab; waiting: boolean };

function describe(node: GraphNode): View {
  if (node.type === "frame") {
    const frame = node.data as FrameData;
    return { title: frame.name, icon: kindIcon({ kind: frame.kind }), state: frame.state, kind: frame.caption, stepKey: node.id, defaultTab: "definition", waiting: false };
  }
  const step = node.data as StepData;
  const waiting = step.gate?.status === "waiting";
  const item = step.kind === "item";
  return {
    title: item ? `Task ${step.itemKey}` : step.name,
    icon: kindIcon(step),
    state: step.state,
    kind: kindLabel(step),
    stepKey: item ? (node.parentId ?? step.key) : step.key,
    defaultTab: waiting || item ? "run" : "definition",
    waiting,
  };
}
