import { Handle, Position, type NodeProps } from "@xyflow/react";
import type { FrameData, Port } from "../data/types";
import { StateMark, kindIcon } from "../ui/status";

const SIDE_PORT_TOP = 120;
const SIDE_PORT_GAP = 34;
const EDGE_PORT_LEFT = 288;
const EDGE_PORT_GAP = 160;

export function FrameNode({ data }: NodeProps) {
  const frame = data as FrameData;
  const Icon = kindIcon({ kind: frame.kind });
  return (
    <div className={`frame frame--${frame.kind} state-${frame.state}`}>
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

function countBefore(ports: Port[], index: number, onEdge: boolean) {
  return ports.slice(0, index).filter((port) => Boolean(port.edge) === onEdge).length;
}

function placement(port: Port, side: "in" | "out", index: number) {
  if (port.edge === "top") {
    return { position: Position.Top, style: { left: EDGE_PORT_LEFT + index * EDGE_PORT_GAP, top: 0 } };
  }
  if (port.edge === "bottom") {
    return { position: Position.Bottom, style: { left: EDGE_PORT_LEFT + index * EDGE_PORT_GAP, top: "100%" } };
  }
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
        const place = placement(port, side, countBefore(ports, index, Boolean(port.edge)));
        const edgeClass = port.edge ? `frame__port--${port.edge}` : `frame__port--${side}`;
        return (
          <div key={port.id} className={`frame__port ${edgeClass}`} style={place.style}>
            <Handle
              type={side === "in" ? "target" : "source"}
              position={place.position}
              id={`${side}:${port.id}`}
              className="port__handle"
            />
            <span className="frame__port-label">{port.label}</span>
          </div>
        );
      })}
    </>
  );
}
