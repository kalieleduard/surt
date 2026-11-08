import { useEffect, useMemo, useState } from "react";
import { History } from "lucide-react";
import { useDefinitions } from "../editor/DefinitionsContext";
import { engine } from "../engine/client";
import { definedSteps } from "../engine/definedSteps";
import { definitionChanges, hasChanges } from "../engine/definitionChanges";
import { parseToml } from "../engine/toml";
import { useRunView } from "./RunViewContext";

const LISTED = 3;

export function FrozenNotice() {
  const { graph, preview } = useRunView();
  const { catalog } = useDefinitions();
  const [live, setLive] = useState<{ file: string; source: string } | null>(null);
  const file = graph && !graph.preview && !graph.item ? catalog?.workflows.find((entry) => entry.name === graph.workflow)?.file : undefined;

  useEffect(() => {
    if (!file) return;
    let cancelled = false;
    engine
      .readDefinition(file)
      .then((source) => !cancelled && setLive({ file, source }))
      .catch(() => !cancelled && setLive(null));
    return () => {
      cancelled = true;
    };
  }, [file, catalog]);

  const changes = useMemo(() => {
    if (!graph || !live || live.file !== file) return null;
    try {
      return definitionChanges(graph.steps, definedSteps(parseToml(live.source)));
    } catch {
      return null;
    }
  }, [graph, live, file]);

  if (!graph || !file || !changes || !hasChanges(changes)) return null;
  return (
    <div className="frozen-notice" role="status">
      <History size={14} strokeWidth={1.75} />
      <span>
        Run {graph.run} uses the <b>{graph.workflow}</b> flow as it was when it started. {summary(changes)}
      </span>
      <button className="btn btn-ghost" onClick={() => preview(graph.workflow)}>
        Show current flow
      </button>
    </div>
  );
}

function summary(changes: ReturnType<typeof definitionChanges>) {
  const parts = [];
  if (changes.added.length) parts.push(`adds ${listed(changes.added)}`);
  if (changes.removed.length) parts.push(`drops ${listed(changes.removed)}`);
  if (changes.changed.length) parts.push(`changes ${listed(changes.changed)}`);
  return `The current file ${parts.join(", ")}.`;
}

function listed(keys: string[]) {
  const shown = keys.slice(0, LISTED).join(", ");
  return keys.length > LISTED ? `${shown} and ${keys.length - LISTED} more` : shown;
}
