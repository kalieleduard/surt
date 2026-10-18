# 04: `surt inspect <feature>` and `surt inspect <feature> <task>`

Spec: [Milestone 2](../spec.md) (stories 1, 2, 5, 7, 8) · Slice: R1 History · Plan: [J. `surt inspect`](../../../docs/architecture/J-cli.md#surt-inspect)

**What to build:** a developer can see how a run got where it is without opening SQLite. `surt inspect <feature>` prints the run's timeline (creation and Base, Revisions published, Approvals with reasons, Driver starts, stops and recoveries, cleanup). `surt inspect <feature> <task>` prints one Task's timeline (Iterations and why each started, Step Runs, every Attempt with its error code and message, blocks and unblocks). It reads only SQLite, so it works after cleanup and after log files are gone.

**Blocked by:** 01 (Run selector and `--run`), 02 (Events carry their details)

**Status:** ready-for-agent

- [x] `Project::inspect` returns a typed timeline view built from `event` and related rows; the engine names no phase concept, the `surt` crate prints Task keys
- [x] Each line carries its time; events from an earlier day also carry their date
- [x] A missing `event.data` field renders as the bare kind; a run with only `{}` events (Milestone 1 data) still renders from kinds and times
- [x] `--run N` selects the run
- [x] An unknown Task key refuses and lists the run's Tasks
- [x] Works after cleanup with the log directories deleted
- [x] Binary test: the brief's E2E (rejected review, new Builder Iteration) is explained by `surt inspect`; "reads never write" covers `inspect`
