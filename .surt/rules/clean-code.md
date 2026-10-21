<clean_code>
  <scope>
    - Apply to changed lines, functions, affected direct callers/callees, and tests.
    - Preserve behavior and project idioms; ignore unrelated legacy code.
    - Exclude generated/vendor code and historical migrations unless requested.
  </scope>

  <priorities>
    Correctness, safety, contracts > clarity, cohesion > numeric goals > micro-optimizations.
    Mandatory comments override the ban.
  </priorities>

  <design>
    - Use precise names, focused functions, guard clauses, and explicit side effects.
    - Avoid duplication, boolean flags, nesting, and speculative abstractions.
    - Soft goals: 20 lines/function, 200 lines/file, 2 nesting levels, complexity 5.
    - When above a goal, simplify or justify in Exceptions.
    - Complexity 10+ requires review: simplify or justify. Never split cohesive code for metrics.
  </design>

  <comments>
    - NEVER add ANY comments inside source code, including during fixes or refactors.
    - Forbidden: //, /* */, ///, //!, TODO, FIXME, NOTE, SAFETY, doc comments, and commented-out code.
    - Do not generate comments via scaffolding or replace old ones.
    - Convey intent through naming, types, functions, tests, and external docs.
    - Preserve existing comments unless asked to edit them.
    - If tooling or contracts require new comments, report the blocker and ask for approval. Never add them silently.
  </comments>

  <values_and_performance>
    - Name domain-policy literals (timeouts, retries, limits); keep obvious local 0/1 inline.
    - Prefer retry(MAX_ATTEMPTS) over retry(3) when 3 defines a policy.
    - Choose algorithms and structures for expected scale and time/space cost.
    - Avoid needless scans, allocations, and I/O. Measure before sacrificing clarity for speed.
  </values_and_performance>

  <report>
    After edits, output exactly three lines. Separate multiple entries with " | ".
    Changed: path - summary
    Checks: command = PASS/FAIL/NOT RUN (reason)
    Exceptions: rule; path:symbol; reason; trade-off. Or "none".
    Include overruns and complexity 10+ reviews. Never claim unrun checks passed.
  </report>
</clean_code>