# 08: `surt show --task --iter` and `--file`

Spec: [Milestone 2](../spec.md) (stories 20, 21, 22, 23) · Slice: R3 Full show and `--json` · Plan: [J. `surt show`](../../../docs/architecture/J-cli.md#surt-show)

**What to build:** a developer can read any Revision and any one file of it. `surt show <feature> <artifact> --task <key> --iter N` prints the Revision produced in Iteration `N` of that Task, so review 1's feedback can be compared with review 2's. `surt show <feature> manifest --file tasks/B.md` prints one file of a multi-file Revision byte for byte.

**Blocked by:** 01 (Run selector and `--run`)

**Status:** ready-for-agent

- [ ] `--iter N` with `--task` prints that Iteration's Revision; an Iteration that produced none refuses
- [ ] `--iter` without `--task` is invalid usage at the clap level, exit `2`
- [ ] `--file <path>` prints that file's bytes unchanged
- [ ] An unknown `--file` path refuses and lists the files of that Revision, exit `3`
- [ ] The flags combine with `--run`
- [ ] Binary tests for each flag and refusal; "reads never write" covers them
