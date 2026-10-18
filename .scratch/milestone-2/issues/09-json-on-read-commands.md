# 09: `--json` on `status`, `inspect` and `show`

Spec: [Milestone 2](../spec.md) (stories 24–31, 79) · Slice: R3 Full show and `--json` · Plan: [J. Output channels and non-interactive use](../../../docs/architecture/J-cli.md#output-channels-and-non-interactive-use), [J. Reading never writes](../../../docs/architecture/J-cli.md#reading-never-writes)

**What to build:** a script can react to a run's state without parsing text. `--json` on `status`, `inspect` and `show` prints one JSON document on stdout with `"schema": 1`, serialised from the same read views the human output renders, not a second query path.

**Blocked by:** 05 (`surt inspect <execution-id>`), 08 (`show --iter` and `--file`)

**Status:** ready-for-agent

- [x] `status --json`: each Feature's run, derived state, Driver, gates (Revision, origin, SHA-256, Approval) and Tasks (state, Iteration, Attempt, merged SHA, blocked reason, last error, resume command)
- [x] `inspect --json`: timeline events and Execution detail as structured fields, with unreported values absent
- [x] `show --json`: Revision metadata (number, origin, SHA-256, Approval) with each file's content as a UTF-8 string, or `content_base64` when the bytes aren't UTF-8
- [x] Timestamps are RFC 3339 UTC strings, ids are full strings, derived states use the human table's uppercase names
- [x] Refusals and errors keep the human `✖` message on stderr and the exit code, and print nothing on stdout
- [x] Driving commands reject `--json` at the clap level, exit `2`
- [x] A J subsection records one example document per command and the rule that fields are only added within `"schema": 1`
- [x] Binary tests parse each document with `serde_json` and check its fields, including a blocked Task in `status --json`; "reads never write" covers every `--json` call
