---
task_id: OTERYN-20261006-lcfa-consumer
governing_issue: 1419
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/agents/programs/OTERYN_PLATFORM_COMPLETION_LEAD_PROGRAMME.md
search_first:
  - app/GameAuth/NativeLogin/RegistryNativeAdmissionScopeResolver.php
  - app/GameAuth/OAuth/IssueGameLoginTicketFromOAuth.php
  - app/GameAuth/NativeRuntimeStatus
optional_reads:
  - Oteryn-Game docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md at current accepted producer revision
---

# OTERYN-20261006 LCFA Platform consumer

## Goal

Implement the bounded Platform consumer for Game ListCharactersForAccount v1 under Issue #1419 so Platform can replace the temporary D171 unverified-character route in testing/preproduction without becoming Character authority.

This slice consumes the accepted Game-authored projection contract. It does not mutate Game/Character state, does not activate native production login, and does not resolve the separate P1/U12 production freshness decision.

## Acceptance criteria

- [ ] Dedicated mTLS ingestion routes accept only the Character Authority projection identity and reject identity reuse across all internal purposes.
- [ ] Snapshot and watermark bodies are bounded and strictly decoded per the v1 contract.
- [ ] Additive persistence enforces monotonic epoch/revision, idempotent equal snapshots, equal-key conflict invalidation and global highest-epoch invalidation.
- [ ] Feed freshness is derived from the accepted testing/preproduction bound and fails closed when stale/unavailable.
- [ ] Native admission §5.4 validates account row, highest epoch, conflict state, AVAILABLE character and authoritative world before route selection.
- [ ] Public owner endpoint GET /api/v1/game-auth/native-characters uses the native OAuth client/scope/generation policy without revoking the bearer token.
- [ ] Sensitive responses are no-cache and all failure surfaces use the contract's empty/typed failure vocabulary.
- [ ] Focused endpoint, monotonicity, auth, freshness and admission tests pass.
- [ ] Exact-head CI/Phase 7/Game Auth concurrency and security gates pass.

## Ownership

```yaml
owned_paths:
  - app/GameAuth/NativeAccountCharacters/**
  - app/Http/Controllers/GameAuth/NativeAccountCharacters*.php
  - app/Http/Middleware/GameAuth/GuardNativeAccountCharactersPeer.php
  - app/GameAuth/OAuth/VerifyNativeOAuthAccess.php
  - app/GameAuth/OAuth/IssueGameLoginTicketFromOAuth.php
  - app/GameAuth/NativeLogin/RegistryNativeAdmissionScopeResolver.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - config/game-auth.php
  - routes/internal.php
  - routes/api.php
  - database/migrations/2026_10_06_090000_add_native_account_character_projection.php
  - tests/Feature/GameAuth/NativeAccountCharacters/**
  - tests/Feature/GameAuth/NativeAdmission/**
  - docs/agents/tasks/active/OTERYN-20261006-lcfa-consumer.md
modules:
  - GameAuth
dependencies:
  - Issue #1419
  - Game LCFA producer contract v1
  - merged native runtime/topology projection foundation
blockers:
  - production activation remains separately blocked by Decision P1/U12 measured freshness acceptance
cross_repository_tasks:
  - Game LCFA producer is read-only evidence; no Game write is authorized
```

## Context checkpoint

```yaml
checkpoint_version: 1
status: implementing
phase: lcfa_consumer
branch: feat/1419-lcfa-consumer
head: 3896bcdf75a511f1e386ac645303eaf8f234ffcf
pr: none
context_routes:
  - game-auth
  - testing
  - security
owned_paths:
  - app/GameAuth/NativeAccountCharacters/**
  - app/Http/Controllers/GameAuth/NativeAccountCharacters*.php
  - app/Http/Middleware/GameAuth/GuardNativeAccountCharactersPeer.php
  - app/GameAuth/OAuth/VerifyNativeOAuthAccess.php
  - app/GameAuth/OAuth/IssueGameLoginTicketFromOAuth.php
  - app/GameAuth/NativeLogin/RegistryNativeAdmissionScopeResolver.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - config/game-auth.php
  - routes/internal.php
  - routes/api.php
  - database/migrations/2026_10_06_090000_add_native_account_character_projection.php
  - tests/Feature/GameAuth/NativeAccountCharacters/**
  - tests/Feature/GameAuth/NativeAdmission/**
  - docs/agents/tasks/active/OTERYN-20261006-lcfa-consumer.md
proven:
  - Issue #1419 has no open PR or competing 1419 branch at claim time.
  - Platform #1459 merged and released config/game-auth.php ownership.
  - Game LCFA v1 defines dedicated projection mTLS, monotonic epoch/revision, watermark freshness, owner read and §5.4 admission semantics.
  - Current RegistryNativeAdmissionScopeResolver still uses D171 testing/preproduction unverified_character_world_id and explicitly names LCFA as the release replacement.
derived:
  - The implementation can be additive and Platform-owned as a private read model without direct Game persistence access.
unknown:
  - Production freshness bound acceptance P1/U12; this slice must not promote testing/preproduction evidence to production.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Reuse GuardNativeRuntimeStatusPeer as-is for LCFA purpose.
  - Call IssueGameLoginTicketFromOAuth::execute() from the native-characters read and revoke the bearer token.
  - Store or mutate authoritative Character rows in Platform.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20261006-lcfa-consumer.md
validation:
  - command: live ownership reconstruction
    result: PASS
    evidence: no open #1419 PR/branch and #1459 is terminal
blockers:
  - production release only: Decision P1/U12
next_action: Implement the additive LCFA read model and strict ingestion decoder first, then wire admission and owner OAuth read.
```
