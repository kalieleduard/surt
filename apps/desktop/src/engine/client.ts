import { invoke, isTauri } from "@tauri-apps/api/core";
import type { CancelImpact, ProjectView, RunDetail, RunsView, WaitingInput, WorkflowView } from "./types";

export const ENGINE_AVAILABLE = isTauri();

export const engine = {
  currentProject: () => invoke<ProjectView | null>("current_project"),
  openProject: (path: string) => invoke<ProjectView>("open_project", { path }),
  runs: () => invoke<RunsView>("runs"),
  runDetail: (subject: string, item: string | null) => invoke<RunDetail>("run_detail", { subject, item }),
  approve: (subject: string, revision: number | null) => invoke<void>("approve", { subject, revision }),
  reject: (subject: string, revision: number | null, reason: string) => invoke<void>("reject", { subject, revision, reason }),
  waitingInput: (subject: string) => invoke<WaitingInput>("waiting_input", { subject }),
  answer: (subject: string, revision: number, answers: Record<string, string>, note: string | null) =>
    invoke<void>("answer", { subject, revision, answers, note }),
  continueRun: (subject: string) => invoke<void>("continue_run", { subject }),
  stop: (subject: string) => invoke<void>("stop", { subject }),
  cancelImpact: (subject: string) => invoke<CancelImpact>("cancel_impact", { subject }),
  cancel: (subject: string) => invoke<string>("cancel", { subject }),
  workflowDefinitions: () => invoke<WorkflowView[]>("workflow_definitions"),
  newRun: (workflow: string, inputs: [string, string][], name: string | null) => invoke<string>("new_run", { workflow, inputs, name }),
};
