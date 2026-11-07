import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { ArrowUpRight } from "lucide-react";
import { SurtWordmark } from "../brand/SurtMark";
import { LayoutView } from "../layouts";
import { WindowControls } from "../panels/chrome";
import { ReferenceShot } from "../reference/ReferenceShot";
import { ChoiceSurface } from "./ChoiceContext";
import { AXES, LAYOUTS, PRESETS, type LayoutId } from "./options";

const FRAME_WIDTH = 1440;
const FRAME_HEIGHT = 900;
const MEASURE_DELAY_MS = 600;

type GalleryProps = {
  onOpen: (target: LayoutId | "reference") => void;
};

export function Gallery({ onOpen }: GalleryProps) {
  return (
    <div className="gallery surface" data-surface="obsidiana" data-states="brand">
      <header className="gallery__bar" data-tauri-drag-region>
        <SurtWordmark size={16} />
        <span className="caption" data-tauri-drag-region>Desktop · protótipo 01</span>
        <span className="gallery__spacer" data-tauri-drag-region />
        <WindowControls />
      </header>
      <div className="gallery__scroll">
        <section className="gallery__intro">
          <span className="caption gallery__kicker">Tauri 2 · canvas de agentes</span>
          <h1>
            Escolha a <em>direção</em>.
          </h1>
          <p>
            Quatro layouts sobre o mesmo run real do workflow <span className="mono">feature</span>, mais a referência
            recriada com a pele da marca. Abra uma opção, combine nó, aresta, superfície, estados e fundo no seletor
            do canto inferior esquerdo e aprove.
          </p>
        </section>
        <div className="gallery__grid">
          {LAYOUTS.map((layout) => (
            <GalleryCard
              key={layout.value}
              letter={layout.letter}
              title={layout.label}
              hint={layout.hint}
              tags={AXES.map((axis) => axis.options.find((option) => option.value === PRESETS[layout.value][axis.key])?.label ?? "")}
              onOpen={() => onOpen(layout.value)}
            >
              <ChoiceSurface choice={PRESETS[layout.value]}>
                <LayoutView layout={layout.value} />
              </ChoiceSurface>
            </GalleryCard>
          ))}
          <GalleryCard
            letter="Ref"
            title="Referência"
            hint="img-to-html da imagem anexada, com cores e fonte do Surt."
            tags={["img-to-html", "Obsidiana"]}
            onOpen={() => onOpen("reference")}
          >
            <ChoiceSurface choice={PRESETS.studio}>
              <ReferenceShot />
            </ChoiceSurface>
          </GalleryCard>
        </div>
      </div>
    </div>
  );
}

type GalleryCardProps = {
  letter: string;
  title: string;
  hint: string;
  tags: string[];
  onOpen: () => void;
  children: ReactNode;
};

function GalleryCard({ letter, title, hint, tags, onOpen, children }: GalleryCardProps) {
  return (
    <div
      className="gallery-card"
      role="button"
      tabIndex={0}
      onClick={onOpen}
      onKeyDown={(event) => (event.key === "Enter" ? onOpen() : undefined)}
    >
      <Thumbnail>{children}</Thumbnail>
      <span className="gallery-card__meta">
        <span className="gallery-card__letter">{letter}</span>
        <span className="gallery-card__title">{title}</span>
        <ArrowUpRight size={16} className="gallery-card__go" />
      </span>
      <span className="gallery-card__hint">{hint}</span>
      <span className="gallery-card__tags">
        {tags.map((tag) => (
          <span key={tag} className="chip">{tag}</span>
        ))}
      </span>
    </div>
  );
}

function Thumbnail({ children }: { children: ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);
  const [scale, setScale] = useState(1);
  const [measured, setMeasured] = useState(false);

  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return;
    const observer = new ResizeObserver(() => setScale(element.clientWidth / FRAME_WIDTH));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    const timer = setTimeout(() => setMeasured(true), MEASURE_DELAY_MS);
    return () => clearTimeout(timer);
  }, []);

  return (
    <div ref={ref} className="thumb" style={{ aspectRatio: `${FRAME_WIDTH} / ${FRAME_HEIGHT}` }}>
      <div
        className={`thumb__frame${measured ? " is-ready" : ""}`}
        style={{ width: FRAME_WIDTH, height: FRAME_HEIGHT, transform: measured ? `scale(${scale})` : "none" }}
      >
        {children}
      </div>
    </div>
  );
}
