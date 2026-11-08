import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { RunGraph } from "../data/types";
import { ENGINE_AVAILABLE, engine } from "./client";
import { buildRunGraph, graphId } from "./runGraph";
import type { DriverView, ProjectView, RunsView, SubjectView } from "./types";
import { usePolled } from "./usePolled";

const PROJECT_KEY = "surt-project";
const TERMINAL = ["succeeded", "cancelled"];
const CLOSING_EVENT = "surt://closing";

type Selection = { subject: string; item: string | null };

type Engine = {
  available: boolean;
  project: ProjectView | null;
  projectError: string | null;
  chooseProject: () => void;
  runs: RunsView | null;
  subjects: SubjectView[];
  drivers: DriverView[];
  closing: boolean;
  runsError: string | null;
  refresh: () => void;
  selection: Selection | null;
  show: (subject: string, item: string | null) => void;
  drill: (item: string) => void;
  back: () => void;
  graph: RunGraph | null;
  detailError: string | null;
};

const EngineContext = createContext<Engine | null>(null);

export function useEngine() {
  const value = useContext(EngineContext);
  if (!value) throw new Error("useEngine needs an EngineProvider");
  return value;
}

export function EngineProvider({ children }: { children: ReactNode }) {
  const { project, projectError, chooseProject } = useProject();
  const loadRuns = useMemo(() => (project ? engine.runs : null), [project]);
  const runs = usePolled(loadRuns, project?.root ?? "");
  const subjects = useMemo(() => [...(runs.data?.subjects ?? [])].reverse(), [runs.data]);
  const [chosen, setChosen] = useState<Selection | null>(null);
  const closing = useClosing();
  const selection = useMemo(() => resolveSelection(chosen, subjects), [chosen, subjects]);
  const loadDetail = useMemo(
    () => (selection ? () => engine.runDetail(selection.subject, selection.item) : null),
    [selection?.subject, selection?.item],
  );
  const detail = usePolled(loadDetail, selection ? graphId(selection.subject, selection.item) : "");
  const subject = subjects.find((candidate) => candidate.name === selection?.subject);
  const graph = useMemo(
    () => (project && subject && selection && detail.data ? buildRunGraph(project, subject, selection.item, detail.data) : null),
    [project, subject, selection?.item, detail.data],
  );

  const value: Engine = {
    available: ENGINE_AVAILABLE,
    project,
    projectError,
    chooseProject: () => {
      setChosen(null);
      chooseProject();
    },
    runs: runs.data,
    subjects,
    drivers: runs.data?.drivers ?? [],
    closing,
    runsError: runs.error,
    refresh: () => {
      runs.refresh();
      detail.refresh();
    },
    selection,
    show: (name, item) => setChosen({ subject: name, item }),
    drill: (item) => selection && setChosen({ subject: selection.subject, item }),
    back: () => selection && setChosen({ subject: selection.subject, item: null }),
    graph,
    detailError: detail.error,
  };
  return <EngineContext.Provider value={value}>{children}</EngineContext.Provider>;
}

function resolveSelection(chosen: Selection | null, subjects: SubjectView[]): Selection | null {
  if (chosen && subjects.some((subject) => subject.name === chosen.subject)) return chosen;
  const active = subjects.find((subject) => !subject.states.some((state) => TERMINAL.includes(state)));
  const first = active ?? subjects[0];
  return first ? { subject: first.name, item: null } : null;
}

function useClosing() {
  const [closing, setClosing] = useState(false);
  useEffect(() => {
    if (!ENGINE_AVAILABLE) return;
    const unlisten = listen(CLOSING_EVENT, () => setClosing(true));
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);
  return closing;
}

function useProject() {
  const [project, setProject] = useState<ProjectView | null>(null);
  const [projectError, setProjectError] = useState<string | null>(null);

  const openAt = useCallback(async (path: string) => {
    try {
      const opened = await engine.openProject(path);
      setProject(opened);
      setProjectError(null);
      remember(opened.root);
    } catch (failure) {
      setProjectError(String(failure));
    }
  }, []);

  useEffect(() => {
    if (!ENGINE_AVAILABLE) return;
    engine
      .currentProject()
      .then((current) => {
        if (current) return setProject(current);
        const last = recalled();
        if (last) openAt(last);
      })
      .catch((failure) => setProjectError(String(failure)));
  }, [openAt]);

  const chooseProject = useCallback(async () => {
    const path = await open({ directory: true, title: "Open a Surt project" });
    if (typeof path === "string") openAt(path);
  }, [openAt]);

  return { project, projectError, chooseProject };
}

function remember(root: string) {
  try {
    localStorage.setItem(PROJECT_KEY, root);
  } catch {
    return;
  }
}

function recalled() {
  try {
    return localStorage.getItem(PROJECT_KEY);
  } catch {
    return null;
  }
}
