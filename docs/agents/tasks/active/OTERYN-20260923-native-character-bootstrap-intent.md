---
task_id: OTERYN-20260923-native-character-bootstrap-intent
governing_issue: 1406
required_reads:
  - docs/architecture/adr/0030-native-character-portfolio-account-center-v2.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/contracts/OTERYN_V2_CHARACTER_AUTHORITY_COMMAND_CONTRACT.md
  - docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - Character bootstrap intent producer
  - canonical AccountId issuance
  - private game-auth mTLS boundary
optional_reads: []
---

# OTERYN-20260923 native Character bootstrap intent

## Goal

Governing GitHub Issue: #1406 — implement the bounded Platform-owned native Character bootstrap-intent issuer and private reconciliation endpoint without Game or Canary mutation.

## Acceptance criteria

- [x] The operator command resolves canonical AccountId from persisted Platform Identity state and never accepts AccountId as issuance authority.
- [x] Issuance is immutable and idempotent by operation identity, with changed bindings rejected and concurrent reuse serialized.
- [x] One additive migration preserves exact reread, positive source ordering and a dedicated Character-bootstrap-intent namespace.
- [x] A purpose-separated TLS 1.3 mTLS endpoint strictly decodes bounded requests and serves only current immutable intents.
- [x] Focused security, retry, conflict, restart, expiry and NativeEvidence-regression tests pass; real MariaDB concurrency tests are present and routed to the required dedicated environment.
- [x] Exact-candidate formatting, static analysis, repository contracts and acceptance coverage checks pass; the local aggregate PHPUnit route is environment-blocked as recorded below.

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_V2_CHARACTER_BOOTSTRAP_INTENT_PRODUCER_CONTRACT.md
  - app/GameAuth/CharacterBootstrapIntent/**
  - app/Console/Commands/IssueCharacterBootstrapIntent.php
  - app/Http/Controllers/GameAuth/CharacterBootstrapIntentController.php
  - app/Http/Middleware/GameAuth/RequireCharacterBootstrapIntentMtlsPeer.php
  - routes/internal.php
  - config/game-auth.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - database/migrations/*character_bootstrap_intent*
  - tests/Feature/GameAuth/**CharacterBootstrapIntent*
  - docs/agents/tasks/active/OTERYN-20260923-native-character-bootstrap-intent.md
modules:
  - GameAuth/CharacterBootstrapIntent
dependencies:
  - Platform protected main@623435ec1b907d6d9770b767806c90300252a71c
blockers:
  - none
cross_repository_tasks:
  - none; Game access and mutation are excluded
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-23T18:41:00Z
head: cc128f6bab458762ccf442ebeac6a87297ee06a8
branch: coord/native-character-bootstrap-intent-1406
pr: 1407
status: validating
context_routes:
  - architecture
  - accounts-characters
  - database
  - api
  - security
  - testing
owned_paths:
  - docs/contracts/OTERYN_V2_CHARACTER_BOOTSTRAP_INTENT_PRODUCER_CONTRACT.md
  - app/GameAuth/CharacterBootstrapIntent/**
  - app/Console/Commands/IssueCharacterBootstrapIntent.php
  - app/Http/Controllers/GameAuth/CharacterBootstrapIntentController.php
  - app/Http/Middleware/GameAuth/RequireCharacterBootstrapIntentMtlsPeer.php
  - routes/internal.php
  - config/game-auth.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - database/migrations/*character_bootstrap_intent*
  - tests/Feature/GameAuth/**CharacterBootstrapIntent*
  - docs/agents/tasks/active/OTERYN-20260923-native-character-bootstrap-intent.md
proven:
  - protected Platform main and local allocation head are 623435ec1b907d6d9770b767806c90300252a71c
  - live Issue 1406 is open and Draft PR #1407 owns branch coord/native-character-bootstrap-intent-1406
  - canonical AccountId is persisted on identities and generated immutably by Platform
  - the existing private NativeEvidence endpoint proves a TLS 1.3 client-certificate provenance interpretation pattern
  - focused Character bootstrap-intent tests and existing NativeEvidence producer regression pass 214 assertions in total
  - Pint and PHPStan pass and repository-contract plus strict acceptance-coverage validation pass
derived:
  - a separate mTLS identity configuration key preserves purpose separation while reusing the proven interpretation
  - the current merged NativeEvidence task record overlaps config and tests historically but has no active source branch writer
unknown:
  - dedicated MariaDB execution result for the two new independent-process concurrency tests
conflicts: []
first_failure:
  marker: local_php_runtime_mismatch
  evidence: local PHP 8.4.22-dev lacks the repository-required PHP 8.5/Argon2 environment; aggregate PHPUnit cannot truthfully validate both Argon2 fixtures and this local runtime, and its exception renderer exhausts the fixed 128 MiB child-process limit
rejected_hypotheses:
  - accepting caller-supplied AccountId as command authority
  - extending NativeEvidence with a fifth operation
  - using numeric Canary world or account identifiers
changed_paths:
  - app/Console/Commands/IssueCharacterBootstrapIntent.php
  - app/GameAuth/CharacterBootstrapIntent/CharacterBootstrapIntentConflict.php
  - app/GameAuth/CharacterBootstrapIntent/CharacterBootstrapIntentContract.php
  - app/GameAuth/CharacterBootstrapIntent/CharacterBootstrapIntentIssuer.php
  - app/GameAuth/CharacterBootstrapIntent/CharacterBootstrapIntentReader.php
  - app/GameAuth/CharacterBootstrapIntent/CharacterBootstrapIntentRequestDecoder.php
  - app/GameAuth/CharacterBootstrapIntent/CharacterBootstrapIntentUnavailable.php
  - app/Http/Controllers/GameAuth/CharacterBootstrapIntentController.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - app/Http/Middleware/GameAuth/RequireCharacterBootstrapIntentMtlsPeer.php
  - config/game-auth.php
  - database/migrations/2026_09_23_000100_create_character_bootstrap_intent_state.php
  - docs/agents/tasks/active/OTERYN-20260923-native-character-bootstrap-intent.md
  - docs/contracts/OTERYN_V2_CHARACTER_BOOTSTRAP_INTENT_PRODUCER_CONTRACT.md
  - routes/internal.php
  - tests/Feature/GameAuth/CharacterBootstrapIntentConcurrencyTest.php
  - tests/Feature/GameAuth/CharacterBootstrapIntentProducerTest.php
validation:
  - command: vendor/bin/pint --test
    result: PASS
    evidence: all repository PHP files pass formatting
  - command: COMPOSER_ALLOW_SUPERUSER=1 composer analyse -- --error-format=table
    result: PASS
    evidence: PHPStan analysed 748 files with no errors
  - command: HASH_DRIVER=bcrypt php artisan test tests/Feature/GameAuth/CharacterBootstrapIntentProducerTest.php tests/Feature/GameAuth/CharacterBootstrapIntentConcurrencyTest.php tests/Feature/GameAuth/NativeEvidenceProducerTest.php
    result: PASS
    evidence: 20 tests and 214 assertions pass; only the two dedicated-MariaDB concurrency cases skip in local SQLite
  - command: GAME_AUTH_CONCURRENCY_TEST=1 php artisan test tests/Feature/GameAuth/CharacterBootstrapIntentConcurrencyTest.php
    result: NOT_RUN
    evidence: dedicated MariaDB service credentials are unavailable in this local workspace; the tests fail closed unless both MariaDB and pcntl are present
  - command: COMPOSER_ALLOW_SUPERUSER=1 composer test:repository-contracts
    result: PASS
    evidence: repository, integration-registration, prompt and workflow inventory contracts pass
  - command: npm --prefix scripts/acceptance run test:coverage-contract:strict
    result: PASS
    evidence: strict portal coverage contract and negative fixtures pass
  - command: COMPOSER_ALLOW_SUPERUSER=1 HASH_DRIVER=bcrypt composer verify
    result: BLOCKED
    evidence: local PHP 8.4.22-dev lacks the required Argon2 runtime and composer-installed dependencies required --ignore-platform-req=php/ext-sodium; forcing bcrypt invalidates existing Argon2 fixture expectations and the fixed 128 MiB test child exhausts memory while rendering those unrelated failures
  - command: python3 tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20260923-native-character-bootstrap-intent.md --require-checkpoint
    result: PASS
    evidence: checkpoint validates against contract version 1
blockers:
  - none for Draft PR publication; exact-head CI must supply the required PHP 8.5, Argon2 and MariaDB concurrency environments before readiness
next_action: qualify the current PR #1407 exact head; repair only exact-head findings, then mark the unchanged qualified candidate ready for governed integration
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

Product runtime E2E against Game is not authorized; the Platform actor-to-result path is the operator command followed by the private exact-reread endpoint.
