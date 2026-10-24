<guidance>
This is the standard shape of every Task document (`tasks/<key>.md`). The Builder gets only this document and the spec, and the Reviewer checks the commit against it, so it must stand on its own and trace back to the spec.

- Refer to the spec by its IDs: Behaviour `B-n`, Acceptance criteria `AC-n`, Decisions `D-n`.
- Number this Task's own criteria `<key>-1`, `<key>-2`... (such as `A-1`). Each one names the spec criterion it serves. The same statements, in the same order, go into the Task's `acceptance_criteria` in `manifest.json`, prefixed with their ID.
- Name only files you read in the repository, or mark them `(new)`.
- Say what to change, not the code to write: the Builder reads the code around the change and follows its conventions.
- Leave a section with `None.` when it truly has nothing, rather than removing it.
- Fill the `<template>` below; do not copy this `<guidance>` or the angle-bracket placeholders into the document.
</guidance>

<template>
# <key>: <Task title>

## Goal

<One paragraph: what this Task delivers and why.> Covers: <AC-n, AC-m>.

## Context

- `<path>`: <what this file does today that matters for the Task.>
- Depends on: <the keys in `needs` and what each one already delivers, or `None.`>

## Changes

1. `<path>`: <what to change and the behaviour it must have afterwards.>
2. `<path>` (new): <what the new file holds.>

## Tests

- <The behaviour to test> in `<test path>`: <the cases, including the error and edge cases.>
- Run: <the commands that must pass, as the project rules name them.>

## Rules

- `<.surt/rules/file.md>`: <the part of it that matters most for this Task.>

## Out of scope

- <What not to do here, including work that belongs to another Task (name its key).>

## Done when

- **<key>-1** (AC-n): <A checkable statement.>
- **<key>-2** (AC-m): <...>
</template>
