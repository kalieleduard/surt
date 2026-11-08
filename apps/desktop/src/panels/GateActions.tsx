import { useEffect, useState } from "react";
import { Check } from "lucide-react";
import { engine } from "../engine/client";
import { recommendedAnswers, unanswered } from "../engine/forms";
import type { GateSummary, QuestionView, WaitingInput } from "../engine/types";
import { useAction } from "../engine/useAction";

export function GateActions({ subject, gate }: { subject: string; gate: GateSummary }) {
  if (gate.status !== "waiting") return null;
  if (gate.kind === "approval") return <ApprovalActions subject={subject} gate={gate} />;
  return <AnswerForm key={`${subject}:${gate.revision}`} subject={subject} />;
}

function ApprovalActions({ subject, gate }: { subject: string; gate: GateSummary }) {
  const { busy, error, run } = useAction();
  const [rejecting, setRejecting] = useState(false);
  const [reason, setReason] = useState("");
  const reject = async () => {
    const done = await run(() => engine.reject(subject, gate.revision, reason.trim()));
    if (done.ok) setRejecting(false);
  };
  return (
    <div className="gate-actions">
      {rejecting ? (
        <>
          <label className="field">
            <span className="caption">Reason</span>
            <textarea
              rows={4}
              value={reason}
              autoFocus
              placeholder="What the step that produced it must change"
              onChange={(event) => setReason(event.target.value)}
            />
          </label>
          <div className="gate-actions__row">
            <button className="btn btn-primary" disabled={busy || !reason.trim()} onClick={reject}>
              Send rejection
            </button>
            <button className="btn btn-ghost" disabled={busy} onClick={() => setRejecting(false)}>
              Back
            </button>
          </div>
        </>
      ) : (
        <div className="gate-actions__row">
          <button className="btn btn-primary" disabled={busy} onClick={() => run(() => engine.approve(subject, gate.revision))}>
            <Check size={13} /> Approve{gate.revision ? ` r${gate.revision}` : ""}
          </button>
          <button className="btn btn-ghost" disabled={busy} onClick={() => setRejecting(true)}>
            Reject
          </button>
        </div>
      )}
      {error ? <p className="form-error">{error}</p> : null}
    </div>
  );
}

function AnswerForm({ subject }: { subject: string }) {
  const { busy, error, run } = useAction();
  const [waiting, setWaiting] = useState<WaitingInput | null>(null);
  const [readError, setReadError] = useState<string | null>(null);
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [note, setNote] = useState("");

  useEffect(() => {
    let current = true;
    engine
      .waitingInput(subject)
      .then((read) => {
        if (!current) return;
        setWaiting(read);
        setAnswers(recommendedAnswers(read));
      })
      .catch((failure) => current && setReadError(String(failure)));
    return () => {
      current = false;
    };
  }, [subject]);

  if (!waiting) return <p className={readError ? "form-error" : "caption gate-actions__pending"}>{readError ?? "Reading questions"}</p>;
  const missing = unanswered(waiting, answers);
  const send = () => run(() => engine.answer(subject, waiting.revision, answers, note.trim() || null));
  return (
    <div className="gate-actions">
      <span className="caption">
        Round {waiting.round} · {waiting.artifact} r{waiting.revision}
      </span>
      {waiting.questions.map((question) => (
        <QuestionField
          key={question.id}
          question={question}
          value={answers[question.id] ?? ""}
          onChange={(value) => setAnswers((current) => ({ ...current, [question.id]: value }))}
        />
      ))}
      <label className="field">
        <span className="caption">Note · optional</span>
        <textarea rows={2} value={note} onChange={(event) => setNote(event.target.value)} />
      </label>
      <div className="gate-actions__row">
        <button className="btn btn-primary" disabled={busy || missing.length > 0} onClick={send}>
          Send answers
        </button>
      </div>
      {error ? <p className="form-error">{error}</p> : null}
    </div>
  );
}

function QuestionField({ question, value, onChange }: { question: QuestionView; value: string; onChange: (value: string) => void }) {
  return (
    <fieldset className="question">
      <legend className="question__text">{question.question}</legend>
      <p className="question__why">{question.why}</p>
      {question.options ? (
        <div className="question__options">
          {question.options.map((choice) => (
            <label key={choice.value} className={`choice${value === choice.value ? " is-chosen" : ""}`}>
              <input type="radio" name={question.id} checked={value === choice.value} onChange={() => onChange(choice.value)} />
              <span className="choice__value">
                {choice.value}
                {choice.value === question.recommended ? <span className="choice__tag caption">recommended</span> : null}
              </span>
              {choice.description ? <span className="choice__description">{choice.description}</span> : null}
            </label>
          ))}
        </div>
      ) : (
        <textarea rows={2} value={value} onChange={(event) => onChange(event.target.value)} />
      )}
    </fieldset>
  );
}
