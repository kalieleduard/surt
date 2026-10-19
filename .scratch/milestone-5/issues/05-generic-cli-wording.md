# 05: The CLI's wording and JSON fit any workflow

Spec: [Milestone 5](../spec.md) (story 27) · Plan: [J](../../../docs/architecture/J-cli.md)

**What to build:** nothing the CLI prints names the feature workflow. Arguments are subjects, item lists are titled by their `per_item` step, gates by their artifact.

**Blocked by:** 04

**Status:** resolved

- [x] Positional arguments are `<subject>`; `--task` becomes `--item` on `show`, `inspect` and `logs`; refusals say subject and item
- [x] Status: header `SUBJECT`; gate rows labelled by artifact; item-file counts from the Revision's files for any Item List gate; items section titled by the `per_item` step key; "merged" stays for integrated items
- [x] Progress says `item <key>`; the delivered line names the step and prints only when an integration branch exists
- [x] JSON keys `features` → `subjects`, `feature` → `subject`, `tasks` → `items`, `task` → `item` in status, inspect and show
- [x] `cancel`/`delete` questions and summaries say items and item branches
- [x] CLI tests updated; J's output examples and JSON keys follow

## Comments

- Resolved on `m5/05-generic-cli-wording`. Engine: `RevisionSummary.item_list: Option<ItemListSummary { items, item_files }>` is set when the gate's artifact is an Item List, decided by the producing Agent's output `format = "item_list"` (`GateArtifacts.item_list` from `Frozen::producer`), with counts read from the Revision's index by `listed_items` (items, and the distinct item files they reference). `RunStatus.item_step: Option<String>` is the key of the root definition's first `per_item` step. `status::item_files` (`files - 1`) and `TASK_STEP` are gone; `status/detail/tasks.rs` is `status/detail/items.rs`, titled by `item_step` (fallback `items`). `status --json` gained `item_step` and per gate Revision `item_list` (additions, schema stays 1).
- CLI: every positional is `subject` (clap shows `<SUBJECT>`); `show --task` is `--item` (`surt show … --task` exits `2`). `inspect` and `logs` never had a `--task` flag: their optional positional is now `[<item>]` and stays positional, so `surt inspect add-login B` keeps working. JSON keys `features`/`feature`/`tasks`/`task` became `subjects`/`subject`/`items`/`item` in status, inspect (timeline, events, Execution detail and its inputs) and blocked entries; the Execution detail's text label `feature` is `subject`.
- The delivered line reads `✔ delivered — <branch> @ <tip>` and names no step: no clean generic source says which step delivered (the snapshot Operation moves the branch, the Template's ship Agent only writes `delivery`), so the step is left out rather than guessed. It still prints only when the run has an Integration Branch. Spec/J say "names the step"; J now documents the unnamed form.
- Tests: CLI unit and integration expectations, test names and helpers (`subject_name`, `with_a_blocked_item`, `cancelled_with_a_blocked_item`, `second_subject`, `item_step`, `of_item`) follow; `feature` stays where it is the Template's workflow (`run_feature`, `Workflow("feature")`) and `tasks()`/`tasks_needing()` in `scripts.rs` script the Template's `tasks` Agent. New: engine `drive/item_list_status.rs` (item list counts, `item_step`, none for a plain artifact), `status/tests/items.rs` (title by `per_item` key, counts from the engine not from files, JSON keys), `usage.rs` (`--task` refused). `follow.rs`, `interrupt.rs`, `claude.rs` and the unix parts of `cancel.rs` were updated by reading them; they do not compile on Windows.
- For 06: `.surt/features/…` still appears in `driving.rs` (awaiting approval line), `decide.rs` (recreated file) and `refusal/edits.rs`, plus the CLI tests that join `features`; child branches `surt/task/…` appear in `delete` tests. `StatusScope::Feature` is used by `status.rs`, `stop.rs` and `continue_run/new_run.rs`.
- For 07: J follows. A-git-integration, B, I-cleanup, K and `docs/smoke-claude-code.md` still mention `--task`, `"features"` or `shipper delivered`; the reject example `-m "task C grande demais"` in J is user text and was kept.
