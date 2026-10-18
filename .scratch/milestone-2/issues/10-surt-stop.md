# 10: `surt stop`

Spec: [Milestone 2](../spec.md) (stories 32–39) · Slice: R4 Stop · Plan: [J. Stop and cancel](../../../docs/architecture/J-cli.md#stop-and-cancel), [A. External control](../../../docs/architecture/A-process-model.md#external-control)

**What to build:** a developer can pause a run driving in a tmux pane, a CI job or another terminal, and keep it resumable. `surt stop <feature>` makes the live Driver perform the same Interruption as a first Ctrl-C: agents stop gracefully, a running Operation finishes, and the Driver exits `130`. `surt stop` waits until that Driver has stopped and prints the resume command.

**Blocked by:** 02 (Events carry their details)

**Status:** ready-for-agent

- [ ] `Project::request_stop` writes `stop_requested_at/by` and a `run.stop_requested` event (with who asked) in one transaction, only when a Driver is live (liveness decided as `status` decides it)
- [ ] With no live Driver it refuses with the run's derived state; the CLI exits `3`
- [ ] The Driver reads the request on its next tick and turns it into the graceful `StopSignal`, sharing the Ctrl-C path; it ends `Interrupted` and its terminal says it was stopped by `surt stop` from another terminal, with the resume command
- [ ] Stopped Executions don't count toward `max_attempts`
- [ ] Every driving command clears `stop_requested_at/by` in the transaction where it records itself as the Driver, so a stale request never stops the next Driver
- [ ] `surt stop` asks no confirmation, reports the Driver's pid and how many Executions it is stopping, waits for the lock with no timeout, then prints the resume command; Ctrl-C on `surt stop` leaves the request in place
- [ ] The stop request appears in `surt inspect <feature>`
- [ ] `Project` tests: refusal without a Driver, leftover request cleared on lock, no Attempt spent
- [ ] Binary test: a stop from a second process against a Fake Execution that waits until stopped ends the Driver with `130`, and `surt start` resumes the run
- [ ] J records that the wait has no timeout
