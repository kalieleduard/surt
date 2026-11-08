import { Handle, Position, type NodeProps } from "@xyflow/react";
import type { FrameData, Port } from "../data/types";
import { StateMark, kindIcon } from "../ui/status";

const SIDE_PORT_TOP = 72;
const SIDE_PORT_GAP = 34;

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

function placement(side: "in" | "out", index: number) {
  return {
    position: side === "in" ? Position.Left : Position.Right,
    style: { top: SIDE_PORT_TOP + index * SIDE_PORT_GAP },
  };
}

function FramePorts({ ports, side }: { ports?: Port[]; side: "in" | "out" }) {
  if (!ports) return null;
  return (
    <>
      {ports.map((port, index) => {
        const place = placement(side, index);
        return (
          <div key={port.id} className={`frame__port frame__port--${side}`} style={place.style}>
            <Handle
              type={side === "in" ? "target" : "source"}
              position={place.position}
              id={`${side}:${port.id}`}
              className="port__handle"
            />
            {port.label ? <span className="frame__port-label">{port.label}</span> : null}
          </div>
        );
      })}
    </>
  );
}
