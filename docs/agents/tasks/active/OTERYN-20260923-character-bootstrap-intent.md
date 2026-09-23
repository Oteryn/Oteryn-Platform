---
task_id: OTERYN-20260923-character-bootstrap-intent
governing_issue: 1406
required_reads: []
search_first: []
optional_reads: []
---

# OTERYN-20260923-character-bootstrap-intent

## Goal

Governing GitHub Issue: 1406 — implement the Platform-owned native Character bootstrap-intent producer and private reconciliation endpoint.

## Acceptance criteria

- [x] Immutable, idempotent operator issuance resolves a canonical Platform AccountId.
- [x] A private TLS 1.3 endpoint returns only a current canonical v1 intent.
- [x] Dedicated monotonic source ordering fails closed on conflicts and rollback.
- [x] Focused tests and static candidate validation pass; full runtime validation remains CI-bound.

## Ownership

```yaml
owned_paths:
  - app/GameAuth/CharacterBootstrapIntent/**
  - app/Console/Commands/IssueCharacterBootstrapIntent.php
  - app/Http/Controllers/GameAuth/CharacterBootstrapIntentController.php
  - app/Http/Middleware/GameAuth/RequireCharacterBootstrapIntentMtlsPeer.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - config/game-auth.php
  - database/migrations/*character_bootstrap_intents.php
  - docs/contracts/OTERYN_V2_CHARACTER_BOOTSTRAP_INTENT_PRODUCER_CONTRACT.md
  - routes/internal.php
  - tests/Feature/GameAuth/*CharacterBootstrapIntent*
  - docs/agents/tasks/active/OTERYN-20260923-character-bootstrap-intent.md
modules:
  - Platform GameAuth
dependencies:
  - accepted Issue 1406 consumer semantics
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-23T13:00:00Z
head: 623435ec1b907d6d9770b767806c90300252a71c
branch: coord/native-character-bootstrap-intent-1406
pr: none
status: ready
context_routes: []
owned_paths:
  - Issue 1406 allowlist
proven:
  - live Issue 1406 is open
  - no existing remote task branch or PR was present at admission
derived:
  - issuance can use the canonical identities.account_id as the authority subject
unknown:
  - exact-head CI result
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260923-character-bootstrap-intent.md
validation:
  - command: php artisan test --compact tests/Feature/GameAuth/CharacterBootstrapIntentProducerTest.php
    result: PASS
    evidence: 5 tests and 23 assertions pass; environment emits only missing .env warnings
  - command: vendor/bin/phpstan analyse --memory-limit=1G focused-paths
    result: PASS
    evidence: no errors
  - command: vendor/bin/pint --test
    result: PASS
    evidence: all changed PHP files formatted
blockers:
  - none
next_action: publish the frozen candidate as one draft PR and await exact-head CI
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository task branch
source_branch_evidence: configured disposition for draft PR publication
```

## Notes

Platform-only implementation; no Game repository access or Canary character-create changes.
