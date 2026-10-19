# 07: `surt logs -f` follows a live run

Spec: [Milestone 2](../spec.md) (stories 11, 16) · Slice: R2 Logs · Plan: [J. `surt logs`](../../../docs/architecture/J-cli.md#surt-logs)

**What to build:** a developer can watch a live run from a second terminal. `surt logs … -f` follows every running agent of the run, multiplexed with the source prefix, and picks up new Executions (the next Attempt, the next Task) as they start, until the run's Driver stops. In a script it never waits forever.

**Blocked by:** 06 (`surt logs`)

**Status:** resolved

- [x] `-f` polls the files and the engine's Execution list at bounded intervals; no file-watcher dependency
- [x] New Executions are picked up as they start
- [x] It stops once the run's Driver is gone and every file is drained, exit `0`
- [x] With no live Driver at start, it prints what exists and exits `0`
- [x] Works with `--raw` and with a Task or Execution target
- [x] Binary test: `logs -f` follows two Tasks of a live Fake run to the end; process waits are bounded polling, no fixed sleeps

## Comments

- 2026-10-18 (orchestrator): Implemented and green on macOS. Merged into `integration/milestone-2` at fded7c3; code-review fixes at 2c96dfe. Resolved.
