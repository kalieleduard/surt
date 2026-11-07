import { Bot, GitBranch, Layers, MessageCircleQuestionMark, Repeat, Search, ShieldCheck, type LucideIcon } from "lucide-react";
import type { AgentId } from "../data/types";
import { AgentAvatar } from "../ui/status";

const STEP_KINDS: { kind: string; detail: string; icon: LucideIcon }[] = [
  { kind: "agent", detail: "an AI role delivers artifacts", icon: Bot },
  { kind: "approval", detail: "a human decides on a revision", icon: ShieldCheck },
  { kind: "input", detail: "a human answers questions", icon: MessageCircleQuestionMark },
  { kind: "operation", detail: "Surt runs git or checks", icon: GitBranch },
  { kind: "loop", detail: "repeat a body until an outcome", icon: Repeat },
  { kind: "per_item", detail: "one child run per item", icon: Layers },
];

const AGENTS: AgentId[] = ["spec", "tasks", "builder", "reviewer", "shipper"];

export function AddStepPalette({ onClose }: { onClose: () => void }) {
  return (
    <div className="palette">
      <label className="search">
        <Search size={13} />
        <input autoFocus placeholder="Add step" />
      </label>
      <div className="caption palette__group">Steps</div>
      {STEP_KINDS.map(({ kind, detail, icon: Icon }) => (
        <button key={kind} className="palette__item" onClick={onClose}>
          <Icon size={14} />
          <span className="mono">{kind}</span>
          <span className="palette__detail">{detail}</span>
        </button>
      ))}
      <div className="caption palette__group">Agents</div>
      {AGENTS.map((agent) => (
        <button key={agent} className="palette__item" onClick={onClose}>
          <AgentAvatar agent={agent} size={16} />
          <span>{agent}</span>
          <span className="palette__detail mono">.surt/agents/{agent}</span>
        </button>
      ))}
    </div>
  );
}
