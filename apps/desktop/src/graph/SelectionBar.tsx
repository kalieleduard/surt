import {
  AlignCenterHorizontal,
  AlignCenterVertical,
  AlignEndHorizontal,
  AlignEndVertical,
  AlignHorizontalDistributeCenter,
  AlignStartHorizontal,
  AlignStartVertical,
  AlignVerticalDistributeCenter,
  Columns3,
  Rows3,
  type LucideIcon,
} from "lucide-react";
import type { Alignment } from "./alignment";

type Action = { alignment: Alignment; icon: LucideIcon; label: string; min?: number };

const GROUPS: Action[][] = [
  [
    { alignment: "row", icon: Columns3, label: "Side by side, evenly spaced" },
    { alignment: "column", icon: Rows3, label: "Stacked, evenly spaced" },
  ],
  [
    { alignment: "left", icon: AlignStartVertical, label: "Align left" },
    { alignment: "center", icon: AlignCenterVertical, label: "Align horizontal centers" },
    { alignment: "right", icon: AlignEndVertical, label: "Align right" },
  ],
  [
    { alignment: "top", icon: AlignStartHorizontal, label: "Align top" },
    { alignment: "middle", icon: AlignCenterHorizontal, label: "Align vertical centers" },
    { alignment: "bottom", icon: AlignEndHorizontal, label: "Align bottom" },
  ],
  [
    { alignment: "distribute-x", icon: AlignHorizontalDistributeCenter, label: "Distribute horizontally", min: 3 },
    { alignment: "distribute-y", icon: AlignVerticalDistributeCenter, label: "Distribute vertically", min: 3 },
  ],
];

export function SelectionBar({ count, onArrange }: { count: number; onArrange: (alignment: Alignment) => void }) {
  return (
    <div className="selection-bar" role="toolbar" aria-label="Arrange selection">
      <span className="selection-bar__count mono">{count} selected</span>
      {GROUPS.map((group, index) => (
        <div key={index} className="selection-bar__group">
          {group.map(({ alignment, icon: Icon, label, min = 2 }) => (
            <button key={alignment} className="icon-btn" onClick={() => onArrange(alignment)} title={label} aria-label={label} disabled={count < min}>
              <Icon size={15} />
            </button>
          ))}
        </div>
      ))}
    </div>
  );
}
