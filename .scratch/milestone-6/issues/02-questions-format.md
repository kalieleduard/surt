# 02: The `questions` format

Spec: [Milestone 6](../spec.md) (Format `questions`) · Plan: [F](../../../docs/architecture/F-agent-contract.md), [H](../../../docs/architecture/H-artifact-storage.md)

**What to build:** a Surt-owned artifact format an Agent delivers when it has questions, validated like the Item List so a bad round never reaches the developer.

```json
{ "questions": [
  { "id": "login-id", "question": "Log in by e-mail only, or by username too?",
    "why": "changes the users table", "recommended": "e-mail only",
    "options": ["e-mail only", "e-mail or username"] } ] }
```

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] `Format::Questions` (`format = "questions"`) parses on inputs and outputs; `json_schema` on it is `MisplacedJsonSchema`
- [ ] Delivery validation: 1 to `MAX_QUESTIONS = 5` entries; `id` follows the Item List key rules; `question`, `why`, `recommended` non-empty; ids unique; `options`, when present, 2+ unique non-empty strings containing `recommended`; failures are a `QuestionsProblem` turned into a `DeliveryRejection`
- [ ] A typed read side (`Questions`, `Question`) that later tickets use to print and answer them
- [ ] Tests for every rule, accepted and rejected, through a delivery
- [ ] F and H document the format
