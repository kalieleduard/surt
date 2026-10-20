# 07: `surt answer`, and the Template's Spec Agent asks

Spec: [Milestone 6](../spec.md) (CLI, The feature Template) · Plan: [J. CLI Surface](../../../docs/architecture/J-cli.md)

**What to build:** the developer sees the questions where the run stops, answers them with `surt answer`, and the feature Template's Spec Agent uses all of M6.

```
? spec-questions rev 1 awaiting answers (round 1)
  1. Log in by e-mail only, or by username too?
     why: changes the users table
     recommended: e-mail only
  answer: surt answer 20261020-add-login
```

**Blocked by:** 01, 02, 04, 05

**Status:** resolved

- [x] `surt answer <subject> [--accept | --file <path> | -m <text>]` (mutually exclusive, exit `2`); TTY prompt per question with the recommendation as default; no flag and no TTY is exit `2`; `--file` TOML `"<id>" = "<answer>"`; refusals map to exit `3`; prints `✔ answered <artifact> rev N by <actor>` and drives as `surt continue`
- [x] `awaiting` prints the input block; progress prints `● <step> continued (iteration I, same conversation)` / `● <step> restarted (iteration I): <reason>`; `status` shows `AWAITING ANSWER` and the input step; `status --json` gate entries gain `kind`
- [x] Template: `feature.toml`'s `spec-loop` with `answer-spec`; the Spec Agent's Outcomes, outputs, `spec-answers` input, `conversation = "loop"` and new instructions (good questions, **Decisions**); the Template tests follow
- [x] CLI tests: each `answer` form, the refusals, the waiting block, status and JSON, and the Template's `run` → answer → approve flow with a Fake script
- [x] J documents the command and output
