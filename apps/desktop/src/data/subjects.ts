import { TASKS } from "./tasks";
import type { Subject } from "./types";

export const ACTIVE_SUBJECT = "20261107-3";

export const SUBJECTS: Subject[] = [
  {
    id: "20261107-3",
    slug: "oauth-login",
    workflow: "feature",
    state: "running",
    status: "implement · 1/4 merged",
    children: TASKS.map((task) => ({ key: task.key, title: task.title, state: task.state, status: task.status })),
  },
  { id: "20261107-2", slug: "billing-export", workflow: "feature", state: "waiting", status: "answer-spec · 3 questions" },
  { id: "20261106-5", slug: "audit-log", workflow: "feature", state: "waiting", status: "approve-tasks · manifest r1" },
  { id: "20261106-1", slug: "progress-footer", workflow: "feature", state: "succeeded", status: "merged into master" },
  { id: "20261105-2", slug: "sqlite-migrations", workflow: "feature", state: "blocked", status: "build · attempts 3/3" },
  { id: "20261104-1", slug: "theme-tokens", workflow: "feature", state: "cancelled", status: "cancelled at plan-tasks" },
];

export const RUN_SUMMARY = {
  subject: "20261107-3",
  slug: "oauth-login",
  request: "Login with GitHub OAuth on the web dashboard",
  base: "master @ b72ec4f",
  branch: "surt/20261107-3",
  stepsDone: 7,
  stepsTotal: 12,
  elapsed: "1h 04m",
  slots: { used: 2, total: 3 },
  delta: "+1 Task merged · 12 min ago",
};
