import { Handle, Position, type NodeProps } from "@xyflow/react";
import type { Port, StepData } from "../data/types";
import { useChoice } from "../prototype/ChoiceContext";
import { AgentAvatar, StateMark, kindIcon, kindLabel } from "../ui/status";
import { Presence } from "./Presence";

export function StepNode({ data, selected }: NodeProps) {
  const step = data as StepData;
  const { node } = useChoice();
  const className = `node node--${node} state-${step.state} kind-${step.kind}${selected ? " is-selected" : ""}`;
  return (
    <div className={className}>
      {node === "card" ? <CardBody step={step} /> : null}
      {node === "compact" ? <CompactBody step={step} /> : null}
      {node === "terminal" ? <TerminalBody step={step} /> : null}
      {node === "card" ? null : <StackedHandles step={step} />}
      {step.presence && step.state === "running" ? <Presence agent={step.presence} /> : null}
    </div>
  );
}

function NodeHead({ step }: { step: StepData }) {
  const Icon = kindIcon(step);
  return (
    <header className="node__head">
      {step.itemKey ? <span className="node__key">{step.itemKey}</span> : <Icon size={13} className="node__icon" />}
      <span className="node__name">{step.itemKey ? step.title : step.name}</span>
      <StateMark state={step.state} />
    </header>
  );
}

function CardBody({ step }: { step: StepData }) {
  const rows = Math.max(step.inputs.length, step.outputs.length);
  return (
    <>
      <NodeHead step={step} />
      <div className="node__sub">{kindLabel(step)}</div>
      <div className="node__ports">
        {Array.from({ length: rows }, (_, index) => (
          <div className="node__row" key={index}>
            <PortCell port={step.inputs[index]} side="in" />
            <PortCell port={step.outputs[index]} side="out" />
          </div>
        ))}
      </div>
      <NodeFoot step={step} />
    </>
  );
}

function PortCell({ port, side }: { port?: Port; side: "in" | "out" }) {
  if (!port) return <span />;
  return (
    <span className={`port port--${side}`}>
      <Handle
        type={side === "in" ? "target" : "source"}
        position={side === "in" ? Position.Left : Position.Right}
        id={`${side}:${port.id}`}
        className="port__handle"
      />
      <span className="port__label">{port.label}</span>
      {port.revision ? <span className="port__rev">{port.revision}</span> : null}
    </span>
  );
}

function NodeFoot({ step }: { step: StepData }) {
  const chips = [
    step.attempt ? `attempt ${step.attempt[0]}/${step.attempt[1]}` : null,
    step.iteration ? `iter ${step.iteration[0]}/${step.iteration[1]}` : null,
    step.duration ?? null,
  ].filter(Boolean);
  if (!chips.length && !step.outcome) return null;
  return (
    <footer className="node__foot">
      {chips.map((chip) => (
        <span key={chip} className="chip">{chip}</span>
      ))}
      {step.outcome ? <span className="chip chip--outcome">{step.outcome}</span> : null}
    </footer>
  );
}

function CompactBody({ step }: { step: StepData }) {
  const Icon = kindIcon(step);
  return (
    <>
      <span className="compact__ring">
        {step.agent ? <AgentAvatar agent={step.agent} size={22} /> : step.itemKey ? <span className="node__key">{step.itemKey}</span> : <Icon size={13} />}
      </span>
      <span className="compact__text">
        <span className="node__name">{step.itemKey ? step.title : step.name}</span>
        <span className="compact__meta">{step.outcome ?? kindLabel(step)}</span>
      </span>
      <StateMark state={step.state} />
    </>
  );
}

function TerminalBody({ step }: { step: StepData }) {
  const lines = step.tail ?? [];
  return (
    <>
      <header className="terminal__bar">
        <span className="caption">{step.itemKey ? `item ${step.itemKey}` : step.name}</span>
        <span className="terminal__who">{kindLabel(step)}</span>
        <StateMark state={step.state} />
      </header>
      <pre className="terminal__body">
        {lines.map((line) => (
          <span key={line} className="terminal__line">{line}</span>
        ))}
        {step.state === "running" ? <span className="terminal__caret">▌</span> : null}
      </pre>
    </>
  );
}

function StackedHandles({ step }: { step: StepData }) {
  return (
    <>
      {step.inputs.map((port) => (
        <Handle key={`in:${port.id}`} type="target" position={Position.Left} id={`in:${port.id}`} className="port__handle port__handle--stacked" />
      ))}
      {step.outputs.map((port) => (
        <Handle key={`out:${port.id}`} type="source" position={Position.Right} id={`out:${port.id}`} className="port__handle port__handle--stacked" />
      ))}
    </>
  );
}
