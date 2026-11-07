import { useState } from "react";
import { Check, X } from "lucide-react";
import { approveChoice } from "./approve";
import { describeChoice, type Choice } from "./options";

type ApproveDialogProps = {
  choice: Choice;
  onClose: () => void;
};

export function ApproveDialog({ choice, onClose }: ApproveDialogProps) {
  const [notes, setNotes] = useState("");
  const [result, setResult] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const confirm = async () => {
    try {
      setResult(await approveChoice(choice, notes));
      setError(null);
    } catch (failure) {
      setError(String(failure));
    }
  };

  return (
    <div className="dialog-scrim" data-surface="obsidiana" data-states="brand" onClick={onClose}>
      <div className="dialog" onClick={(event) => event.stopPropagation()}>
        <header className="dialog__head">
          <span>Aprovar esta combinação</span>
          <button className="icon-btn" onClick={onClose} aria-label="Fechar">
            <X size={14} />
          </button>
        </header>
        <ul className="dialog__summary">
          {describeChoice(choice).map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
        <textarea
          className="reason"
          placeholder="Notas: o que manter, o que mudar, o que misturar de outra opção"
          value={notes}
          onChange={(event) => setNotes(event.target.value)}
        />
        {result ? <p className="dialog__result">{result}</p> : null}
        {error ? <p className="dialog__error">{error}</p> : null}
        <footer className="dialog__foot">
          <button className="btn btn-ghost" onClick={onClose}>{result ? "Fechar" : "Cancelar"}</button>
          {result ? null : (
            <button className="btn btn-primary" onClick={confirm}>
              <Check size={14} /> Aprovar
            </button>
          )}
        </footer>
      </div>
    </div>
  );
}
