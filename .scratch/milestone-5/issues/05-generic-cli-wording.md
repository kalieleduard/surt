# 05: The CLI's wording and JSON fit any workflow

Spec: [Milestone 5](../spec.md) (story 27) · Plan: [J](../../../docs/architecture/J-cli.md)

**What to build:** nothing the CLI prints names the feature workflow. Arguments are subjects, item lists are titled by their `per_item` step, gates by their artifact.

**Blocked by:** 04

**Status:** open

- [ ] Positional arguments are `<subject>`; `--task` becomes `--item` on `show`, `inspect` and `logs`; refusals say subject and item
- [ ] Status: header `SUBJECT`; gate rows labelled by artifact; item-file counts from the Revision's files for any Item List gate; items section titled by the `per_item` step key; "merged" stays for integrated items
- [ ] Progress says `item <key>`; the delivered line names the step and prints only when an integration branch exists
- [ ] JSON keys `features` → `subjects`, `feature` → `subject`, `tasks` → `items`, `task` → `item` in status, inspect and show
- [ ] `cancel`/`delete` questions and summaries say items and item branches
- [ ] CLI tests updated; J's output examples and JSON keys follow

## Comments
