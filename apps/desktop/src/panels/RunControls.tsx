import { useState } from "react";
import { Ban, Play, Square } from "lucide-react";
import { engine } from "../engine/client";
import { useEngine } from "../engine/EngineContext";
import { runControls } from "../engine/runControls";
import type { CancelImpact } from "../engine/types";
import { useAction } from "../engine/useAction";
import { StateDot } from "../ui/status";

export function RunControls() {
  const { selection, subjects, drivers } = useEngine();
  const { busy, error, run } = useAction();
  const [impact, setImpact] = useState<CancelImpact | null>(null);
  const subject = subjects.find((candidate) => candidate.name === selection?.subject);
  const controls = runControls(subject, drivers.find((driver) => driver.subject === subject?.name));
  if (!subject || !(controls.driving || controls.canContinue || controls.canCancel)) return null;
  const name = subject.name;
  const askCancel = async () => {
    const read = await run(() => engine.cancelImpact(name));
    if (read.ok) setImpact(read.value);
  };
  const cancel = async () => {
    await run(() => engine.cancel(name));
    setImpact(null);
  };
  const message = error ?? controls.notice;
  return (
    <div className="canvas-toolbar__group run-controls">
      {controls.driving ? (
        <span className="run-controls__driving caption">
          <StateDot state="running" /> driving
        </span>
      ) : null}
      {controls.canContinue ? (
        <button className="btn btn-primary" disabled={busy} onClick={() => run(() => engine.continueRun(name))}>
          <Play size={12} /> Continue
        </button>
      ) : null}
      {controls.canStop ? (
        <button className="btn btn-ghost" disabled={busy} onClick={() => run(() => engine.stop(name))} title="Stop the Driver; the run stays resumable">
          <Square size={11} /> Stop
        </button>
      ) : null}
      {controls.canCancel ? (
        <button className="icon-btn" disabled={busy} onClick={askCancel} title="Cancel the run">
          <Ban size={15} />
        </button>
      ) : null}
      {impact ? <CancelConfirm subject={name} impact={impact} busy={busy} onCancel={cancel} onKeep={() => setImpact(null)} /> : null}
      {message && !impact ? <p className="run-controls__notice">{message}</p> : null}
    </div>
  );
}

type ConfirmProps = { subject: string; impact: CancelImpact; busy: boolean; onCancel: () => void; onKeep: () => void };

function CancelConfirm({ subject, impact, busy, onCancel, onKeep }: ConfirmProps) {
  const lines = [
    impact.executions ? `stops ${plural(impact.executions, "running Execution")}` : null,
    impact.items ? `ends ${plural(impact.items, "Task")}, ${impact.merged} already merged` : null,
    impact.integrationBranch ? `keeps ${impact.integrationBranch}` : null,
  ].filter(Boolean);
  return (
    <div className="run-controls__confirm" role="dialog" aria-label={`Cancel ${subject}`}>
      <span className="run-controls__confirm-title">Cancel {subject}?</span>
      <span className="run-controls__confirm-text">
        A cancelled run cannot be resumed; only a new run of the Subject starts the work again.
        {lines.length ? ` It ${lines.join("; ")}.` : ""}
      </span>
      <div className="gate-actions__row">
        <button className="btn btn-primary" disabled={busy} onClick={onCancel}>
          Cancel run
        </button>
        <button className="btn btn-ghost" disabled={busy} onClick={onKeep}>
          Keep it
        </button>
      </div>
    </div>
  );
}

function plural(count: number, noun: string) {
  return `${count} ${noun}${count === 1 ? "" : "s"}`;
}
