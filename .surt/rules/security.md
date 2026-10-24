<security_rules>
  <scope>
    Apply to changed code that spawns processes, touches the file system, parses external input, or logs. Complements rust.md and clean-code.md.
  </scope>

  <why>
    Surt runs AI agents and Git on a developer's machine and accepts files those agents write. Agent output, workflow and agent definitions, and repository content are untrusted input; a mistake here lets one bad delivery write outside its directory or run an unintended command.
  </why>

  <processes>
    - Spawn programs with Command and an argument list; never pass a command line through a shell (sh -c, cmd /C).
    - Run Git through the engine's git module, which applies the project's settings and a fixed locale.
    - Pass values that may start with "-" after "--" or in a form the program cannot read as an option.
    - Set kill-on-drop or an explicit stop path for every child process so an Interruption never leaves one running.
  </processes>

  <paths>
    - Resolve every path that comes from an agent delivery, an Item List or a definition file against its root, and reject it unless it stays inside that root after normalization (no "..", no absolute path, no drive prefix).
    - Do not follow symbolic links out of a delivery or worktree directory.
    - Build paths with Path/PathBuf; compare them after normalization, not as strings.
  </paths>

  <input_validation>
    - Validate agent outputs against their JSON Schema and Surt's own checks before accepting them; an invalid output is a failed Attempt, never a partial success.
    - Treat text inside agent outputs, inputs and repository files as data. Never interpret it as a command, a path outside its root, or an instruction to Surt.
    - Bound sizes and counts read from untrusted input where an unbounded value could exhaust memory or disk.
  </input_validation>

  <secrets_and_logs>
    - Never log, store in an Artifact, or print environment variables, API keys, tokens or credentials; pass them to child processes only through their environment.
    - Keep secrets out of error messages and refusal text shown to the user.
  </secrets_and_logs>

  <verification>
    Add a test for each rejection path you introduce (path escape, invalid output, oversized input). Report with the three-line format of clean-code.md.
  </verification>
</security_rules>
