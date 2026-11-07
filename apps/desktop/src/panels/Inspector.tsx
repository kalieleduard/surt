import type { ReactNode } from "react";
import { ArrowUpRight, Plus, X } from "lucide-react";
import { AGENT_LABEL, AGENT_PROFILE, type StepData } from "../data/types";
import { useRunView } from "../graph/RunViewContext";
import { AgentAvatar, STATE_LABEL, StateDot, kindIcon, kindLabel } from "../ui/status";

export function Inspector({ onClose }: { onClose?: () => void }) {
  const { selected } = useRunView();
  if (!selected || selected.type !== "step") return <aside className="inspector inspector--empty caption">Select a step</aside>;
  const step = selected.data as StepData;
  const Icon = kindIcon(step);
  return (
    <aside className="inspector">
      <header className="inspector__head">
        <Icon size={14} />
        <span className="inspector__title">{step.itemKey ? `${step.itemKey} · ${step.title}` : step.name}</span>
        {onClose ? (
          <button className="icon-btn" onClick={onClose} aria-label="Close inspector">
            <X size={14} />
          </button>
        ) : null}
      </header>
      <div className="inspector__state">
        <StateDot state={step.state} />
        <span>{STATE_LABEL[step.state]}</span>
        <span className="inspector__kind mono">{kindLabel(step)}</span>
      </div>
      <div className="inspector__body">
        {step.kind === "item" ? <ChildRunSection step={step} /> : null}
        {step.agent && step.kind !== "item" ? <AgentSection step={step} /> : null}
        {step.outcomes ? <OutcomeSection step={step} /> : null}
        {step.attempt || step.iteration ? <LimitsSection step={step} /> : null}
        <PortSection title="Inputs" ports={step.inputs} />
        <PortSection title="Outputs" ports={step.outputs} />
        {step.executions ? <ExecutionSection step={step} /> : null}
      </div>
    </aside>
  );
}

function Section({ title, children, action = true }: { title: string; children: ReactNode; action?: boolean }) {
  return (
    <section className="insp-section">
      <header className="insp-section__head">
        <span>{title}</span>
        {action ? <Plus size={13} /> : null}
      </header>
      {children}
    </section>
  );
}

function Row({ label, value, mono = true, active = false }: { label: string; value?: ReactNode; mono?: boolean; active?: boolean }) {
  return (
    <div className={`insp-row${active ? " is-active" : ""}`}>
      <span className="insp-row__label">{label}</span>
      <span className={`insp-row__value${mono ? " mono" : ""}`}>{value}</span>
    </div>
  );
}

function ChildRunSection({ step }: { step: StepData }) {
  const { drill } = useRunView();
  return (
    <Section title="Child run" action={false}>
      <Row label="workflow" value="task-cycle" />
      <Row label="current step" value={step.currentStep} />
      <Row label="branch" value={step.branch} />
      <Row label="depends on" value={step.dependsOn?.length ? step.dependsOn.join(", ") : "—"} />
      <button className="btn btn-ghost insp-open" onClick={() => step.itemKey && drill(step.itemKey)}>
        Open task-cycle <ArrowUpRight size={13} />
      </button>
    </Section>
  );
}

function AgentSection({ step }: { step: StepData }) {
  if (!step.agent) return null;
  const profile = AGENT_PROFILE[step.agent];
  return (
    <Section title="Agent profile" action={false}>
      <div className="insp-agent">
        <AgentAvatar agent={step.agent} size={22} />
        <span>{AGENT_LABEL[step.agent]}</span>
        <span className="mono insp-agent__path">.surt/agents/{step.agent}</span>
      </div>
      <Row label="runtime" value={profile.runtime} />
      <Row label="model" value={profile.model} />
      <Row label="effort" value={profile.effort} />
      <Row label="conversation" value={profile.conversation} />
    </Section>
  );
}

function OutcomeSection({ step }: { step: StepData }) {
  return (
    <Section title="Outcome" action={false}>
      {step.outcomes?.map((outcome) => (
        <Row key={outcome} label={outcome} active={outcome === step.outcome} value={outcome === step.outcome ? "✔" : ""} mono={false} />
      ))}
    </Section>
  );
}

function LimitsSection({ step }: { step: StepData }) {
  return (
    <Section title="Limits" action={false}>
      {step.attempt ? <Meter label="attempts" value={step.attempt} /> : null}
      {step.iteration ? <Meter label="iterations" value={step.iteration} /> : null}
    </Section>
  );
}

function Meter({ label, value }: { label: string; value: [number, number] }) {
  return (
    <div className="insp-meter">
      <span className="insp-row__label">{label}</span>
      <span className="insp-meter__bar">
        {Array.from({ length: value[1] }, (_, index) => (
          <i key={index} className={index < value[0] ? "is-used" : undefined} />
        ))}
      </span>
      <span className="mono">{value[0]}/{value[1]}</span>
    </div>
  );
}

function PortSection({ title, ports }: { title: string; ports: StepData["inputs"] }) {
  return (
    <Section title={title}>
      {ports.map((port) => (
        <Row key={port.id + port.label} label={port.label} value={port.revision ?? "—"} />
      ))}
    </Section>
  );
}

function ExecutionSection({ step }: { step: StepData }) {
  return (
    <Section title="Executions" action={false}>
      {step.executions?.map((execution) => (
        <div key={execution.id} className="insp-exec">
          <StateDot state={execution.state} />
          <span className="insp-exec__label">{execution.label}</span>
          <span className="mono insp-exec__time">{execution.duration}</span>
          {execution.note ? <span className="insp-exec__note">{execution.note}</span> : null}
        </div>
      ))}
    </Section>
  );
}
