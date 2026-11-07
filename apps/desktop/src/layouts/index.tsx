import type { LayoutId } from "../prototype/options";
import { ConversationLayout } from "./ConversationLayout";
import { FocusLayout } from "./Focus";
import { MissionControlLayout } from "./MissionControl";
import { StudioLayout } from "./Studio";

export function LayoutView({ layout }: { layout: LayoutId }) {
  if (layout === "focus") return <FocusLayout />;
  if (layout === "mission") return <MissionControlLayout />;
  if (layout === "conversation") return <ConversationLayout />;
  return <StudioLayout />;
}
