# 07: Milestone 5 close-out

Spec: [Milestone 5](../spec.md) · Plan: [K](../../../docs/architecture/K-roadmap.md)

**What to build:** the docs say one thing, and the smoke covers M5.

**Blocked by:** 06

**Status:** ready-for-human

- [x] `CONTEXT.md`: Workflow Definition and Agent are files under `.surt/`; **Template** and **Run Input** added; **Agent Override** removed; the feature section describes the Template
- [x] ADR 0011 links ADR 0013; the map's "project-specific workflows" out-of-scope line is marked lifted by M5
- [x] A consistency pass over A–L for `surt feature`, `plan`, `start`, "built-in", `.surt/features/` and `--task`
- [x] K: the M5 row says implemented
- [x] `docs/smoke-claude-code.md` gains an M5 section: `init --template feature`, an edited Reviewer instruction seen in `inspect`, `run`/`approve`/`continue` to the end
- [x] fmt, clippy `-D warnings` and `cargo test --features test-support` pass (locally on Windows)
- [ ] CI is green on macOS, Linux and Windows
- [ ] The manual check on macOS passes and is recorded under "Smoke results"

## Smoke results

One line per run of the Milestone 5 section of [the smoke checklist](../../../docs/smoke-claude-code.md):

## Comments

- Docs on `m5/07-close-out`. `CONTEXT.md`: Workflow Definition and Agent are files under `.surt/`, frozen at root-run creation; new **Template** (_Avoid_: built-in workflow), **Run Input** and **Item Branch** (the child Run Branch `surt/item/…`; it replaces Task Branch everywhere, since the CLI already says "item branches"; Task stays the feature Template's word for its items); **Agent Override** removed; the feature section is "The feature Template". ADR 0011 opens with "Amended by ADR 0013"; ADR 0007 gains a note that M5 renamed the trailer `Surt-Item` and the message `Merge item` (ADRs had never been amended before; a note keeps the decision text intact). The map's out-of-scope line is marked lifted by M5 without rewriting it.
- Consistency pass over A–L: `surt feature`/`plan`/`start` → `surt run feature --input …`/`surt continue`, `--new-run` examples → `surt continue <subject> --new-run`, progress mocks `task X` → `item X`, `✔ shipper delivered` → `✔ delivered`, built-in/Catalog statements → Template and project files, `--base` → `--input base=…`. A-git's "✔ shipped" status mock (never implemented) became the real delivered/succeeded lines. K: M5 row implemented, a "Milestone 5 is implemented" paragraph, v1 now ends with M5, and the map's out-of-scope sentence names the M5 exception.
- Intentionally left: the historical milestone specs and tickets under `.scratch/milestone-1..4` and the map's decision summaries; K's Milestone 1 goal, mock, command table, S2 slice and file tree (they describe M1; a note after each points at the M5 CLI and layout); ADR 0011's body, ADR 0013's considered options and ADR 0006's "built-in agent id" argument (history of the decisions); ADR 0007's `Surt-Task` in its considered options (with the note); F's "Built-in tools" and "built-in system prompt" (Claude Code's); F's "there are no built-in Agents" (true); J's reject example `-m "task C grande demais"` (user text); the `"title": "Built-in …"` of `commit`, `feedback`, `delivery` and `manifest` schemas in `docs/architecture/schemas/` and in the Template assets (changing Template bytes is out of this docs ticket; a follow-up can retitle both).
- Smoke: `surt inspect <execution>` prints no instructions, so M5.2 takes the Reviewer's Execution id and role from `inspect`, reads its frozen `instructions.md` from the agent snapshot with `sqlite3`, and sees the effect in `surt show … feedback --item <key>`; a line added after creation must appear in neither. Sections 1–8 and M2–M4 use the M5 CLI (M4.1 edits `.surt/agents/builder/agent.toml` in place; M4.3's second `continue` on a kept Subject now refuses with exit 3 and removes nothing). The M5 section's custom workflow, the init rerun and the snapshot query were tried with the debug binary and the Fake Runtime in a scratch repository.
- Checks (Windows, `CARGO_INCREMENTAL=0`): `cargo fmt --all -- --check` clean; `cargo clippy --workspace --all-targets --features test-support -- -D warnings` clean; `cargo test --workspace --features test-support` 899 passed, 0 failed (surt unit 142, surt CLI 153, surt-engine 604, doc-tests 0). Open: CI on macOS, Linux and Windows (needs the branch pushed) and the manual macOS smoke (M5 section of `docs/smoke-claude-code.md`).
