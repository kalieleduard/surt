import { useState } from "react";
import { LayoutGrid, RotateCcw, ThumbsUp, X } from "lucide-react";
import { SurtSymbol } from "../brand/SurtMark";
import { ApproveDialog } from "./ApproveDialog";
import { AXES, LAYOUTS, PRESETS, layoutOf, type AxisKey, type Choice, type LayoutId } from "./options";

type PrototypeDockProps = {
  choice: Choice;
  onChange: (choice: Choice) => void;
  onLayout: (layout: LayoutId) => void;
  onGallery: () => void;
  onReference: () => void;
  referenceActive: boolean;
};

const REFERENCE_AXES: AxisKey[] = ["surface"];

export function PrototypeDock({ choice, onChange, onLayout, onGallery, onReference, referenceActive }: PrototypeDockProps) {
  const [open, setOpen] = useState(false);
  const [approving, setApproving] = useState(false);
  const layout = layoutOf(choice.layout);

  if (!open) {
    return (
      <button className="proto-trigger" onClick={() => setOpen(true)} title="Opções do protótipo">
        <SurtSymbol size={16} />
        <span>{referenceActive ? "Ref" : layout.letter}</span>
      </button>
    );
  }

  const axes = referenceActive ? AXES.filter((axis) => REFERENCE_AXES.includes(axis.key)) : AXES;

  return (
    <>
      <div className="proto-dock" data-surface="obsidiana" data-states="brand">
        <header className="proto-dock__head">
          <SurtSymbol size={16} />
          <span>Protótipo · escolha e aprove</span>
          <button className="icon-btn" onClick={() => setOpen(false)} aria-label="Fechar opções">
            <X size={14} />
          </button>
        </header>
        <div className="proto-axis">
          <span className="caption">Layout</span>
          <div className="proto-segments">
            {LAYOUTS.map((item) => (
              <button
                key={item.value}
                title={item.hint}
                className={!referenceActive && choice.layout === item.value ? "is-active" : undefined}
                onClick={() => onLayout(item.value)}
              >
                <b>{item.letter}</b> {item.label}
              </button>
            ))}
            <button className={referenceActive ? "is-active" : undefined} onClick={onReference} title="img-to-html da imagem de referência">
              Ref
            </button>
          </div>
        </div>
        {axes.map((axis) => (
          <div key={axis.key} className="proto-axis">
            <span className="caption">{axis.label}</span>
            <div className="proto-segments">
              {axis.options.map((option) => (
                <button
                  key={option.value}
                  title={option.hint}
                  className={choice[axis.key] === option.value ? "is-active" : undefined}
                  onClick={() => onChange({ ...choice, [axis.key]: option.value })}
                >
                  {option.label}
                </button>
              ))}
            </div>
          </div>
        ))}
        <p className="proto-dock__hint">{referenceActive ? "Referência recriada com a pele da marca." : layout.hint}</p>
        <footer className="proto-dock__foot">
          <button className="btn btn-ghost" onClick={onGallery}>
            <LayoutGrid size={13} /> Galeria
          </button>
          <button className="btn btn-ghost" onClick={() => onChange(PRESETS[choice.layout])} title="Voltar ao preset do layout">
            <RotateCcw size={13} />
          </button>
          <button className="btn btn-primary" onClick={() => setApproving(true)}>
            <ThumbsUp size={13} /> Aprovar esta combinação
          </button>
        </footer>
      </div>
      {approving ? <ApproveDialog choice={choice} onClose={() => setApproving(false)} /> : null}
    </>
  );
}
