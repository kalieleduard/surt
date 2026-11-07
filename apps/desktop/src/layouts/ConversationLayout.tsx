import { RunCanvas } from "../graph/RunCanvas";
import { RunViewProvider } from "../graph/RunViewContext";
import { CanvasToolbar } from "../panels/CanvasToolbar";
import { Console } from "../panels/Console";
import { Conversation } from "../panels/Conversation";
import { IconRail, TitleBar } from "../panels/chrome";
import { SourceOverlay } from "../panels/SourceOverlay";

export function ConversationLayout() {
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
            <Console initiallyOpen={false} />
          </main>
          <Conversation />
        </div>
      </div>
    </RunViewProvider>
  );
}
