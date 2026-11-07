import { RunCanvas } from "../graph/RunCanvas";
import { RunViewProvider } from "../graph/RunViewContext";
import { CanvasToolbar } from "../panels/CanvasToolbar";
import { IconRail, TitleBar } from "../panels/chrome";
import { Inspector } from "../panels/Inspector";
import { SourceOverlay } from "../panels/SourceOverlay";
import { Timeline } from "../panels/Timeline";

export function MissionControlLayout() {
  return (
    <RunViewProvider>
      <div className="app">
        <TitleBar />
        <div className="app__body">
          <IconRail />
          <main className="workspace">
            <CanvasToolbar />
            <div className="stage">
              <RunCanvas padding={0.06} />
              <SourceOverlay />
            </div>
            <Timeline />
          </main>
          <Inspector />
        </div>
      </div>
    </RunViewProvider>
  );
}
