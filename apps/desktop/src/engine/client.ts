import { invoke, isTauri } from "@tauri-apps/api/core";
import type { CancelImpact, Catalog, Checkouts, ProjectView, RunDetail, RunsView, WaitingInput, WorkflowView } from "./types";

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
  definitions: () => invoke<Catalog>("definitions"),
  readDefinition: (path: string) => invoke<string>("read_definition", { path }),
  saveDefinition: (path: string, content: string) => invoke<Catalog>("save_definition", { path, content }),
  deleteDefinition: (path: string) => invoke<Catalog>("delete_definition", { path }),
  checkouts: () => invoke<Checkouts>("checkouts"),
  switchBranch: (worktree: string, branch: string, create: boolean) => invoke<Checkouts>("switch_branch", { worktree, branch, create }),
  createWorktree: (branch: string, base: string | null) => invoke<Checkouts>("create_worktree", { branch, base }),
};
