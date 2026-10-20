# 04: The `input` step kind waits for answers

Spec: [Milestone 6](../spec.md) (Step kind `input`) · Plan: [E](../../../docs/architecture/E-workflow-schema.md), [G. State Machines](../../../docs/architecture/G-state-machines.md), [H](../../../docs/architecture/H-artifact-storage.md)

**What to build:** in the engine, a workflow step where a human answers an Agent's questions. It waits like an approval gate, records an accumulating answers Revision, and is reachable through the `Project` facade.

**Blocked by:** 02, 03

**Status:** resolved

- [x] `StepSpec::Input { artifact, answer_artifact, needs, when }`, `StepKind::Input` (`"input"`), Outcome `answered`; `artifact` must be produced upstream with format `questions`; `answer_artifact` unique across every `rejection_artifact`/`answer_artifact`; the step produces `answer_artifact` as `json`
- [x] `Work::Input`: a pending input step opens and waits; `finish()` returns at it; `DriveOutcome::AtGate` and the public `Gate` carry `kind` (`approval` | `input`); a Loop whose body contains an `approval` or `input` step has no Iteration limit
- [x] Store: `record_answer` in one write transaction (step `done`/`answered`, new Revision of `answer_artifact` with `human_action = "answer"` and file `<answer_artifact>.json` holding every previous round plus this one, events `answer.recorded` and `step.done`); the SQL that hard-codes `'approval'` (`latest_gate`, `waiting_gates`, `gate_views`) covers input steps where the spec says so
- [x] `Project::waiting_input(run)` (step, questions Revision, round) and `Project::answer(run, revision, answers, note, actor)` refusing `NoInputWaiting`, `StaleRevision`, `UnansweredQuestions { missing, unknown }`; approve/reject refuse `NoGateWaiting` while only an input step waits
- [x] `RunState::AwaitingAnswer`; status views and gate views include input steps (`not_reached | waiting | answered`)
- [x] Engine tests with the Fake: ask → answer → ask → answer → deliver → approve; the answers Revision holds every round; a delivered output skips the input step; the static refusals; the refusals of `answer`; approve/reject while only the input waits; no Iteration limit with only an input step in the body
- [x] E, G and H document the step kind
