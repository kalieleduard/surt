import {
  Box,
  Brush,
  ChevronDown,
  Circle,
  CodeXml,
  GitBranch,
  Image,
  MousePointer2,
  Plus,
  Square,
  type LucideIcon,
} from "lucide-react";
import { ReferenceCanvas } from "./ReferenceCanvas";
import { ReferenceInspector } from "./ReferenceInspector";
import { AddElementPopover, ReferenceFiles, ReferenceRail } from "./ReferenceSidebar";
import "./reference.css";

const MENUS = ["File", "Edit", "View", "Window", "Help"];

const COLLABORATORS: { initial: string; name: string; tone: "lava" | "brasa" | "mineral" }[] = [
  { initial: "D", name: "Designer", tone: "lava" },
  { initial: "T", name: "Tester", tone: "brasa" },
  { initial: "V", name: "Developer", tone: "mineral" },
];

const LEFT_TOOLS: { label: string; icon: LucideIcon }[] = [
  { label: "Select", icon: MousePointer2 },
  { label: "Add", icon: Plus },
  { label: "Image", icon: Image },
  { label: "3D object", icon: Box },
];

const SHAPES: { label: string; icon: LucideIcon; active?: boolean; filled?: boolean }[] = [
  { label: "Cube", icon: Box },
  { label: "Shere", icon: Circle, active: true, filled: true },
  { label: "Plane", icon: Square },
];

function Menubar() {
  return (
    <header className="ref-menubar">
      <span className="ref-lights">
        <span />
        <span />
        <span />
      </span>
      <nav className="ref-menus">
        {MENUS.map((menu) => (
          <span key={menu} className="ref-menu">
            {menu}
          </span>
        ))}
      </nav>
      <div className="ref-menubar-end">
        <span className="ref-avatars">
          {COLLABORATORS.map((person) => (
            <span key={person.name} className="ref-avatar" data-tone={person.tone} title={person.name}>
              {person.initial}
            </span>
          ))}
        </span>
        <button type="button" className="ref-chip">
          Preview
        </button>
        <button type="button" className="ref-chip ref-chip-primary">
          Share
        </button>
      </div>
    </header>
  );
}

function CanvasToolbar() {
  return (
    <div className="ref-toolbar">
      {LEFT_TOOLS.map(({ label, icon: Icon }) => (
        <button type="button" key={label} className="ref-tool" aria-label={label}>
          <Icon size={15} />
        </button>
      ))}
      <span className="ref-crumbs">
        Projects / <b>Index.js</b>
      </span>
      <div className="ref-toolbar-end">
        <button type="button" className="ref-tool" aria-label="Style">
          <Brush size={15} />
        </button>
        <button type="button" className="ref-tool" aria-label="Code">
          <CodeXml size={15} />
        </button>
        <button type="button" className="ref-tool ref-tool-active" aria-label="Logic" aria-pressed="true">
          <GitBranch size={15} />
        </button>
      </div>
    </div>
  );
}

function ShapeMenu() {
  return (
    <div className="ref-shapes" role="menu" aria-label="3D objects">
      {SHAPES.map(({ label, icon: Icon, active, filled }) => (
        <button type="button" role="menuitem" key={label} className="ref-shape" data-active={active}>
          <Icon size={12} fill={filled ? "currentColor" : "none"} />
          {label}
        </button>
      ))}
    </div>
  );
}

function Console() {
  return (
    <footer className="ref-console">
      <ChevronDown size={13} />
      Console
      <span className="ref-console-count">12</span>
    </footer>
  );
}

export function ReferenceShot() {
  return (
    <div className="ref-root">
      <div className="ref-window">
        <div className="ref-frame">
          <Menubar />
          <ReferenceRail />
          <ReferenceFiles />
          <CanvasToolbar />
          <ReferenceCanvas />
          <Console />
          <ReferenceInspector />
        </div>
        <ShapeMenu />
        <AddElementPopover />
      </div>
    </div>
  );
}
