# 06: `surt logs` for a feature, a Task or an Execution

Spec: [Milestone 2](../spec.md) (stories 9, 10, 12, 13, 14, 15, 17, 77) · Slice: R2 Logs · Plan: [J. `surt logs`](../../../docs/architecture/J-cli.md#surt-logs), [I. Logs](../../../docs/architecture/I-cleanup.md#logs)

**What to build:** a developer can skim what agents did without reading JSON. `surt logs <feature>` prints the latest Execution of each Task; `surt logs <feature> <task>` and `surt logs <execution-id>` narrow it. Each `stream.jsonl` line renders to zero or one display line: a tool call becomes `<Tool> <main argument>`, an assistant message becomes its first line in quotes, an accepted delivery becomes `✔ delivered …`. `surt status` stops printing a raw log path and points at the commands instead.

**Blocked by:** 05 (`surt inspect <execution-id>`)

**Status:** ready-for-agent

- [ ] The engine exposes each Execution's log directory path and the Execution selection (latest per Task, a Task, an Execution); the CLI opens and renders the files
- [ ] The renderer is a pure function in the `surt` crate; tool results, unknown event types and unknown fields render nothing and never error
- [ ] `output.log` and `stderr.log` lines pass through; process-running Operations (Checks, the integration re-check) show combined output with one header per Check
- [ ] The source prefix (`B build#2 │`) appears only when more than one source is printed
- [ ] `--raw` prints the original `stream.jsonl` lines untouched
- [ ] A missing log directory says so and points at `surt inspect <id>`
- [ ] `--run N` selects the run
- [ ] A blocked step in `surt status` prints `inspect: surt inspect <execution>` and `logs: surt logs <execution>` instead of a file path
- [ ] Renderer tests over the real Claude Code fixtures plus synthetic unknown lines; binary tests for the three targets, `--raw` and the missing directory; "reads never write" covers `logs`
