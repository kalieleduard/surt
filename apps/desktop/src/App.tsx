import { useState } from "react";
import { LayoutView } from "./layouts";
import { ChoiceSurface } from "./prototype/ChoiceContext";
import { Gallery } from "./prototype/Gallery";
import { PRESETS, type Choice, type LayoutId } from "./prototype/options";
import { PrototypeDock } from "./prototype/PrototypeDock";
import { ReferenceShot } from "./reference/ReferenceShot";

type Screen = "gallery" | "reference" | "layout";

export function App() {
  const [screen, setScreen] = useState<Screen>("gallery");
  const [choice, setChoice] = useState<Choice>(PRESETS.studio);

  if (screen === "gallery") {
    return (
      <Gallery
        onOpen={(target) => {
          if (target === "reference") {
            setScreen("reference");
            return;
          }
          setChoice(PRESETS[target]);
          setScreen("layout");
        }}
      />
    );
  }

  const openLayout = (layout: LayoutId) => {
    if (layout !== choice.layout) setChoice(PRESETS[layout]);
    setScreen("layout");
  };

  return (
    <ChoiceSurface choice={choice} className="screen">
      {screen === "reference" ? <ReferenceShot /> : <LayoutView key={choice.layout} layout={choice.layout} />}
      <PrototypeDock
        choice={choice}
        onChange={setChoice}
        onLayout={openLayout}
        onGallery={() => setScreen("gallery")}
        onReference={() => setScreen("reference")}
        referenceActive={screen === "reference"}
      />
    </ChoiceSurface>
  );
}
