import { invoke, isTauri } from "@tauri-apps/api/core";
import type { ProjectView, RunDetail, RunsView } from "./types";

export const ENGINE_AVAILABLE = isTauri();

export const engine = {
  currentProject: () => invoke<ProjectView | null>("current_project"),
  openProject: (path: string) => invoke<ProjectView>("open_project", { path }),
  runs: () => invoke<RunsView>("runs"),
  runDetail: (subject: string, item: string | null) => invoke<RunDetail>("run_detail", { subject, item }),
};
