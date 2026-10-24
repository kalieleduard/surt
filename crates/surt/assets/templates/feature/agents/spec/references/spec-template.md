<guidance>
This is the standard shape of every `spec.md`. The Task Agent, the Builders and the Reviewers all read the spec after you, and they trace their work back to it by the IDs below, so keep every section, in this order, with these headings.

- IDs are stable: `B-1` for Behaviour, `AC-1` for Acceptance criteria, `D-1` for Decisions. When you revise a spec, keep the IDs of the items you keep, never renumber them, and give new items the next free number.
- Each Behaviour and Acceptance criterion is one observable statement. Write what a user or a test can see, not how the code does it.
- Each criterion names how to verify it, so a Reviewer can check it without guessing.
- Affected areas lists only modules, files, commands or tables you read in the repository. It is where the Task Agent starts planning, so a wrong path costs a whole plan.
- Leave a section with `None.` when it truly has nothing, rather than removing it.
- Fill the `<template>` below; do not copy this `<guidance>` or the angle-bracket placeholders into the spec.
</guidance>

<template>
# <Feature name, as an imperative phrase>

## Problem

<What is missing or wrong today, from the developer's point of view, in two to five sentences. Name who is affected and what they cannot do.>

## Behaviour

- **B-1**: <When ..., Surt/the system ...: one concrete, observable statement.>
- **B-2**: <...>

## Out of scope

- <What this Feature deliberately does not do, and why when it is not obvious.>

## Acceptance criteria

1. **AC-1** (B-1): <A checkable statement.> Verify: <the test, command or observation that proves it.>
2. **AC-2** (B-2): <...> Verify: <...>

## Affected areas

- `<path/or/module>`: <what it does today and what this Feature changes in it.>

## Constraints

- <Project rules, compatibility, platforms, performance or security limits this Feature must respect. Name the rule file when one applies, such as `.surt/rules/sqlite.md`.>

## Decisions

- **D-1**: <The question asked.> → <The answer given.> (<round N | note | chosen by the Spec Agent because ...>)

## Glossary

- **<Term>**: <A new domain term this Feature introduces, in the style of CONTEXT.md. `None.` if it introduces none.>
</template>
