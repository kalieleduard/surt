<domain_language>
  <scope>
    Apply to changed identifiers, user-facing messages, refusal text, tests, documents and agent instructions.
  </scope>

  <why>
    CONTEXT.md defines Surt's ubiquitous language, and each term lists the synonyms to avoid. Using one word per concept keeps the code, the CLI output and the docs searchable and stops two names from drifting into two meanings.
  </why>

  <usage>
    - Read CONTEXT.md before naming a type, function, field, message or test that refers to a domain concept.
    - Use the defined term (Workflow Run, Step Run, Execution, Attempt, Iteration, Outcome, Driver, Intention, Blocked...) and never a word listed under its _Avoid_.
    - Keep the distinctions the glossary draws: an Attempt follows a technical failure, an Iteration follows a decision; an Interruption keeps the run, a Cancellation ends it.
    - When a change introduces a concept the glossary lacks, add its entry to CONTEXT.md in the same change, in the existing format, and name it in the Changed line of the report.
  </usage>
</domain_language>
