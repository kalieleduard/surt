import {
  Bot,
  FlaskConical,
  GitBranch,
  GitMerge,
  Layers,
  MessageCircleQuestionMark,
  Repeat,
  ShieldCheck,
  SquareDot,
  type LucideIcon,
} from "lucide-react";
import type { StepData, StepKind, StepState } from "../data/types";

export const STATE_SYMBOL: Record<StepState, string> = {
  running: "●",
  waiting: "⏳",
  succeeded: "✔",
  blocked: "⛔",
  pending: "○",
  cancelled: "⏹",
  skipped: "−",
  interrupted: "⏸",
  failed: "✖",
};

export const STATE_LABEL: Record<StepState, string> = {
  running: "running",
  waiting: "waiting on you",
  succeeded: "succeeded",
  blocked: "blocked",
  pending: "pending",
  cancelled: "cancelled",
  skipped: "skipped",
  interrupted: "interrupted",
  failed: "failed",
};

const KIND_ICON: Record<StepKind, LucideIcon> = {
  agent: Bot,
  approval: ShieldCheck,
  input: MessageCircleQuestionMark,
  operation: GitBranch,
  loop: Repeat,
  per_item: Layers,
  item: SquareDot,
};

export function kindIcon(data: Pick<StepData, "kind" | "operation">): LucideIcon {
  if (data.operation === "checks.run") return FlaskConical;
  if (data.operation?.includes("merge") || data.operation === "git.integrate") return GitMerge;
  return KIND_ICON[data.kind];
}

export function kindLabel(data: StepData) {
  if (data.kind === "operation") return data.operation ?? "operation";
  if (data.kind === "approval") return "approval gate";
  if (data.kind === "input") return "input step";
  if (data.kind === "item") return data.summary ?? "child run";
  return data.agent ?? data.kind;
}

export function StateMark({ state }: { state: StepState }) {
  return (
    <span className={`state-mark state-${state}`} title={STATE_LABEL[state]}>
      {STATE_SYMBOL[state]}
    </span>
  );
}

export function StateDot({ state }: { state: StepState }) {
  return <span className={`state-dot state-${state}`} title={STATE_LABEL[state]} />;
}

export function AgentAvatar({ agent, size = 20 }: { agent: string; size?: number }) {
  return (
    <span className={`agent-avatar agent-${agent}`} style={{ width: size, height: size, fontSize: size * 0.5 }}>
      {agent.slice(0, 1).toUpperCase()}
    </span>
  );
}
