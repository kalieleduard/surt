import { useState } from "react";
import { Check, CornerDownLeft, FileText, Terminal } from "lucide-react";
import { APPROVAL, QUESTIONS, SESSION, type SessionEntry } from "../data/inbox";
import { AgentAvatar } from "../ui/status";

type Tab = "session" | "questions" | "approval";

export function Conversation() {
  const [tab, setTab] = useState<Tab>("questions");
  return (
    <aside className="conversation">
      <div className="tabs conversation__tabs">
        <button className={tab === "session" ? "is-active" : undefined} onClick={() => setTab("session")}>Session</button>
        <button className={tab === "questions" ? "is-active" : undefined} onClick={() => setTab("questions")}>
          Questions <span className="badge">3</span>
        </button>
        <button className={tab === "approval" ? "is-active" : undefined} onClick={() => setTab("approval")}>
          Approval <span className="badge">1</span>
        </button>
      </div>
      {tab === "session" ? <SessionView /> : null}
      {tab === "questions" ? <QuestionsView /> : null}
      {tab === "approval" ? <ApprovalView /> : null}
    </aside>
  );
}

function SessionView() {
  return (
    <div className="conversation__body">
      <div className="conversation__meta">
        <AgentAvatar agent="builder" size={22} />
        <span>Builder · task B · iteration 2 · attempt 2</span>
        <span className="mono conversation__session">session 0194f1c2</span>
      </div>
      {SESSION.map((entry, index) => (
        <SessionLine key={index} entry={entry} />
      ))}
      <div className="session-line session-line--typing">
        <span className="terminal__caret">▌</span>
      </div>
    </div>
  );
}

function SessionLine({ entry }: { entry: SessionEntry }) {
  if (entry.kind === "context") return <div className="session-line session-line--context mono">{entry.text}</div>;
  if (entry.kind === "agent") return <div className="session-line session-line--agent">{entry.text}</div>;
  return (
    <div className="session-line session-line--tool mono">
      <Terminal size={12} />
      <span className="session-line__tool">{entry.tool}</span>
      <span>{entry.target}</span>
      {entry.result ? <span className="session-line__result">{entry.result}</span> : null}
    </div>
  );
}

function QuestionsView() {
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const answered = Object.keys(answers).length;
  return (
    <div className="conversation__body">
      <div className="conversation__meta">
        <AgentAvatar agent="spec" size={22} />
        <span>{QUESTIONS.subject}</span>
        <span className="mono conversation__session">{QUESTIONS.step} · round {QUESTIONS.round}</span>
      </div>
      {QUESTIONS.items.map((question, index) => (
        <fieldset key={question.id} className="question">
          <legend>
            <span className="caption">Q{index + 1}</span> {question.text}
          </legend>
          {question.choices.map((choice) => (
            <button
              key={choice.label}
              className={`choice${answers[question.id] === choice.label ? " is-picked" : ""}`}
              onClick={() => setAnswers({ ...answers, [question.id]: choice.label })}
            >
              <span className="choice__radio" />
              <span className="choice__text">
                <span>{choice.label}</span>
                <small>{choice.detail}</small>
              </span>
              {choice.recommended ? <span className="choice__rec caption">recommended</span> : null}
            </button>
          ))}
        </fieldset>
      ))}
      <footer className="conversation__footer">
        <span className="mono">{answered}/{QUESTIONS.items.length} answered</span>
        <button className="btn btn-primary" disabled={answered < QUESTIONS.items.length}>
          Send answers <CornerDownLeft size={13} />
        </button>
      </footer>
    </div>
  );
}

function ApprovalView() {
  return (
    <div className="conversation__body">
      <div className="conversation__meta">
        <FileText size={16} />
        <span>{APPROVAL.subject}</span>
        <span className="mono conversation__session">{APPROVAL.artifact} {APPROVAL.revision}</span>
      </div>
      <div className="manifest">
        {APPROVAL.tasks.map((task) => (
          <div key={task.key} className="manifest__row">
            <span className="node__key">{task.key}</span>
            <span className="manifest__title">{task.title}</span>
            <span className="mono manifest__deps">{task.deps.length ? `needs ${task.deps.join(", ")}` : "root"}</span>
          </div>
        ))}
      </div>
      <textarea className="reason" placeholder="Reason, if you reject: it becomes manifest-rejection-reason r1" />
      <footer className="conversation__footer">
        <button className="btn btn-ghost">Reject</button>
        <button className="btn btn-primary">
          <Check size={14} /> Approve {APPROVAL.revision}
        </button>
      </footer>
    </div>
  );
}
