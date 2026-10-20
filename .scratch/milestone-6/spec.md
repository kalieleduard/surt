# Spec: Milestone 6 — talk the spec through

Labels: ready-for-agent
Type: spec
Status: open (7 of 8 tickets resolved; 08 awaits CI and the manual smoke; tickets in [issues/](issues/))
Map: [Surt — Architecture and implementation plan](../surt-architecture/map.md)
Plan: [E. Workflow Definition Schema](../../docs/architecture/E-workflow-schema.md), [F. Agent Contract](../../docs/architecture/F-agent-contract.md), [G. State Machines](../../docs/architecture/G-state-machines.md), [H. Artifact Storage](../../docs/architecture/H-artifact-storage.md), [I. Recovery](../../docs/architecture/I-recovery.md), [J. CLI Surface](../../docs/architecture/J-cli.md), [D. Database](../../docs/architecture/D-database.md), [ADR 0014](../../docs/adr/0014-a-conversation-continues-across-a-loop.md)
Previous: [Milestone 5](../milestone-5/spec.md)

## Problem Statement

The feature Template turns a request into a spec in one shot. The Spec Agent runs headless with stdin closed and no question tool, so whatever it does not know it must guess, and it writes the guesses under **Open questions**. The developer answers them by rejecting the spec with `surt reject -m "…"`, and a new Spec Agent starts with no memory: it rereads the repository, the previous spec and the reason, and rebuilds the reasoning that produced the questions.

- **Questions arrive as a rejection.** A spec that is fine except for two decisions only the developer can make has to be rejected to get them answered, and the answers are a free-text reason the agent must parse.
- **Nothing makes the questions good.** Nothing stops the agent from asking what the repository already answers, asking ten things at once, or burying a blocking decision among trivia. Nothing gives the developer a recommended answer to accept.
- **Every round starts cold.** Each answer costs a fresh session that rereads the code and loses why the agent asked.
- **A spec can be "done" with blocking questions in it.** The agent has to deliver `spec.md` to finish, so an unresolved product decision reaches the gate as an assumption the developer may not notice.

## Solution

Milestone 6 lets an Agent ask the developer questions and wait for the answers, inside one continuing conversation, until it delivers its output with no question left open. The engine stays generic: it gains a step kind, a format and a profile field; the feature Template's Spec Agent is the first to use them.

```
$ surt run feature --input request="add login"
✔ created 20261020-add-login   workflow: feature   base: main @ a11c9d0
● spec started
? spec-questions rev 1 awaiting answers (round 1)
  1. Log in by e-mail only, or by username too?
     why: changes the users table and acceptance criteria 2–4
     recommended: e-mail only
  2. Does a session expire, and after how long?
     why: decides whether the Feature needs a refresh flow
     recommended: 7 days, no refresh
  answer: surt answer 20261020-add-login

$ surt answer add-login
1. Log in by e-mail only, or by username too? [e-mail only] ⏎
2. Does a session expire, and after how long? [7 days, no refresh] 30 days
✔ answered spec-questions rev 1 by kaliel
● spec continued (iteration 2, same conversation)
⏸ spec rev 1 awaiting approval in .surt/subjects/20261020-add-login
  approve: surt approve 20261020-add-login
  reject:  surt reject 20261020-add-login -m "<reason>"
```

- **An output belongs to Outcomes.** `[outputs.<name>] outcomes = [...]` says which Outcomes deliver it; absent, every Outcome does, as today. The Spec Agent delivers `spec` on `done` and `spec-questions` on `questions`.
- **A format for questions.** `format = "questions"` is Surt-owned, like the Item List: one to five questions, each with an id, the question, why it matters and a recommended answer.
- **A step kind for answers.** `kind = "input"` waits for a human like `approval` does: no Execution, no limit. `surt answer` records the answers as a human-origin Revision and drives on. It runs only when the agent asked, through `when`.
- **The conversation continues.** An Agent whose profile says `conversation = "loop"` continues the Claude conversation of its last successful Execution of the same step in the same Loop. Each Execution forks it under its own session id and receives only the Inputs that changed.
- **Correctness never depends on the conversation.** Answers accumulate in one artifact. A conversation that cannot be continued restarts fresh with every Input, in the same Execution, and says so.

## User Stories

### Asking good questions

1. As a developer, I want the Spec Agent to ask me only what it cannot learn from the repository and what changes the spec's behaviour or acceptance criteria, so that every question is worth my time.
2. As a developer, I want at most five questions per round, so that a round is something I can answer in a minute.
3. As a developer, I want every question to say why it matters and to come with a recommended answer, so that I can accept it when I agree and only type when I disagree.
4. As a developer, I want optional choices listed when the question has a closed set of answers, so that I pick instead of phrasing.
5. As a developer, I want a delivery with no question, more than five, a question without a recommendation, or a duplicated id refused as an invalid delivery, so that a bad round costs an Attempt and never reaches me.
6. As a developer, I want the agent to deliver the spec only when no blocking question is left, so that a spec at the gate has no hidden product decision in it.
7. As a developer, I want the delivered spec to record each question and its answer under **Decisions**, so that the reviewer and the Task Agent see why the spec says what it says.

### Answering

8. As a developer, I want `surt run` and `surt continue` to stop with exit `0` and print the questions when the agent asks, so that a question is a normal pause like a gate.
9. As a developer, I want `surt status` to show the run `AWAITING ANSWER` with the questions' artifact, round and the command to answer, and `status --json` to list the waiting input step, so that I find a waiting question from any terminal or script.
10. As a developer at a terminal, I want `surt answer <subject>` to ask the questions one by one with the recommendation as the default, so that pressing Enter accepts it.
11. As a developer, I want `surt answer <subject> --accept` to accept every recommendation, so that a round I agree with costs one command.
12. As a developer scripting Surt, I want `surt answer <subject> --file answers.toml` with one answer per question id, so that answers can come from a file.
13. As a developer, I want `-m "<text>"` to accept the recommendations and add a free note to the round, so that I can say "decide the rest yourself" or add what the questions did not cover.
14. As a developer, I want an answers file that misses a question id or names an unknown one refused (exit `3`) listing the ids, so that no question is silently unanswered.
15. As a developer without a terminal and without `--accept`, `--file` or `-m`, I want a usage error (exit `2`), so that a script never hangs waiting for input.
16. As a developer, I want `surt answer` to drive the run on right after recording, so that answering feels like a conversation and not like two commands.
17. As a developer, I want `surt answer` on a run with no input step waiting refused (exit `3`) saying what the run waits for, and `surt approve`/`reject` refused while only an input step waits, so that I never answer or approve the wrong thing.
18. As a developer, I want `surt show <subject> spec-questions` and `spec-answers` to print every round, so that I can review the whole conversation later.

### One conversation

19. As a developer, I want the agent to remember what it read and why it asked when I answer, so that the next round builds on the last one instead of starting over.
20. As a developer, I want a rejection at the gate to continue the same conversation, so that the agent revises the spec it wrote with the reasons it already has.
21. As a developer who edits `spec.md` in the Working Copy before rejecting, I want the continued conversation to receive my edit, so that the agent never revises from its own memory of a spec I changed.
22. As a developer, I want each round to send only what is new, so that a long conversation does not resend every Input every time.
23. As a developer, I want every Execution to keep its own session id, so that `surt inspect` and the logs still map one Execution to one session.
24. As a developer, I want `surt inspect <execution>` to say whether it continued an earlier Execution (and which) or started fresh, and why, so that I can tell when the agent lost its memory.
25. As a developer whose Execution was interrupted or failed, I want the next Attempt to continue from the last successful one, so that a half-finished turn never pollutes the conversation.
26. As a developer, I want a conversation that cannot be continued to restart fresh with every Input in the same Execution, spending no Attempt, so that a lost transcript costs memory, never correctness.
27. As a developer, I want the conversation to stay inside its Loop, so that the Task Agent and the Builders still start with no memory of it.

### Milestone close-out

28. As a Surt maintainer, I want the M6 suite to pass in CI on macOS, Linux and Windows, and an M6 section of the manual Claude Code smoke on macOS (two rounds of questions, one rejection, one restart after deleting the transcript), so that the conversation is checked against the real runtime.
29. As a Surt maintainer, I want the map's "a new Claude session per execution", F's ban on `--resume`/`--fork-session` and I's "without trusting an old Claude session" rewritten to what ADR 0014 decides, so that the plan still wins on divergence.

## Implementation Decisions

Nothing in A–L decides M6 yet. Every point below is decided here and written back into the section named.

### What the runtime already showed

Probe on Windows, Claude Code 2.1.304, subscription auth, 2026-10-20, F's flag set with `--safe-mode --setting-sources user`, an allowlisted environment and `claude-haiku-5-5`. Execution A was told a code word; B continued A; D continued B with another model.

- `--resume <A> --fork-session --session-id <B>` works under `-p` with `--json-schema`: `system/init` reports `session_id = B`, and B's `structured_output` delivered the code word from A's turn.
- Forking leaves A's transcript untouched; B's transcript is a complete copy of A's history plus B's turn. Continuing needs only the previous Execution's transcript, not the chain.
- The continued turn read A's prompt cache (`cache_read_input_tokens` 4263, `cache_creation_input_tokens` 152).
- `--resume <A> --session-id <B>` without `--fork-session` is refused before launch (`--session-id can only be used with --continue or --resume if --fork-session is also specified.`).
- Resuming a session with no transcript emits no `system/init`, only a `result` with `subtype = "error_during_execution"`, `num_turns = 0`, `errors = ["No conversation found with session ID: <A>"]`, the same text on stderr, and exit `1`.
- Continuing with another model is accepted silently and remembers, but reads no cache. Within a run the Agent is frozen, so it never happens by Surt's choice.
- Transcripts live at `~/.claude/projects/<working directory, mangled>/<session id>.jsonl`. An Agent without a worktree runs at the project root ([drive/launch.rs](../../crates/surt-engine/src/drive/launch.rs)), which is stable; moving the project restarts the conversation.
- An Execution that failed still wrote its transcript, and the CLI would continue it. Choosing only successful Executions is Surt's job.

### Outputs per Outcome (F)

- **Schema.** `[outputs.<name>] outcomes = ["<outcome>", …]`: a non-empty, unique subset of the Agent's Outcomes. Absent means every Outcome. An Outcome that delivers no output is allowed.
- **Envelope schema.** `envelope_schema` keeps `outcome` as an enum and lists every declared output as a property with `additionalProperties = false`; `required` holds only the outputs every Outcome delivers. No `oneOf`: the per-Outcome rule is Surt's to enforce, not the model's.
- **Validation.** `check_names` requires exactly the outputs of the delivered Outcome: `MissingOutput` for one it should deliver, `UndeclaredOutput` for one it should not. Item files and delivery checks apply only to delivered outputs.
- **Static validation (E).** A step that consumes an artifact produced only on some Outcomes keeps today's rule (it is produced upstream); a missing Revision at runtime is the existing optional/required rule. A delivery check's output must be delivered by the Outcome `no_blocking_finding` names.

### Format `questions` (F, H)

- `format = "questions"` on an output (and an input) holds one JSON file: `{ "questions": [ { "id", "question", "why", "recommended", "options"? } ] }`. Rules: 1 to 5 entries (`MAX_QUESTIONS = 5`); `id` matches the Item List key rules; every string non-empty; ids unique; `options`, when present, has 2 or more unique non-empty strings and contains `recommended`. No `json_schema` on it (`MisplacedJsonSchema` as for Text). An invalid file is an invalid delivery, through a `QuestionsProblem` like `ItemListProblem`.

### Step kind `input` (E, G)

- **Schema.** `kind = "input"` with `artifact` (an artifact of format `questions`), `answer_artifact`, optional `needs` and `when`; one Outcome, `answered` (`INPUT_OUTCOMES`). `answer_artifact` must be unique among every step's `rejection_artifact` and `answer_artifact`. The input step produces `answer_artifact` as format `json` for `flow::check`.
- **Scheduling.** `Work::Input` mirrors `Work::Approval`: a pending input step opens (pending → running) and waits; `finish()` returns at it as at a gate. `DriveOutcome::AtGate` carries both kinds; the public `Gate` gains a `kind` (`approval` | `input`) and the artifact. A Loop whose body contains an `approval` or an `input` step has no Iteration limit.
- **Answers accumulate (H).** Recording a round publishes a new human-origin Revision of `answer_artifact` (`human_action = "answer"`, file `<answer_artifact>.json`): `{ "rounds": [ { "questions": <the answered Revision's questions>, "answers": { "<id>": "<text>" }, "note"?: "<text>" } ] }`, the previous Revision's rounds followed by this one. Rounds are numbered by their position. The step becomes `done` with Outcome `answered` and the events `answer.recorded` and `step.done` are written in the same transaction, as `record_decision` does for gates.
- **Project facade.** `Project::waiting_input(run)` returns the waiting input step with its questions Revision and round number; `Project::answer(run, revision, answers, note, actor)` refuses with `NoInputWaiting`, `StaleRevision` or `UnansweredQuestions { missing, unknown }`. Approve and reject act on approval steps only; with only an input step waiting they refuse `NoGateWaiting` naming the input step.
- **Status.** `RunState::AwaitingAnswer` when an input step is `running`; `waiting_gates` and `gate_views` cover both kinds; `status --json` gate entries gain `kind`, and an input entry's `status` is `not_reached | waiting | answered`.
- **Working Copy.** The questions and answers are not Working Copy artifacts; only approval artifacts are.

### Conversation (F, I)

- **Profile.** `[profile] conversation = "none" | "loop"`, default `"none"`. Every existing Agent and test keeps today's launch. A `"loop"` Agent used by a step outside every Loop body refuses at creation (`InvalidDefinition` on the step).
- **Previous Execution.** For an agent step in a Loop whose frozen Agent says `"loop"`, the Driver looks up the latest `succeeded` Execution of a Step Run with the same step key in the same Loop of the same run, at any earlier Iteration or Attempt, with a recorded `session_id`. Interrupted, cancelled and failed Executions are never chosen.
- **Request.** `RuntimeRequest` gains `continuation: Option<Continuation { session_id, execution_id, context }>`, where `context` is the delta context below. `RuntimeRequest.context` stays the full context.
- **Delta context.** The Context Assembler writes a second file, `context-continued.md`, with the same sections but only the Inputs whose fixed Revision differs from the previous Execution's `execution_input` for that name, excluding Revisions that previous Execution produced itself. A human-origin edit of an output it produced is therefore included; its own unchanged output is not.
- **Launcher.** A `"loop"` Agent never gets `--no-session-persistence`. With a continuation: `--resume <previous session id> --fork-session --session-id <new>`, stdin = the delta context; every other argument unchanged. Without one: today's arguments minus `--no-session-persistence`, stdin = the full context.
- **Restart.** When a continued launch ends without a `system/init` event, the Claude Code runtime launches again in the same Execution, fresh, with the full context and the same new session id, and reports the restart: `transcript_missing` when the result's `errors` contain `No conversation found`, otherwise `continuation_refused`. A failure after a continued `system/init` is an ordinary failure. The Fake runtime gains a scripted `transcript_missing` behaviour that makes it restart the same way.
- **Recorded.** `runtime_info` gains `conversation`: `{ "continued_from": "<execution id>" }`, `{ "restarted": "<reason>", "previous": "<execution id>" }`, or `{ "fresh": "no_previous" }` for a `"loop"` Agent's first Execution; absent for `"none"` Agents. Display only; `inspect <execution>` prints one line from it.
- **Recovery (I).** "Without trusting an old Claude session" becomes "without trusting an old Claude session for correctness": every Recovery decision still comes from rows, Revisions and Git.

### CLI (J)

- **`surt answer <subject> [--accept | --file <path> | -m <text>]`**, mutually exclusive (exit `2`). With none, on a TTY, it prints each question (with `options` when present) and reads one line, Enter meaning the recommendation; with none and stdin not a TTY, exit `2`. `--file` reads TOML `"<id>" = "<answer>"`. `-m` records the recommendations plus `note`. It answers the Revision it printed, refusing `StaleRevision` if another round replaced it, prints `✔ answered <artifact> rev N by <actor>`, then drives exactly as `surt continue <subject>` does.
- **Waiting output.** `driving.rs`'s `awaiting` prints, for an input step: `? <artifact> rev N awaiting answers (round R)`, the questions as above, and `answer: surt answer <subject>`; exit `0`. Approval blocks are unchanged.
- **Continued line.** The progress line of an agent Execution that continued says `● <step> continued (iteration I, same conversation)`; one that restarted says `● <step> restarted (iteration I): <reason>`.
- **Status.** `AWAITING ANSWER` in `state_name`; the detail lists the input step with its artifact, round and `answer:` line.

### Database (D)

- **Migration `0002_input_steps.sql`**, the first after `0001`. `step_run.kind` accepts `'input'` and `artifact_revision.human_action` accepts `'answer'`. Both tables are referenced by `ON DELETE CASCADE` foreign keys, so the migration follows SQLite's twelve-step table rebuild with foreign keys off: the file starts with `-- no-transaction`, sets `PRAGMA foreign_keys = OFF`, rebuilds both tables and their indexes inside an explicit `BEGIN`/`COMMIT`, runs `PRAGMA foreign_key_check` before committing, and sets `PRAGMA foreign_keys = ON` at the end. Never `ALTER TABLE … RENAME` the old table with foreign keys on: SQLite would repoint the children at it.
- **Plan copy.** `docs/architecture/migrations/0002_input_steps.sql` is the byte-for-byte copy the store test compares against, as for `0001`.
- **Tests.** A database created by `0001` with rows in every table migrates with every row and foreign key intact (`integrity_check`, `foreign_key_check`); an `input` step row and an `answer` Revision insert after it; the migration count and `SchemaTooNew` expectations follow.

### The feature Template

- **`feature.toml`.** `spec-loop`'s body becomes `["spec", "answer-spec", "approve-spec"]`, with `answer-spec` (`kind = "input"`, `artifact = "spec-questions"`, `answer_artifact = "spec-answers"`, `needs = ["spec"]`, `when = { step = "spec", outcome = "questions" }`) and `approve-spec` gaining `needs = ["answer-spec"]`, `when = { step = "spec", outcome = "done" }`.
- **Spec Agent.** `outcomes = ["done", "questions"]`; `[outputs.spec] outcomes = ["done"]`; `[outputs.spec-questions] format = "questions"`, `outcomes = ["questions"]`; `[inputs.spec-answers] format = "json"`, `optional = true`; `[profile] conversation = "loop"`. Instructions cover stories 1–7 and replace **Open questions** with **Decisions**. The other four Agents are unchanged.
- Runs created before M6 keep the definitions they froze; a project gets the new Template files by deleting the old ones and rerunning `surt init --template feature`.

### Documentation updates

- **ADR 0014** records the decision; **L** lists it. **Map:** the brief's constraint becomes "a new Claude session id per Execution; an Agent may continue the conversation of its previous Execution in the same Loop (ADR 0014)".
- **CONTEXT.md:** **Conversation** (_Avoid_: session, thread) and **Input Step** (_Avoid_: prompt, chat).
- **E** (`input`, output `outcomes`, `questions`), **F** (`conversation`, launcher, restart, envelope), **G** (input step states, `AWAITING ANSWER`, Iteration limit), **H** (answers Revision), **I** (Recovery wording), **J** (`surt answer`, output, status, JSON), **D** (`0002`, `human_action`, `runtime_info.conversation`), **K** (M6 row).
- **Smoke checklist:** an M6 section.

## Testing Decisions

- **Same seams as M1–M5.** `Project` with the Fake Runtime and real Git for the engine; the binary with TOML Fake scripts for the CLI; launcher argv unit tests and the Unix stub `claude` for the Claude Code runtime. Tests check rows, Revisions, events, `runtime_info`, argv, exit codes and output, never private state.
- **Outputs per Outcome:** both Outcomes of a two-output Agent deliver exactly their outputs; a missing or extra output is rejected; a check on an output the Outcome does not deliver is refused statically.
- **Questions:** every rule of the format, accepted and rejected, through a delivery.
- **Input step:** a Fake agent that asks, then asks again, then delivers; the run stops `AwaitingAnswer` each round, the answers Revision holds every round, an Iteration with questions skips the gate and the next one starts; a delivered output skips the input step; approve/reject refuse while only the input waits; stale and incomplete answers refuse.
- **Conversation:** the second Execution of a `"loop"` step is asked to continue the first's session; an interrupted or failed Execution is never chosen; a rejection continues; the delta holds only changed Inputs, includes a human edit and omits the Agent's own output; the Fake's `transcript_missing` restarts with the full context and spends no Attempt; `"none"` Agents' requests have no continuation; a `"loop"` Agent outside a Loop refuses.
- **Launcher:** argv for fresh, continued and restarted `"loop"` Executions; `"none"` argv unchanged (the existing `never_resumes_or_continues_a_session` and `--no-session-persistence` tests keep holding for them); the stub `claude` replays the probe's `No conversation found` stream.
- **Migration:** as under Database.
- **CLI:** `surt answer` with `--accept`, `--file`, `-m` and a TTY-less refusal; unknown and missing ids; the waiting block, `AWAITING ANSWER` in `status` and `status --json`, the continued and restarted lines; the Template's `run` → answer → approve flow.
- **CI** as before: fmt, clippy `-D warnings`, `cargo test --features test-support` on macOS, Linux and Windows. Coverage stays above 90%.

## Out of Scope

- Questions from any Template Agent other than the Spec Agent.
- An agent waiting mid-turn with its process alive (`--input-format stream-json`), or an interactive `claude` in the developer's terminal.
- Continuing a conversation across steps, Loops or runs.
- Editing questions or answers after they are recorded; a wrong answer is corrected by rejecting the output with a reason.
- Limits on rounds: the Loop contains a human step, so a human decides every round.

## Further Notes

- **The brief changes.** "A new Claude session per execution" was a permanent constraint. ADR 0014 narrows it: the session id stays per Execution and correctness never depends on the transcript, but an opted-in Agent keeps its memory within one Loop. The developer accepted that trade-off on 2026-10-20.
- **Rejections continue the conversation** by the developer's choice: the agent revises with the reasons it already has, instead of rereading its own output as a stranger.
- **Shipping** follows M1–M5: the suite in CI on the three OSes and the M6 section of the manual smoke on macOS.
