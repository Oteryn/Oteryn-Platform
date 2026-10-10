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

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Packet B3 (CREATE-OFF-1) of the removal plan under `docs/agents/programs/`.

Character creation returns the typed unavailable state after input validation. The legacy creator container binding and the account binding lookup are removed, so no write and no legacy connection is attempted.

## Acceptance criteria

- [x] `CreateCharacter` validates input, then throws `CharacterCreationUnavailable`; it has no gateway dependency and reads no account binding.
- [x] The creator gateway container binding is removed from `AppServiceProvider`.
- [x] `CharacterCreationTest` asserts the unavailable response, an unbound gateway and no opened legacy connection.
- [x] The legacy token ratchet passes with falling runtime and tests counts.
- [ ] Actor-to-result browser path asserts the unavailable submission result.
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
  - acceptance specs under scripts/acceptance/tests need an owned-path expansion
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-10T23:30:00Z
head: UNKNOWN
branch: head branch of PR 1492
pr: 1492
status: blocked
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
  marker: acceptance browser job
  evidence: specs full-acceptance, player-journey, character-boundaries and security-boundary-critical expect a created character at 023bb5ad
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
  - command: python tools/agents/checkpoint.py --require-checkpoint
    result: PASS
    evidence: task checkpoint validated
  - command: php -l on changed PHP files
    result: PASS
    evidence: no syntax errors
  - command: product runtime E2E
    result: FAIL
    evidence: the acceptance browser job runs the real create-character submission and fails because four specs under scripts/acceptance/tests still expect a created character; they are outside this packet and need an owned-path expansion to assert the unavailable result
blockers:
  - acceptance specs under scripts/acceptance/tests (full-acceptance, player-journey, character-boundaries, security-boundary-critical) expect character creation to succeed; owned paths must be extended to update them
next_action: Extend owned paths to the four acceptance specs, assert the unavailable submission result, then re-read exact-head CI.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
