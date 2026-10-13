---
task_id: OTERYN-20261013-optional-ui-skills
governing_issue: 1602
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20261013-optional-ui-skills

## Goal

Governing Issue: https://github.com/Oteryn/Oteryn-Platform/issues/1602

Install project-local design, UI-review and browser-testing skills behind the task-scoped user grant in root AGENTS.md. Select the smallest relevant set after permission; keep required checks unaffected.

## Acceptance criteria

- [x] Explicit grant, continuation, revocation, named-skill scope and worker handoff rules.
- [x] Pinned sources, SHA-256 provenance and retained Apache-2.0/MIT licenses.
- [x] Laravel/Blade-specific wrappers and browser helper limits.
- [ ] Focused validation, exact-head CI and governed integration.

## Ownership

```yaml
owned_paths:
  - AGENTS.md
  - .agents/skills/**
  - docs/agents/tasks/active/OTERYN-20261013-optional-ui-skills.md
  - docs/agents/tasks/archive/OTERYN-20261013-optional-ui-skills.md
modules:
  - agent-governance
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Game issue 2080; separately authorized in the same owner chat
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-13T17:53:59.601261Z
head: UNKNOWN
branch: codex/optional-ui-skills-20261013
pr: none
status: implementing
context_routes:
  - agent-governance
owned_paths:
  - AGENTS.md
  - .agents/skills/**
  - docs/agents/tasks/active/OTERYN-20261013-optional-ui-skills.md
  - docs/agents/tasks/archive/OTERYN-20261013-optional-ui-skills.md
proven:
  - Sources are pinned and licenses retained.
derived: []
unknown:
  - Actual client discovery and model behavior are NOT_EVALUATED.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - AGENTS.md
  - .agents/skills/**
  - docs/agents/tasks/active/OTERYN-20261013-optional-ui-skills.md
validation:
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: No runtime UI, dependency, security or application code changes.
blockers:
  - none
next_action: Run focused checks and publish the authoring branch.
```

## Review and evidence

Independent AI review not selected: optional-aid instructions narrow selection without changing autonomous write/integration authority, runtime authorization or workflow gates. Self-review and deterministic checks required. The catalog distinguishes static validation from unperformed client/model trials.

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: Await exact-head qualification and governed integration.
source_branch_evidence: pending
```
