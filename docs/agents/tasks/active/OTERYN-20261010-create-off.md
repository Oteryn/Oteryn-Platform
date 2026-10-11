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
- [x] Actor-to-result browser path asserts the unavailable submission result (five acceptance specs updated; the lead approved extending owned paths).
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
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - tools/validation/legacy_token_ratchet.py
  - tools/validation/test_legacy_token_ratchet.py
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
branch: claude/decanary-create-off-1-20261010
pr: 1492
status: validating
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
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - tools/validation/legacy_token_ratchet.py
  - tools/validation/test_legacy_token_ratchet.py
proven:
  - Legacy token ratchet passes locally; runtime 497 to 488, tests 712 to 701.
derived:
  - The existing controller already maps the unavailable exception to a bounded form error.
unknown:
  - PHP suite result; composer dependencies could not be installed in the authoring sandbox, so CI is the first run.
  - Follow-up CREATE-OFF-2: success, duplicate, foreign-id, quota and public-visibility creation states are not exercised while creation is off; the coverage manifest records the unavailable contract as covered, and these states return when creation is re-enabled (decision D993).
  - The account overview still reports character creation as allowed and shows a create button; that projection and view are outside this packet and need a follow-up, routed to the account backlog item B2 (account projection) (decision D988).
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
  - scripts/acceptance/tests/portal-visual-review.spec.mjs
  - scripts/acceptance/coverage/portal-coverage-manifest.json
  - tools/validation/legacy_token_ratchet.py
  - tools/validation/test_legacy_token_ratchet.py
  - docs/agents/tasks/active/OTERYN-20261010-create-off.md
validation:
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: counts at or below baseline; the ratchet ignores the task record branch line (decision D989), docs stays under 1693
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20261010-create-off.md --require-checkpoint
    result: PASS
    evidence: task checkpoint validated
  - command: php -l on changed PHP files
    result: PASS
    evidence: no syntax errors
  - command: product runtime E2E
    result: NOT_RUN
    evidence: at 9984a8e8 the acceptance job passed 18 of 19 and failed only the portal visual review spec; that spec now asserts the unavailable alert and seeds its roster character through the game database, so no test is skipped; five specs now assert the unavailable alert; the browser job did not run in the sandbox, exact-head CI is the first run
blockers:
  - none
next_action: Read exact-head CI for the acceptance job; the account overview creation affordance stays open for backlog item B2 (account projection).
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
