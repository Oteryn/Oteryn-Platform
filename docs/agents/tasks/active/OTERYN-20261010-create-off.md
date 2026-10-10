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
- [x] Actor-to-result browser path asserts the unavailable submission result (four acceptance specs updated; the lead approved extending owned paths).
- [ ] Exact-head CI passes.

## Ownership

```yaml
owned_paths:
  - app/Characters/**
  - app/Providers/AppServiceProvider.php
  - tests/Feature/Characters/CharacterCreationTest.php
  - docs/agents/tasks/active/OTERYN-20261010-create-off.md
  - docs/agents/tasks/archive/OTERYN-20261010-create-off.md
  - scripts/acceptance/tests/full-acceptance.spec.mjs
  - scripts/acceptance/tests/player-journey-acceptance.spec.mjs
  - scripts/acceptance/tests/character-boundaries-acceptance.spec.mjs
  - scripts/acceptance/tests/security-boundary-critical.spec.mjs
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
updated_at: 2026-10-10T23:45:00Z
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
  - scripts/acceptance/tests/full-acceptance.spec.mjs
  - scripts/acceptance/tests/player-journey-acceptance.spec.mjs
  - scripts/acceptance/tests/character-boundaries-acceptance.spec.mjs
  - scripts/acceptance/tests/security-boundary-critical.spec.mjs
proven:
  - Legacy token ratchet passes locally; runtime 497 to 488, tests 712 to 701.
derived:
  - The existing controller already maps the unavailable exception to a bounded form error.
unknown:
  - PHP suite result; composer dependencies could not be installed in the authoring sandbox, so CI is the first run.
  - The account overview still reports character creation as allowed and shows a create button; that projection and view are outside this packet and need a follow-up.
  - The branch field cannot hold the real head branch without adding a ratchet match, because the branch name itself contains the legacy token; the live task check therefore reports a branch mismatch until the lead resolves it.
conflicts: []
first_failure:
  marker: acceptance browser job
  evidence: four acceptance specs expected a created character at the previous head; updated in this commit
rejected_hypotheses: []
changed_paths:
  - app/Characters/Actions/CreateCharacter.php
  - app/Providers/AppServiceProvider.php
  - tests/Feature/Characters/CharacterCreationTest.php
  - scripts/acceptance/tests/full-acceptance.spec.mjs
  - scripts/acceptance/tests/player-journey-acceptance.spec.mjs
  - scripts/acceptance/tests/character-boundaries-acceptance.spec.mjs
  - scripts/acceptance/tests/security-boundary-critical.spec.mjs
  - docs/agents/tasks/active/OTERYN-20261010-create-off.md
validation:
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: counts at or below baseline
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261010-create-off.md --require-checkpoint
    result: PASS
    evidence: task checkpoint validated
  - command: php -l on changed PHP files
    result: PASS
    evidence: no syntax errors
  - command: product runtime E2E
    result: FAIL
    evidence: at 9984a8e8 the acceptance job passes 18 of 19 and fails only the portal visual review spec (character roster row); the browser job runs the real login, create-character submission and result path; four specs now assert the unavailable alert after valid input while validation checks, MFA, sessions and logout stay unchanged; the browser job did not run in the sandbox, exact-head CI is the first run
blockers:
  - scripts/acceptance/tests/portal-visual-review.spec.mjs also creates a character and then expects it in the account roster; it needs an owned-path extension and a seeded character instead of the creation step
next_action: Extend owned paths to the portal visual review spec and seed its character, then re-read exact-head CI.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
