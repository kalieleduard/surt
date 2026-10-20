# 08: Milestone 6 close-out

Spec: [Milestone 6](../spec.md) (stories 28–29, Documentation updates) · Plan: [K. Roadmap](../../../docs/architecture/K-roadmap.md)

**What to build:** the plan says what was built, and the manual smoke proves it with the real Claude Code.

**Blocked by:** 01–07

**Status:** ready-for-human

- [x] Map, CONTEXT.md (**Conversation**, **Input Step**), D, E, F, G, H, I, J, K and L agree with the code; ADR 0014 in L; K gains the M6 row
- [x] The smoke checklist gains an M6 section: two rounds of questions, one rejection that continues, and a restart after deleting the previous transcript under `~/.claude/projects/`
- [x] fmt, clippy `-D warnings` and `cargo test --features test-support` pass (locally on Windows)
- [ ] CI is green on macOS, Linux and Windows
- [ ] The manual check on macOS passes and is recorded under "Smoke results"

## Smoke results

One line per run of the Milestone 6 section of [the smoke checklist](../../../docs/smoke-claude-code.md):

## Comments

- Docs on `m6/08-close-out`. Map: the brief's permanent constraint reads "a new Claude session id per Execution; an Agent may continue the conversation of its previous Execution in the same Loop (ADR 0014)", and the decisions list gains a Milestone 6 line. `CONTEXT.md`: new **Input Step** (_Avoid_: prompt, chat) and **Conversation** (_Avoid_: session, thread); Execution (an Input Step has none; every agent Execution keeps its own session id), Agent Profile, Context Assembler, Input / Output (a round of answers is a human-origin Revision) and the Spec Agent follow.
- A–L against the code: D gains `runtime_info.conversation` with its three shapes (ticket 05 had not added it) and the `answer.recorded` event with its data; `human_action = 'answer'`, `step_run.kind = 'input'` and migration `0002` were already there. E's `feature.toml` and step table gain `answer-spec`, the agent notes name `outputs.<name>.outcomes` and `[profile] conversation`, a "How the spec Loop behaves" list, and static check 10 (`ConversationOutsideLoop`). F's Template table gives the Spec Agent `done`/`questions`, `spec-questions` and `spec-answers?`; the conversation, launcher, restart, envelope, per-Outcome outputs and `questions` sections were complete. G says which Execution a conversation continues. H's Execution directory shows `context.md` and `context-continued.md`. I. Recovery already said "for correctness"; I. Cleanup names the context files, the `conversation` part of `runtime_info`, and Claude Code's transcripts, which no ending and no `surt delete` touches. J gains `inspect`'s `conversation` line and its `reported.conversation` JSON, and `run`/`continue` drive to an input step. K: the M6 row and a "Milestone 6 is implemented" paragraph. L: ADR 0014. A. Process Model, B and C name the input step and `surt answer` where they named only Approval Gates (B's `AtGate` sketch now carries `gates: Vec<Gate>`).
- Spec vs code, documented as the code is: the restart line is `↻ <step> restarted (iteration I): <reason>`, printed after `● <step> continued (iteration I, same conversation)`, not a `●` line in place of it; the terminal prompt of `surt answer` prints `why:` (and `options:`) and puts the recommendation in brackets on its own line, not after the question.
- Left as found (not M6): H's Execution directory tree shows inputs as `inputs/spec.md`, while the Context Assembler writes each Input under `inputs/<name>/<rel_path>`; ADR 0010's "a new Claude session with no memory" (history of the decision, and still true of the Builder).
- Smoke: the M6 section asks a deliberately vague request, answers round 1 on the terminal (Enter on one recommendation, one typed answer), round 2 with `-m` when the Agent asks again (the result line records the number of rounds, since a second round cannot be forced), rejects once with a continued conversation, then deletes the latest spec Execution's transcript (`~/.claude/projects/*/<session-id>.jsonl`, session id from `surt inspect`) before a second rejection and expects `↻ spec restarted (iteration M): transcript_missing`, then approves. Sections 1–8 and M2–M5 answer with `--accept` if the Spec Agent asks first.
- Checks (Windows, `CARGO_INCREMENTAL=0`): `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets --features test-support -- -D warnings` clean; `cargo test --workspace --features test-support` 1000 passed, 0 failed (surt unit 161, surt CLI 167, surt-engine 672, doc-tests 0). Open: CI on macOS, Linux and Windows (needs the branch pushed) and the manual macOS smoke (M6 section of `docs/smoke-claude-code.md`).
