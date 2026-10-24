<clean_code>
  <scope>
    - Apply to changed lines, functions, affected direct callers/callees, and tests.
    - Preserve behavior and project idioms; ignore unrelated legacy code.
    - Exclude generated/vendor code and historical migrations unless requested.
  </scope>

  <priorities>
    Correctness, safety, contracts > clarity, cohesion > numeric goals > micro-optimizations.
    Mandatory comments (see comments) override the ban.
  </priorities>

  <design>
    - Use precise names, focused functions, guard clauses, and explicit side effects.
    - Avoid duplication, boolean flags, nesting, and speculative abstractions.
    - Make the smallest change that solves the task; leave unrelated refactors and features out.
    - Soft goals: 20 lines/function, 200 lines/file, 2 nesting levels, complexity 5.
    - When above a goal, simplify or justify in Exceptions.
    - Complexity 10+ requires review: simplify or justify. Never split cohesive code for metrics.
  </design>

  <comments>
    <why>Comments drift from the code they describe and reviewers cannot test them; this project keeps intent in names, types, functions and tests instead.</why>
    - Write no comments in source code, including during fixes or refactors.
    - This covers //, /* */, ///, //!, TODO, FIXME, NOTE, SAFETY, doc comments, and commented-out code.
    - Do not generate comments via scaffolding or replace old ones.
    - Convey intent through naming, types, functions, tests, and external docs.
    - Preserve existing comments unless asked to edit them.
    - If tooling or contracts require a new comment, report the blocker and ask for approval instead of adding it.
  </comments>

  <values_and_performance>
    - Name domain-policy literals (timeouts, retries, limits); keep obvious local 0/1 inline.
    - Prefer retry(MAX_ATTEMPTS) over retry(3) when 3 defines a policy.
    - Choose algorithms and structures for expected scale and time/space cost.
    - Avoid needless scans, allocations, and I/O. Measure before sacrificing clarity for speed.
  </values_and_performance>

  <report>
    <why>One shared format lets a reviewer compare every change the same way and see at once what was verified.</why>
    This is the single report format for every rule in .surt/rules/. After edits, output exactly three lines; separate multiple entries with " | ".
    Changed: path - summary
    Checks: command = PASS/FAIL/NOT RUN (reason)
    Exceptions: rule; path:symbol; reason; trade-off. Or "none".
    Include overruns and complexity 10+ reviews. Report a check as PASS only when you ran it and saw it pass.
    An agent that finishes with a structured result writes these lines in its final message, before that result.
  </report>
</clean_code>
