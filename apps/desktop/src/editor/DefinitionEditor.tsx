import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, Bot, FilePlus2, FileText, RotateCcw, Save, Trash2, Workflow, X } from "lucide-react";
import { CodeEditor } from "../code/Code";
import { languageOf } from "../code/languages";
import { engine } from "../engine/client";
import type { Catalog } from "../engine/types";
import { AgentAvatar } from "../ui/status";
import { confirmAction } from "../ui/confirm";
import { agentsIn, targetKey, useDefinitions, workflowsIn, type EditTarget } from "./DefinitionsContext";
import "./editor.css";

type Draft = { saved: string | null; text: string };
type FileEntry = { path: string; label: string; group: string; removable: boolean };

const NEW_FILE_NAME = /^[\w.-]+(\/[\w.-]+)*$/;
const NOTICE_MS = 2400;

export function DefinitionEditor() {
  const { target } = useDefinitions();
  if (!target) return null;
  return <EditorPanel key={targetKey(target)} target={target} />;
}

function EditorPanel({ target }: { target: EditTarget }) {
  const { catalog, setCatalog, close, edit } = useDefinitions();
  const [drafts, setDrafts] = useState<Record<string, Draft>>({});
  const files = useMemo(() => filesOf(target, catalog, drafts), [target, catalog, drafts]);
  const [active, setActive] = useState<string | null>(target.kind === "agent" && target.file ? `agents/${target.id}/${target.file}` : null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const current = active ?? files[0]?.path ?? null;
  const draft = current ? drafts[current] : undefined;
  const dirtyPaths = Object.entries(drafts)
    .filter(([, entry]) => entry.text !== entry.saved)
    .map(([path]) => path);
  const related = useRelated(target, catalog, drafts);

  useEffect(() => {
    if (!current || drafts[current]) return;
    engine
      .readDefinition(current)
      .then((text) => setDrafts((all) => (all[current] ? all : { ...all, [current]: { saved: text, text } })))
      .catch((failure) => setError(String(failure)));
  }, [current, drafts]);

  useEffect(() => {
    if (!notice) return;
    const timer = window.setTimeout(() => setNotice(null), NOTICE_MS);
    return () => window.clearTimeout(timer);
  }, [notice]);

  const update = (text: string) => current && setDrafts((all) => ({ ...all, [current]: { saved: all[current]?.saved ?? null, text } }));

  const save = async () => {
    if (saving || dirtyPaths.length === 0) return;
    setSaving(true);
    setError(null);
    try {
      let next: Catalog | null = null;
      for (const path of dirtyPaths) next = await engine.saveDefinition(path, drafts[path].text);
      setDrafts((all) => {
        const saved = { ...all };
        for (const path of dirtyPaths) saved[path] = { saved: all[path].text, text: all[path].text };
        return saved;
      });
      if (next) setCatalog(next);
      setNotice(dirtyPaths.length === 1 ? `Saved ${shortName(dirtyPaths[0])}` : `Saved ${dirtyPaths.length} files`);
    } catch (failure) {
      setError(String(failure));
    } finally {
      setSaving(false);
    }
  };

  const revert = () => current && draft && setDrafts((all) => ({ ...all, [current]: { saved: draft.saved, text: draft.saved ?? "" } }));

  const leave = async (then: () => void) => {
    if (dirtyPaths.length && !(await confirmAction(`${dirtyPaths.length} file(s) have unsaved changes. Discard them?`, "Unsaved changes", "Discard"))) return;
    then();
  };

  const remove = async (file: FileEntry) => {
    if (!(await confirmAction(`Delete ${file.path}? This removes the file from disk.`, "Delete file", "Delete"))) return;
    try {
      if (drafts[file.path]?.saved !== null) setCatalog(await engine.deleteDefinition(file.path));
      setDrafts((all) => {
        const rest = { ...all };
        delete rest[file.path];
        return rest;
      });
      if (current === file.path) setActive(null);
    } catch (failure) {
      setError(String(failure));
    }
  };

  const create = (relative: string) => {
    if (target.kind !== "agent") return;
    const path = `agents/${target.id}/${relative}`;
    setDrafts((all) => (all[path] ? all : { ...all, [path]: { saved: null, text: "" } }));
    setActive(path);
  };

  const problems = catalog?.workflows.filter((workflow) => workflow.problem) ?? [];
  const Icon = target.kind === "agent" ? Bot : Workflow;
  const title = target.kind === "agent" ? target.id : target.name;

  return (
    <div className="editor-backdrop">
      <section className="editor" role="dialog" aria-label={`Edit ${title}`}>
        <header className="editor__head">
          {target.kind === "agent" ? <AgentAvatar agent={target.id} size={24} /> : <Icon size={16} />}
          <span className="editor__title">{title}</span>
          <span className="caption">{target.kind === "agent" ? "agent" : "workflow"}</span>
          <span className="editor__spacer" />
          {notice ? <span className="editor__notice">{notice}</span> : null}
          {dirtyPaths.length ? <span className="editor__dirty mono">{dirtyPaths.length} unsaved</span> : null}
          <button className="btn btn-ghost" onClick={revert} disabled={!draft || draft.text === draft.saved} title="Discard changes to this file">
            <RotateCcw size={13} /> Revert
          </button>
          <button className="btn btn-primary" onClick={save} disabled={saving || dirtyPaths.length === 0} title="Save all changes (Ctrl+S)">
            <Save size={13} /> {saving ? "Saving…" : "Save"}
          </button>
          <button className="icon-btn" onClick={() => leave(close)} aria-label="Close editor">
            <X size={15} />
          </button>
        </header>
        <div className="editor__main">
          <nav className="editor__files">
            <FileList files={files} current={current} drafts={drafts} onSelect={setActive} onRemove={remove} />
            {target.kind === "agent" ? <NewFile onCreate={create} /> : null}
            <Related target={target} related={related} onOpen={(next) => leave(() => edit(next))} />
          </nav>
          <div className="editor__pane">
            {current ? (
              <div className="editor__path mono">
                <FileText size={12} /> .surt/{current}
                {draft?.saved === null ? <span className="editor__badge">new</span> : null}
              </div>
            ) : null}
            {draft ? (
              <CodeEditor key={current} value={draft.text} language={languageOf(current ?? "")} onChange={update} onSave={save} wrap={current?.endsWith(".md") ?? false} />
            ) : (
              <div className="editor__loading caption">{current ? "Loading…" : "No files"}</div>
            )}
            {error ? (
              <div className="editor__error">
                <AlertTriangle size={13} /> {error}
              </div>
            ) : null}
            {problems.length ? <Problems problems={problems} /> : null}
          </div>
        </div>
      </section>
    </div>
  );
}

function FileList({
  files,
  current,
  drafts,
  onSelect,
  onRemove,
}: {
  files: FileEntry[];
  current: string | null;
  drafts: Record<string, Draft>;
  onSelect: (path: string) => void;
  onRemove: (file: FileEntry) => void;
}) {
  const groups = [...new Set(files.map((file) => file.group))];
  return (
    <>
      {groups.map((group) => (
        <div key={group} className="editor__group">
          <span className="caption">{group}</span>
          {files
            .filter((file) => file.group === group)
            .map((file) => {
              const entry = drafts[file.path];
              const dirty = entry && entry.text !== entry.saved;
              return (
                <div key={file.path} className={`editor__file${file.path === current ? " is-active" : ""}`}>
                  <button className="editor__file-name" onClick={() => onSelect(file.path)} title={file.path}>
                    <span>{file.label}</span>
                    {dirty ? <i className="editor__file-dot" aria-label="unsaved" /> : null}
                  </button>
                  {file.removable ? (
                    <button className="icon-btn icon-btn--small editor__file-remove" onClick={() => onRemove(file)} aria-label={`Delete ${file.label}`}>
                      <Trash2 size={11} />
                    </button>
                  ) : null}
                </div>
              );
            })}
        </div>
      ))}
    </>
  );
}

function NewFile({ onCreate }: { onCreate: (relative: string) => void }) {
  const [name, setName] = useState<string | null>(null);
  const [folder, setFolder] = useState("references");
  if (name === null) {
    return (
      <button className="editor__add" onClick={() => setName("")}>
        <FilePlus2 size={13} /> New file
      </button>
    );
  }
  const valid = NEW_FILE_NAME.test(name) && !name.split("/").includes("..");
  const submit = () => {
    if (!valid) return;
    onCreate(`${folder}/${name}`);
    setName(null);
  };
  return (
    <form
      className="editor__new"
      onSubmit={(event) => {
        event.preventDefault();
        submit();
      }}
    >
      <select value={folder} onChange={(event) => setFolder(event.target.value)}>
        <option value="references">references/</option>
        <option value="schemas">schemas/</option>
      </select>
      <input
        autoFocus
        placeholder="name.md"
        value={name}
        onChange={(event) => setName(event.target.value)}
        onKeyDown={(event) => event.key === "Escape" && setName(null)}
      />
      <span className="editor__new-actions">
        <button type="button" className="btn btn-ghost" onClick={() => setName(null)}>
          Cancel
        </button>
        <button type="submit" className="btn btn-primary" disabled={!valid}>
          Add
        </button>
      </span>
    </form>
  );
}

function Related({ target, related, onOpen }: { target: EditTarget; related: EditTarget[]; onOpen: (target: EditTarget) => void }) {
  if (!related.length) return null;
  return (
    <div className="editor__group editor__related">
      <span className="caption">{target.kind === "agent" ? "Used in flows" : "In this flow"}</span>
      {related.map((entry) => (
        <button key={targetKey(entry)} className="editor__link" onClick={() => onOpen(entry)}>
          {entry.kind === "agent" ? <AgentAvatar agent={entry.id} size={16} /> : <Workflow size={13} />}
          <span>{entry.kind === "agent" ? entry.id : entry.name}</span>
        </button>
      ))}
    </div>
  );
}

function Problems({ problems }: { problems: Catalog["workflows"] }) {
  return (
    <div className="editor__problems">
      {problems.map((workflow) => (
        <div key={workflow.name} className="editor__problem">
          <AlertTriangle size={13} />
          <span>
            <b>{workflow.name}</b> {workflow.problem}
          </span>
        </div>
      ))}
    </div>
  );
}

function filesOf(target: EditTarget, catalog: Catalog | null, drafts: Record<string, Draft>): FileEntry[] {
  if (target.kind === "workflow") {
    const file = catalog?.workflows.find((workflow) => workflow.name === target.name)?.file ?? `workflows/${target.name}.toml`;
    return [{ path: file, label: file.replace("workflows/", ""), group: "Flow", removable: false }];
  }
  const prefix = `agents/${target.id}/`;
  const listed = catalog?.agents.find((agent) => agent.id === target.id)?.files ?? ["instructions.md", "agent.toml"];
  const fresh = Object.keys(drafts)
    .filter((path) => path.startsWith(prefix))
    .map((path) => path.slice(prefix.length))
    .filter((file) => !listed.includes(file));
  return [...listed, ...fresh].map((file) => {
    const folder = file.includes("/") ? file.slice(0, file.indexOf("/")) : "";
    return {
      path: prefix + file,
      label: folder ? file.slice(folder.length + 1) : file,
      group: folder ? capitalize(folder) : "Agent",
      removable: Boolean(folder),
    };
  });
}

function useRelated(target: EditTarget, catalog: Catalog | null, drafts: Record<string, Draft>): EditTarget[] {
  const [usedIn, setUsedIn] = useState<EditTarget[]>([]);
  const workflowFile = target.kind === "workflow" ? (catalog?.workflows.find((entry) => entry.name === target.name)?.file ?? null) : null;
  const workflowText = workflowFile ? drafts[workflowFile]?.text : undefined;
  const agentId = target.kind === "agent" ? target.id : null;
  const workflows = catalog?.workflows;

  useEffect(() => {
    if (!agentId || !workflows) return;
    let cancelled = false;
    Promise.all(workflows.map((entry) => engine.readDefinition(entry.file).then((text) => (agentsIn(text).includes(agentId) ? entry.name : null))))
      .then((names) => {
        if (!cancelled) setUsedIn(names.filter((name): name is string => Boolean(name)).map((name) => ({ kind: "workflow", name })));
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [agentId, workflows]);

  if (agentId) return usedIn;
  if (!workflowText) return [];
  const agents: EditTarget[] = agentsIn(workflowText).map((id) => ({ kind: "agent", id }));
  const children: EditTarget[] = workflowsIn(workflowText).map((name) => ({ kind: "workflow", name }));
  return [...agents, ...children];
}

function shortName(path: string) {
  return path.slice(path.lastIndexOf("/") + 1);
}

function capitalize(text: string) {
  return text.slice(0, 1).toUpperCase() + text.slice(1);
}
