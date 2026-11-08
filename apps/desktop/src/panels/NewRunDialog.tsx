import { useEffect, useState } from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { engine } from "../engine/client";
import { useCheckout } from "../checkout/CheckoutContext";
import { useEngine } from "../engine/EngineContext";
import { missingInputs, runInputValues } from "../engine/forms";
import type { RunInput, WorkflowView } from "../engine/types";
import { useAction } from "../engine/useAction";

export function NewRunDialog({ onClose }: { onClose: () => void }) {
  const { show } = useEngine();
  const { busy, error, run } = useAction();
  const [workflows, setWorkflows] = useState<WorkflowView[] | null>(null);
  const [readError, setReadError] = useState<string | null>(null);
  const [chosen, setChosen] = useState<string | null>(null);
  const [values, setValues] = useState<Record<string, string>>({});
  const [name, setName] = useState("");

  useEffect(() => {
    engine
      .workflowDefinitions()
      .then((read) => {
        setWorkflows(read);
        setChosen(read.find((workflow) => workflow.runInputs)?.name ?? null);
      })
      .catch((failure) => setReadError(String(failure)));
  }, []);

  const { baseRef, selected } = useCheckout();
  const workflow = workflows?.find((candidate) => candidate.name === chosen);
  const inputs = workflow?.runInputs ?? [];

  useEffect(() => {
    if (!baseRef) return;
    const refs = inputs.filter((input) => input.format === "git_ref");
    if (refs.length) setValues((current) => ({ ...Object.fromEntries(refs.map((input) => [input.name, baseRef])), ...current }));
  }, [workflow?.name, baseRef]);

  const missing = missingInputs(inputs, values);
  const start = async () => {
    if (!workflow) return;
    const started = await run(() => engine.newRun(workflow.name, runInputValues(inputs, values), name.trim() || null));
    if (!started.ok) return;
    show(started.value, null);
    onClose();
  };

  return createPortal(
    <div className="dialog-backdrop" onPointerDown={(event) => event.target === event.currentTarget && onClose()}>
      <div className="dialog" role="dialog" aria-label="New run">
        <header className="dialog__head">
          <span className="dialog__title">New run</span>
          <button className="icon-btn" onClick={onClose} aria-label="Close">
            <X size={14} />
          </button>
        </header>
        <div className="dialog__body">
          {readError ? <p className="form-error">{readError}</p> : null}
          {workflows && !workflows.length ? <p className="dialog__hint">This project has no Workflow Definitions under .surt/workflows.</p> : null}
          {workflows?.length ? (
            <label className="field">
              <span className="caption">Workflow</span>
              <select value={chosen ?? ""} onChange={(event) => setChosen(event.target.value)}>
                {workflows.map((candidate) => (
                  <option key={candidate.name} value={candidate.name}>
                    {candidate.name}
                    {candidate.problem ? " · invalid" : ""}
                  </option>
                ))}
              </select>
            </label>
          ) : null}
          {workflow?.problem ? <p className="form-error">{workflow.problem}</p> : null}
          {inputs.map((input) => (
            <InputField
              key={`${workflow?.name}:${input.name}`}
              input={input}
              origin={input.format === "git_ref" && selected && values[input.name] === baseRef ? `from ${selected.main ? "the main worktree" : selected.name}` : null}
              value={values[input.name] ?? ""}
              onChange={(value) => setValues((current) => ({ ...current, [input.name]: value }))}
            />
          ))}
          {workflow?.runInputs ? (
            <label className="field">
              <span className="caption">Subject name · optional</span>
              <input value={name} placeholder="numbered by date when empty" onChange={(event) => setName(event.target.value)} />
            </label>
          ) : null}
          {error ? <p className="form-error">{error}</p> : null}
        </div>
        <footer className="dialog__foot">
          <button className="btn btn-ghost" onClick={onClose}>
            Close
          </button>
          <button className="btn btn-primary" disabled={busy || !workflow?.runInputs || missing.length > 0} onClick={start}>
            Start run
          </button>
        </footer>
      </div>
    </div>,
    document.querySelector<HTMLElement>(".surface") ?? document.body,
  );
}

function InputField({ input, origin, value, onChange }: { input: RunInput; origin: string | null; value: string; onChange: (value: string) => void }) {
  const label = `${input.name}${input.optional ? " · optional" : ""}`;
  const placeholder = input.default ? `default ${input.default}` : undefined;
  return (
    <label className="field">
      <span className="caption">{label}</span>
      {input.format === "git_ref" ? (
        <input className="mono" value={value} placeholder={placeholder} onChange={(event) => onChange(event.target.value)} />
      ) : (
        <textarea
          className={input.format === "json" ? "mono" : undefined}
          rows={input.format === "text" ? 3 : 6}
          value={value}
          placeholder={placeholder}
          onChange={(event) => onChange(event.target.value)}
        />
      )}
      {input.description || origin ? <span className="field__hint">{[input.description, origin].filter(Boolean).join(" · ")}</span> : null}
    </label>
  );
}
