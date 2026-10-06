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

- [x] Dedicated mTLS ingestion routes accept only the Character Authority projection identity and reject identity reuse across all internal purposes.
- [x] Snapshot and watermark bodies are bounded and strictly decoded per the v1 contract.
- [x] Additive persistence enforces monotonic epoch/revision, idempotent equal snapshots, equal-key conflict invalidation and global highest-epoch invalidation.
- [x] Feed freshness is derived from the accepted testing/preproduction bound and fails closed when stale/unavailable.
- [x] Native admission §5.4 validates account row, highest epoch, conflict state, AVAILABLE character and authoritative world before route selection.
- [x] Public owner endpoint GET /api/v1/game-auth/native-characters uses the native OAuth client/scope/generation policy without revoking the bearer token.
- [x] Sensitive responses are no-cache and all failure surfaces use the contract's empty/typed failure vocabulary.
- [ ] Focused endpoint, monotonicity, auth, freshness and admission tests pass.
- [ ] Exact-head CI/Phase 7/Game Auth concurrency and security gates pass.

## Ownership

```yaml
owned_paths:
  - app/GameAuth/NativeAccountCharacters/**
  - app/Http/Controllers/GameAuth/NativeAccountCharacters*.php
  - app/Http/Middleware/GameAuth/GuardNativeAccountCharactersPeer.php
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimeStatusSettings.php
  - app/Http/Middleware/GameAuth/RequireNativeEvidenceMtlsPeer.php
  - app/Http/Middleware/GameAuth/RequireCharacterBootstrapIntentMtlsPeer.php
  - app/ProductsEntitlements/Premium/PremiumSnapshotSettings.php
  - app/GameAuth/OAuth/VerifyNativeOAuthAccess.php
  - app/GameAuth/OAuth/VerifiedNativeOAuthAccess.php
  - app/GameAuth/OAuth/IssueGameLoginTicketFromOAuth.php
  - app/GameAuth/NativeLogin/RegistryNativeAdmissionScopeResolver.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - app/Providers/AppServiceProvider.php
  - config/game-auth.php
  - routes/internal.php
  - routes/api.php
  - database/migrations/2026_10_06_090000_add_native_account_character_projection.php
  - tests/Feature/GameAuth/NativeAccountCharacters/**
  - tests/Feature/GameAuth/NativeLogin/NativeAdmissionIssuerTest.php
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
status: validating
phase: pre_pr_validation
branch: feat/1419-lcfa-consumer
head: 5ef182be2c7e7546903d634a119a2655d2081f28
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
  - RegistryNativeAdmissionScopeResolver now requires a live LCFA highest-epoch account view whenever LCFA is enabled; D171 remains available only while LCFA is explicitly disabled in testing/preproduction.
  - Owner read reuses the exact native OAuth client/scope/generation verifier but does not revoke the bearer or refresh token.
  - Projection certificate subjects are rejected symmetrically by runtime-status, scope-assignment, native-evidence, character-bootstrap and Premium purposes.
derived:
  - The implementation can be additive and Platform-owned as a private read model without direct Game persistence access.
unknown:
  - Production freshness bound acceptance P1/U12; this slice must not promote testing/preproduction evidence to production.
  - U1 native OAuth selection/issuance of an oteryn-native-game-gateway ticket remains a separate #1419 successor; this LCFA slice does not alter public ticket kind semantics.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Reuse GuardNativeRuntimeStatusPeer as-is for LCFA purpose.
  - Call IssueGameLoginTicketFromOAuth::execute() from the native-characters read and revoke the bearer token.
  - Store or mutate authoritative Character rows in Platform.
changed_paths:
  - app/GameAuth/NativeAccountCharacters/**
  - app/GameAuth/NativeLogin/RegistryNativeAdmissionScopeResolver.php
  - app/GameAuth/NativeRuntimeStatus/NativeRuntimeStatusSettings.php
  - app/GameAuth/OAuth/IssueGameLoginTicketFromOAuth.php
  - app/GameAuth/OAuth/VerifiedNativeOAuthAccess.php
  - app/GameAuth/OAuth/VerifyNativeOAuthAccess.php
  - app/Http/Controllers/GameAuth/NativeAccountCharacters*.php
  - app/Http/Middleware/GameAuth/EnforceNativeAccountCharactersHttpBounds.php
  - app/Http/Middleware/GameAuth/GuardNativeAccountCharactersPeer.php
  - app/Http/Middleware/GameAuth/RequireCharacterBootstrapIntentMtlsPeer.php
  - app/Http/Middleware/GameAuth/RequireNativeEvidenceMtlsPeer.php
  - app/ProductsEntitlements/Premium/PremiumSnapshotSettings.php
  - app/Providers/AppServiceProvider.php
  - config/game-auth.php
  - routes/api.php
  - routes/internal.php
  - database/migrations/2026_10_06_090000_add_native_account_character_projection.php
  - tests/Feature/GameAuth/NativeAccountCharacters/**
  - tests/Feature/GameAuth/NativeLogin/NativeAdmissionIssuerTest.php
  - docs/agents/tasks/active/OTERYN-20261006-lcfa-consumer.md
validation:
  - command: live ownership reconstruction
    result: PASS
    evidence: no competing #1419 PR exists, this branch is the sole claimed LCFA lane, and #1459 is merged
  - command: contract-to-diff whole-slice review
    result: PASS
    evidence: reviewed against Platform §5 and current Game LCFA v1 for strict wire, characters-only equal-key digest, epoch/watermark rules, symmetric mTLS purpose isolation, bearer-only owner rate key, no-revocation OAuth read and §5.4 failure mapping
  - command: focused PHP/feature validation
    result: NOT_RUN
    evidence: repository checkout is unavailable in this chat runtime; exact candidate validation will run through normal PR CI before integration
blockers:
  - production release only: Decision P1/U12
next_action: Open the bounded LCFA consumer PR, hold the exact head, and repair only evidence-backed CI/review failures.
```
