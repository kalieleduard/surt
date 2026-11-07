import { Diamond, MousePointer2, Plus, SquarePen } from "lucide-react";
import { ReferenceCodeNode } from "./ReferenceCodeNode";
import { ReferenceEdges } from "./ReferenceEdges";
import { ReferencePhone } from "./ReferencePhone";
import { NODES, type GraphNode, type NodeRow } from "./referenceGraph";
import "./reference-canvas.css";

type CursorTone = "lava" | "brasa" | "mineral";

const CURSORS: { name: string; tone: CursorTone; x: number; y: number }[] = [
  { name: "Designer", tone: "lava", x: 718, y: 160 },
  { name: "Tester", tone: "brasa", x: 159, y: 367 },
  { name: "Developer", tone: "mineral", x: 519, y: 567 },
];

function rowKey(row: NodeRow) {
  return `${row.left ?? ""}:${row.right ?? ""}`;
}

function NodeCard({ node }: { node: GraphNode }) {
  return (
    <div className="ref-node" style={{ left: node.x, top: node.y, width: node.width }}>
      <div className="ref-node-head">
        {node.marked && <Diamond size={10} fill="currentColor" />}
        <span className="ref-node-title">{node.title}</span>
        <SquarePen size={12} className="ref-node-edit" />
      </div>
      {node.rows.map((row) => (
        <div key={rowKey(row)} className="ref-node-row" data-lead={row.lead} data-strong={row.strong}>
          <span className="ref-node-label">{row.left}</span>
          <span className="ref-node-value">{row.right}</span>
        </div>
      ))}
    </div>
  );
}

function Cursor({ name, tone, x, y }: (typeof CURSORS)[number]) {
  return (
    <div className="ref-cursor" data-tone={tone} style={{ left: x, top: y }}>
      <MousePointer2 size={16} />
      <span className="ref-cursor-tag">{name}</span>
    </div>
  );
}

export function ReferenceCanvas() {
  return (
    <section className="ref-canvas" aria-label="Canvas">
      <ReferenceCodeNode />
      {Object.entries(NODES).map(([name, node]) => (
        <NodeCard key={name} node={node} />
      ))}
      <ReferenceEdges />
      <ReferencePhone />
      {CURSORS.map((cursor) => (
        <Cursor key={cursor.name} {...cursor} />
      ))}
      <div className="ref-mode" role="group" aria-label="Mode">
        <button type="button" aria-pressed="false">
          Default
        </button>
        <button type="button" aria-pressed="true">
          Debug
        </button>
      </div>
      <button type="button" className="ref-fab" aria-label="Add node">
        <Plus size={22} strokeWidth={2.2} />
      </button>
    </section>
  );
}
