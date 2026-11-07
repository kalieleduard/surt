import {
  ChartLine,
  CodeXml,
  Diamond,
  Download,
  DraftingCompass,
  Eye,
  File as FileIcon,
  Folder,
  Plus,
  Rocket,
  Upload,
  type LucideIcon,
} from "lucide-react";
import { SurtSymbol } from "../brand/SurtMark";
import "./reference-sidebar.css";

type TreeIcon = "folder" | "file" | "code" | "component" | "instance" | "text";

type TreeRow = {
  label: string;
  depth: 0 | 1 | 2;
  icon: TreeIcon;
  open?: boolean;
  dirty?: boolean;
  selected?: boolean;
  pinned?: boolean;
};

const RAIL: { label: string; icon: LucideIcon; active?: boolean }[] = [
  { label: "Set up", icon: DraftingCompass },
  { label: "Explore", icon: Folder, active: true },
  { label: "Deploy", icon: Rocket },
  { label: "Statistics", icon: ChartLine },
];

const TREE: TreeRow[] = [
  { label: "components", depth: 0, icon: "folder" },
  { label: "grid.js", depth: 1, icon: "component", dirty: true },
  { label: "button.js", depth: 1, icon: "component" },
  { label: "index.js", depth: 0, icon: "file", open: true, selected: true },
  { label: "container", depth: 1, icon: "code" },
  { label: "container", depth: 1, icon: "code", open: true },
  { label: "grid", depth: 2, icon: "instance", pinned: true },
  { label: "grid", depth: 2, icon: "instance" },
  { label: "Text", depth: 2, icon: "text", dirty: true },
  { label: "container", depth: 1, icon: "code" },
  { label: "container", depth: 1, icon: "code" },
  { label: "container", depth: 1, icon: "code" },
  { label: "person.js", depth: 0, icon: "file", open: true },
  { label: "container", depth: 1, icon: "code" },
  { label: "components", depth: 0, icon: "folder" },
  { label: "person.js", depth: 0, icon: "file", open: true },
  { label: "person.js", depth: 0, icon: "file", open: true },
];

const ELEMENTS: { title: string; kind: "element" | "component"; items: string[] }[] = [
  { title: "Elements", kind: "element", items: ["Div", "Span"] },
  { title: "Components", kind: "component", items: ["Tabbar", "Tab"] },
];

function TreeGlyph({ icon }: { icon: TreeIcon }) {
  if (icon === "folder") return <Folder size={13} fill="currentColor" className="ref-tree-icon" />;
  if (icon === "file") return <FileIcon size={13} fill="currentColor" className="ref-tree-icon" />;
  if (icon === "code") return <CodeXml size={13} className="ref-tree-icon" />;
  if (icon === "text") return <span className="ref-tree-icon ref-tree-letter">A</span>;
  return <Diamond size={11} fill="currentColor" className="ref-tree-icon" data-kind={icon} />;
}

export function ReferenceRail() {
  return (
    <nav className="ref-rail" aria-label="Sections">
      <div className="ref-rail-logo">
        <SurtSymbol size={18} />
      </div>
      {RAIL.map(({ label, icon: Icon, active }) => (
        <button type="button" key={label} className="ref-rail-item" aria-current={active ? "page" : undefined}>
          <Icon size={17} />
          {label}
        </button>
      ))}
    </nav>
  );
}

export function ReferenceFiles() {
  return (
    <aside className="ref-files">
      <div className="ref-tabs" role="tablist">
        <button type="button" role="tab" className="ref-tab" aria-selected="true">
          Files
        </button>
        <button type="button" role="tab" className="ref-tab" aria-selected="false">
          Libraries
        </button>
      </div>
      <input className="ref-search" placeholder="Search" aria-label="Search files" />
      <div className="ref-files-head">
        <span className="ref-caret" data-open="true" />
        Files
        <span className="ref-files-tools">
          <Download size={13} />
          <Upload size={13} />
          <Plus size={13} />
        </span>
      </div>
      <div className="ref-tree" role="tree">
        {TREE.map((row, index) => (
          <div
            key={index}
            role="treeitem"
            aria-level={row.depth + 1}
            aria-selected={row.selected ?? false}
            className="ref-tree-row"
            data-depth={row.depth}
            data-pinned={row.pinned}
          >
            <span className="ref-caret" data-open={row.open ?? false} />
            <TreeGlyph icon={row.icon} />
            <span className="ref-tree-label">{row.label}</span>
            {row.dirty && <span className="ref-dirty" />}
            <span className="ref-tree-actions">
              <Eye size={13} />
              <Plus size={13} />
            </span>
          </div>
        ))}
      </div>
    </aside>
  );
}

export function AddElementPopover() {
  return (
    <div className="ref-add" role="dialog" aria-label="Add element">
      <input className="ref-add-input" placeholder="Add element" aria-label="Add element" />
      {ELEMENTS.map((group) => (
        <div key={group.title} className="ref-add-group">
          <div className="ref-add-title">{group.title}</div>
          {group.items.map((item) => (
            <button type="button" key={item} className="ref-add-item" data-kind={group.kind} data-active={item === "Div"}>
              <Diamond size={10} fill="currentColor" />
              {item}
            </button>
          ))}
        </div>
      ))}
    </div>
  );
}
