import { Plus } from "lucide-react";
import { RunCanvas } from "../graph/RunCanvas";
import { RunViewProvider } from "../graph/RunViewContext";
import { CanvasToolbar } from "../panels/CanvasToolbar";
import { Console } from "../panels/Console";
import { IconRail, TitleBar } from "../panels/chrome";
import { Inspector } from "../panels/Inspector";
import { RunCard } from "../panels/RunCard";
import { RunTree } from "../panels/RunTree";
import { SourceOverlay } from "../panels/SourceOverlay";

const STUDIO_PADDING = { top: "40px", right: "28px", bottom: "40px", left: "292px" } as const;
const STUDIO_FOCUS = ["implement", "cycle"];

export function StudioLayout() {
  return (
    <RunViewProvider>
      <div className="app">
        <TitleBar />
        <div className="app__body">
          <IconRail />
          <RunTree />
          <main className="workspace">
            <CanvasToolbar />
            <div className="stage">
              <RunCanvas padding={STUDIO_PADDING} focus={STUDIO_FOCUS} />
              <RunCard />
              <SourceOverlay />
              <button className="fab" aria-label="Add step">
                <Plus size={22} />
              </button>
            </div>
            <Console initiallyOpen={false} />
          </main>
          <Inspector />
        </div>
      </div>
    </RunViewProvider>
  );
}
