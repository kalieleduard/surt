import { useEffect, useState } from "react";
import { AlertTriangle, ExternalLink, Pencil, Save } from "lucide-react";
import { CodeBlock, CodeEditor } from "../code/Code";
import { engine } from "../engine/client";
import { useEngine } from "../engine/EngineContext";
import { replaceStepBlock, stepBlock } from "../engine/stepBlock";
import { parseToml } from "../engine/toml";
import { useDefinitions } from "../editor/DefinitionsContext";
import { DefSection } from "./DefSection";

type Props = { stepKey: string; workflow: string; file: string; live: string | null; frozen: string; run: number | null };

export function StepSource({ stepKey, workflow, file, live, frozen, run }: Props) {
  const { available } = useEngine();
  const { edit, setCatalog } = useDefinitions();
  const frozenBlock = stepBlock(frozen, stepKey)?.block ?? null;
  const liveBlock = live === null ? null : (stepBlock(live, stepKey)?.block ?? null);
  const shown = liveBlock ?? frozenBlock;
  const [draft, setDraft] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const changed = run !== null && liveBlock !== null && frozenBlock !== null && liveBlock !== frozenBlock;

  useEffect(() => {
    setDraft(null);
    setError(null);
  }, [stepKey]);

  const save = async () => {
    if (draft === null || live === null || saving) return;
    const next = replaceStepBlock(live, stepKey, draft);
    try {
      parseToml(next);
    } catch (failure) {
      setError(`Not valid TOML: ${String(failure).replace(/^Error: /, "")}`);
      return;
    }
    setSaving(true);
    setError(null);
    try {
      setCatalog(await engine.saveDefinition(file, next));
      setDraft(null);
    } catch (failure) {
      setError(String(failure));
    } finally {
      setSaving(false);
    }
  };

  const actions =
    available && live !== null && draft === null ? (
      <span className="def-section__tools">
        <button className="icon-btn icon-btn--small" onClick={() => setDraft(shown ?? `[steps.${stepKey}]\n`)} title="Edit this step" aria-label="Edit this step">
          <Pencil size={12} />
        </button>
        <button className="icon-btn icon-btn--small" onClick={() => edit({ kind: "workflow", name: workflow })} title={`Open ${file}`} aria-label={`Open ${file}`}>
          <ExternalLink size={12} />
        </button>
      </span>
    ) : null;

  return (
    <DefSection title="Definition" aside={actions}>
      <div className="def-source__path mono">
        .surt/{file}
        {changed ? <span className="def-source__badge">changed since run {run}</span> : null}
      </div>
      {draft === null ? (
        shown ? (
          <CodeBlock text={shown} language="toml" className="def-source__code" />
        ) : (
          <p className="def__empty">This step is not in the workflow file.</p>
        )
      ) : (
        <div className="def-source__edit">
          <CodeEditor value={draft} language="toml" onChange={setDraft} onSave={save} gutter={false} autoFocus className="def-source__editor" />
          <p className="def-source__hint">Edits change what new runs start from.{run !== null ? ` Run ${run} stays frozen.` : ""}</p>
          {error ? (
            <p className="def-source__error">
              <AlertTriangle size={12} /> {error}
            </p>
          ) : null}
          <div className="def-actions">
            <button className="btn btn-ghost" onClick={() => setDraft(null)} disabled={saving}>
              Cancel
            </button>
            <button className="btn btn-primary" onClick={save} disabled={saving || draft === shown}>
              <Save size={12} /> {saving ? "Saving…" : "Save"}
            </button>
          </div>
        </div>
      )}
    </DefSection>
  );
}
