import { useEffect } from "react";
import { Handle, NodeResizer, Position, useStore, useUpdateNodeInternals, type InternalNode, type Node, type NodeProps } from "@xyflow/react";
import type { FrameData, Port } from "../data/types";
import { ENTER, LEAVE } from "../engine/runEdges";
import { StateMark, kindIcon } from "../ui/status";
import { useShiftKey } from "./useShiftKey";

const DEFAULT_PORT_Y = 68;
const FRAME_MIN = { width: 240, height: 110 };

export function FrameNode({ id, data, selected }: NodeProps) {
  const frame = data as FrameData;
  const shift = useShiftKey();
  const Icon = kindIcon({ kind: frame.kind });
  const inY = useFollowedY(frame.entry, "target", "in:in");
  const outY = useFollowedY(frame.exit, "source", "out:out");
  const updateInternals = useUpdateNodeInternals();
  useEffect(() => updateInternals(id), [id, inY, outY, updateInternals]);
  return (
    <>
      <NodeResizer
        isVisible={selected}
        minWidth={FRAME_MIN.width}
        minHeight={FRAME_MIN.height}
        keepAspectRatio={shift}
        lineClassName="resize-line"
        handleClassName="resize-handle"
      />
      <div className={`frame frame--${frame.kind} state-${frame.state}${selected ? " is-selected" : ""}`}>
        <span className="frame__shape" />
        <header className="frame__head">
          <Icon size={14} className="frame__icon" />
          <span className="frame__name">{frame.name}</span>
          <span className="frame__caption">{frame.caption}</span>
          <span className="frame__detail mono" title={frame.detail}>
            {frame.detail}
          </span>
          <StateMark state={frame.state} />
        </header>
        <FramePorts ports={frame.inputs} side="in" followed={inY} />
        <FramePorts ports={frame.outputs} side="out" followed={outY} />
      </div>
    </>
  );
}

function useFollowedY(member: string | undefined, type: "source" | "target", handle: string): number | null {
  return useStore((store) => (member ? handleCenter(store.nodeLookup.get(member), type, handle) : null));
}

function handleCenter(node: InternalNode<Node> | undefined, type: "source" | "target", id: string): number | null {
  const bounds = node?.internals.handleBounds?.[type]?.find((handle) => handle.id === id);
  if (!node || !bounds) return null;
  return node.position.y + bounds.y + bounds.height / 2;
}

function FramePorts({ ports, side, followed }: { ports?: Port[]; side: "in" | "out"; followed: number | null }) {
  if (!ports) return null;
  const outer = side === "in" ? Position.Left : Position.Right;
  return (
    <>
      {ports.map((port, index) => (
        <div key={port.id} className={`frame__port frame__port--${side}`} style={{ top: (index === 0 ? followed : null) ?? port.y ?? DEFAULT_PORT_Y }}>
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
