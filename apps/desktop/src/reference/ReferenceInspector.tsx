import { Diamond, List, Plus, type LucideIcon } from "lucide-react";
import "./reference-inspector.css";

type Swatch = "dot" | "square";

const PROPS: { caption: string; value: string }[] = [
  { caption: "class", value: "btn primary btn-lg" },
  { caption: "ID", value: "$myID" },
];

const STYLES: { caption: string; value: string; swatch?: Swatch }[] = [
  { caption: "position", value: "relative" },
  { caption: "background-color", value: "--dark-purple", swatch: "dot" },
  { caption: "background", value: "linear-gradient(blue, pink)", swatch: "square" },
  { caption: "top", value: "auto" },
  { caption: "bottom", value: "auto" },
  { caption: "left", value: "0" },
  { caption: "margin", value: "0 auto" },
];

function SectionHead({ title, icon: Icon = Plus }: { title: string; icon?: LucideIcon }) {
  return (
    <div className="ref-insp-section">
      <span>{title}</span>
      <button type="button" aria-label={`Add to ${title}`}>
        <Icon size={13} />
      </button>
    </div>
  );
}

function Field({ caption, value, swatch }: { caption: string; value: string; swatch?: Swatch }) {
  return (
    <>
      <div className="ref-insp-caption">{caption}</div>
      <div className="ref-insp-value">
        {swatch && <span className="ref-swatch" data-shape={swatch} />}
        {value}
      </div>
    </>
  );
}

export function ReferenceInspector() {
  return (
    <aside className="ref-inspector">
      <header className="ref-insp-title">
        <Diamond size={10} fill="currentColor" />
        Button
      </header>
      <div className="ref-insp-body">
        <SectionHead title="Events" />
        <div className="ref-insp-item">
          <span>OnClick</span>
          <span className="ref-insp-dim">AddClass()</span>
        </div>
        <SectionHead title="States" />
        <div className="ref-insp-item" data-selected="true">
          Default
        </div>
        <div className="ref-insp-item">Hover</div>
        <SectionHead title="Props(Args)" />
        {PROPS.map((prop) => (
          <Field key={prop.caption} {...prop} />
        ))}
        <SectionHead title="Styles" icon={List} />
        <div className="ref-insp-add">
          <input placeholder="add style" aria-label="Add style" />
        </div>
        {STYLES.map((style) => (
          <Field key={style.caption} {...style} />
        ))}
      </div>
    </aside>
  );
}
