import type { ReactNode } from "react";
import { ArrowUpRight, X } from "lucide-react";
import type { Execution, StepData } from "../data/types";
import { useEngine } from "../engine/EngineContext";
import { useRunView } from "../graph/RunViewContext";
import { AgentAvatar, STATE_LABEL, StateDot, kindIcon, kindLabel } from "../ui/status";
import { duration, moment } from "../ui/time";
import { GateActions } from "./GateActions";

export function Inspector({ onClose }: { onClose?: () => void }) {
  const { selected } = useRunView();
  if (!selected || selected.type !== "step") return <aside className="inspector inspector--empty caption">Select a step</aside>;
  const step = selected.data as StepData;
  const Icon = kindIcon(step);
  return (
    <aside className="inspector">
      <header className="inspector__head">
        <Icon size={14} />
        <span className="inspector__title">{step.itemKey ? `Task ${step.itemKey}` : step.name}</span>
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
        {step.kind === "item" ? <ChildRunSection step={step} /> : <StepSection step={step} />}
        {step.gate ? <GateSection step={step} /> : null}
        {step.kind !== "item" ? <LimitsSection step={step} /> : null}
        {step.published.length ? <PublishedSection step={step} /> : null}
        {step.executions.length ? <ExecutionSection executions={step.executions} /> : null}
      </div>
    </aside>
  );
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="insp-section">
      <header className="insp-section__head">
        <span>{title}</span>
      </header>
      {children}
    </section>
  );
}

function Row({ label, value, mono = true }: { label: string; value?: ReactNode; mono?: boolean }) {
  return (
    <div className="insp-row">
      <span className="insp-row__label">{label}</span>
      <span className={`insp-row__value${mono ? " mono" : ""}`}>{value ?? "—"}</span>
    </div>
  );
}

function StepSection({ step }: { step: StepData }) {
  return (
    <Section title="Step">
      {step.agent ? (
        <div className="insp-agent">
          <AgentAvatar agent={step.agent} size={22} />
          <span>{step.agent}</span>
          <span className="mono insp-agent__path">.surt/agents/{step.agent}</span>
        </div>
      ) : null}
      {step.operation ? <Row label="operation" value={step.operation} /> : null}
      {step.artifact ? <Row label="artifact" value={step.artifact} /> : null}
      <Row label="outcome" value={step.outcome} />
      {step.condition ? <Row label="runs when" value={step.condition} /> : null}
      {step.executions.at(-1)?.runtime ? <Row label="runtime" value={step.executions.at(-1)?.runtime} /> : null}
    </Section>
  );
}

function ChildRunSection({ step }: { step: StepData }) {
  const { drill } = useRunView();
  return (
    <Section title="Child run">
      <Row label="workflow" value={step.workflow} />
      <Row label="current step" value={step.currentStep} />
      <Row label="iteration" value={step.iteration} />
      <Row label="attempt" value={step.attempts || undefined} />
      <Row label="merged as" value={step.mergedAs?.slice(0, 7)} />
      <Row label="needs" value={step.needs?.length ? step.needs.join(", ") : undefined} />
      <Row label="waits for" value={step.waitsFor?.length ? step.waitsFor.join(", ") : undefined} />
      <button className="btn btn-ghost insp-open" onClick={() => step.itemKey && drill(step.itemKey)}>
        Open {step.workflow ?? "child run"} <ArrowUpRight size={13} />
      </button>
    </Section>
  );
}

function GateSection({ step }: { step: StepData }) {
  const { graph } = useRunView();
  const gate = step.gate;
  if (!gate) return null;
  return (
    <Section title={gate.kind === "approval" ? "Approval gate" : "Input step"}>
      <Row label="status" value={gate.status.replace("_", " ")} />
      <Row label="revision" value={gate.revision ? `${gate.artifact} r${gate.revision}` : undefined} />
      <Row label="decided by" value={gate.decidedBy} mono={false} />
      <Row label="decided at" value={gate.decidedAt ? moment(gate.decidedAt) : undefined} />
      {graph && !graph.item ? <GateActions subject={graph.subject} gate={gate} /> : null}
    </Section>
  );
}

function LimitsSection({ step }: { step: StepData }) {
  const { project } = useEngine();
  if (!project || (!step.attempts && !step.iteration)) return null;
  return (
    <Section title="Limits">
      {step.attempts ? <Meter label="attempts" used={step.attempts} total={project.maxAttempts} /> : null}
      {step.iteration ? <Meter label="iterations" used={step.iteration} total={project.maxIterations} /> : null}
    </Section>
  );
}

function Meter({ label, used, total }: { label: string; used: number; total: number }) {
  return (
    <div className="insp-meter">
      <span className="insp-row__label">{label}</span>
      <span className="insp-meter__bar">
        {Array.from({ length: Math.max(total, used) }, (_, index) => (
          <i key={index} className={index < used ? "is-used" : undefined} />
        ))}
      </span>
      <span className="mono">
        {used}/{total}
      </span>
    </div>
  );
}

function PublishedSection({ step }: { step: StepData }) {
  return (
    <Section title="Published">
      {step.published.map((entry) => (
        <Row key={entry.artifact} label={entry.artifact} value={`r${entry.revision}`} />
      ))}
    </Section>
  );
}

function ExecutionSection({ executions }: { executions: Execution[] }) {
  return (
    <Section title="Executions">
      {[...executions].reverse().map((execution) => (
        <div key={execution.id} className="insp-exec" title={execution.id}>
          <StateDot state={execution.state} />
          <span className="insp-exec__label">{execution.label}</span>
          <span className="mono insp-exec__time">{duration(execution.startedAt, execution.endedAt)}</span>
          {execution.note ? <span className="insp-exec__note">{execution.note}</span> : null}
        </div>
      ))}
    </Section>
  );
}
