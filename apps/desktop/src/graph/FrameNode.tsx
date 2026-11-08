import { Handle, Position, type NodeProps } from "@xyflow/react";
import type { FrameData, Port } from "../data/types";
import { ENTER, LEAVE } from "../engine/runEdges";
import { StateMark, kindIcon } from "../ui/status";

const DEFAULT_PORT_Y = 68;

export function FrameNode({ data, selected }: NodeProps) {
  const frame = data as FrameData;
  const Icon = kindIcon({ kind: frame.kind });
  return (
    <div className={`frame frame--${frame.kind} state-${frame.state}${selected ? " is-selected" : ""}`}>
      <span className="frame__shape" />
      <header className="frame__head">
        <Icon size={13} className="frame__icon" />
        <span className="frame__name">{frame.name}</span>
        <span className="caption">{frame.caption}</span>
        <span className="frame__detail mono">{frame.detail}</span>
        <StateMark state={frame.state} />
      </header>
      <FramePorts ports={frame.inputs} side="in" />
      <FramePorts ports={frame.outputs} side="out" />
    </div>
  );
}

function FramePorts({ ports, side }: { ports?: Port[]; side: "in" | "out" }) {
  if (!ports) return null;
  const outer = side === "in" ? Position.Left : Position.Right;
  return (
    <>
      {ports.map((port) => (
        <div key={port.id} className={`frame__port frame__port--${side}`} style={{ top: port.y ?? DEFAULT_PORT_Y }}>
          <Handle type={side === "in" ? "target" : "source"} position={outer} id={`${side}:${port.id}`} className="port__handle port__handle--frame" />
          <Handle
            type={side === "in" ? "source" : "target"}
            position={side === "in" ? Position.Right : Position.Left}
            id={`${side === "in" ? ENTER : LEAVE}:${port.id}`}
            className="port__handle port__handle--inner"
          />
          {port.label ? <span className="frame__port-label mono">{port.label}</span> : null}
        </div>
      ))}
    </>
  );
}
