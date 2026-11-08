import { Handle, Position, type NodeProps } from "@xyflow/react";
import type { StepData } from "../data/types";
import { AgentAvatar, StateMark, kindIcon, kindLabel } from "../ui/status";
import { Presence } from "./Presence";

export function StepNode({ data, selected }: NodeProps) {
  const step = data as StepData;
  const className = `node state-${step.state} kind-${step.kind}${selected ? " is-selected" : ""}`;
  return (
    <div className={className}>
      <span className="node__ring">
        <NodeBadge step={step} />
      </span>
      <span className="node__text">
        <span className="node__name">{step.name}</span>
        <span className="node__meta">{step.outcome ?? kindLabel(step)}</span>
      </span>
      <StateMark state={step.state} />
      <NodeHandles step={step} />
      {step.agent && step.state === "running" ? <Presence agent={step.agent} /> : null}
    </div>
  );
}

function NodeBadge({ step }: { step: StepData }) {
  if (step.agent) return <AgentAvatar agent={step.agent} size={22} />;
  if (step.itemKey) return <span className="node__key">{step.itemKey}</span>;
  const Icon = kindIcon(step);
  return <Icon size={13} />;
}

function NodeHandles({ step }: { step: StepData }) {
  return (
    <>
      {step.inputs.map((port) => (
        <Handle key={`in:${port.id}`} type="target" position={Position.Left} id={`in:${port.id}`} className="port__handle" />
      ))}
      {step.outputs.map((port) => (
        <Handle key={`out:${port.id}`} type="source" position={Position.Right} id={`out:${port.id}`} className="port__handle" />
      ))}
    </>
  );
}
