---
task_id: OTERYN-20261010-create-off
governing_issue: 1474
required_reads: []
search_first:
  - app/Characters/Actions/CreateCharacter.php
optional_reads: []
---

# OTERYN-20261010 character creation off

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Packet B3 (CREATE-OFF-1) of `docs/agents/programs/DECANARY_REMOVAL_PLAN.md`.

Character creation returns the typed unavailable state after input validation. The legacy creator container binding and the account binding lookup are removed, so no write and no legacy connection is attempted.

## Acceptance criteria

- [x] `CreateCharacter` validates input, then throws `CharacterCreationUnavailable`; it has no gateway dependency and reads no account binding.
- [x] The creator gateway container binding is removed from `AppServiceProvider`.
- [x] `CharacterCreationTest` asserts the unavailable response, an unbound gateway and no opened legacy connection.
- [x] The legacy token ratchet passes with falling runtime and tests counts.
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - app/Characters/**
  - app/Providers/AppServiceProvider.php
  - tests/Feature/Characters/CharacterCreationTest.php
  - docs/agents/tasks/active/OTERYN-20261010-create-off.md
  - docs/agents/tasks/archive/OTERYN-20261010-create-off.md
modules:
  - characters
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-10T23:00:00Z
head: UNKNOWN
branch: claude/decanary-create-off-1-20261010
pr: 1491
status: validating
context_routes:
  - characters
owned_paths:
  - app/Characters/**
  - app/Providers/AppServiceProvider.php
  - tests/Feature/Characters/CharacterCreationTest.php
  - docs/agents/tasks/active/OTERYN-20261010-create-off.md
  - docs/agents/tasks/archive/OTERYN-20261010-create-off.md
proven:
  - Legacy token ratchet passes locally; runtime 497 to 488, tests 712 to 701.
derived:
  - The existing controller already maps the unavailable exception to a bounded form error.
unknown:
  - PHP suite result; composer dependencies could not be installed in the authoring sandbox, so CI is the first run.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - app/Characters/Actions/CreateCharacter.php
  - app/Providers/AppServiceProvider.php
  - tests/Feature/Characters/CharacterCreationTest.php
  - docs/agents/tasks/active/OTERYN-20261010-create-off.md
validation:
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: counts at or below baseline
  - command: php -l on changed PHP files
    result: PASS
    evidence: no syntax errors
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: behavior is covered by the feature test in CI
blockers:
  - none
next_action: Read exact-head CI and hand the review packet to the lead.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
