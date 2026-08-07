---
task_id: OTERYN-20260807-native-oauth-revocation-generation
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
  - app/AGENTS.md
  - app/GameAuth/AGENTS.md
  - app/Identity/AGENTS.md
  - tests/AGENTS.md
  - docs/contracts/AGENTS.md
search_first:
  - game_auth_generation
  - native OAuth
optional_reads: []
---

# OTERYN-20260807-native-oauth-revocation-generation

## Goal

Remediate Issue #801 so native OAuth authorization established before an identity game-authorization revocation cannot mint a new game login ticket after the identity generation changes.

## Acceptance criteria

- [x] Native OAuth authorization/token security context is bound to the identity game authorization generation active when that authorization is established.
- [x] Native game-login bootstrap fails closed when the token-bound generation differs from the current Identity generation.
- [x] Refresh-token descendants of pre-revocation authorization remain unable to bypass the generation boundary.
- [x] Post-revocation authorization can issue a game-login ticket when all other security gates pass.
- [x] Existing single-use, disabled/terminated identity, scope and native-client gates remain enforced in code and regression coverage.
- [x] Security regression coverage and the authoritative auth/game-login contract are updated and validated.
- [ ] Heightened exact-head validation passes before merge; no production activation or external-repository mutation occurs.

## Ownership

```yaml
owned_paths:
  - app/Identity/Actions/RevokeIdentityGameAuthorizations.php
  - app/Identity/Credentials/IdentityCredentialUpdater.php
  - app/GameAuth/OAuth/IssueGameLoginTicketFromOAuth.php
  - app/GameAuth/OAuth/NativeOAuthGenerationBinding.php
  - app/Http/Controllers/GameAuth/GameLoginTicketIssueController.php
  - app/Providers/AppServiceProvider.php
  - app/Providers/GameAuthOAuthServiceProvider.php
  - bootstrap/providers.php
  - database/migrations/2026_08_07_142500_add_game_auth_generation_to_passport_authorizations.php
  - tests/Feature/GameAuth/OAuth/**
  - tests/Feature/GameAuth/GameAuthRevocationTest.php
  - tests/Feature/GameAuth/Concurrency/GameTicketConcurrencyTest.php
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260807-native-oauth-revocation-generation.md
modules:
  - native-oauth-game-auth
  - identity-game-auth-revocation
dependencies:
  - issue:#801
blockers:
  - none
cross_repository_tasks:
  - none
coordination_key: module:native-oauth-revocation-generation
validation_intensity: HEIGHTENED
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-07T15:47:00Z
head: caf1011ae4d0de362307c222c4258c9be2b6f92d
branch: repair/issue-801
pr: 825
status: validating
context_routes:
  - security
  - identity
  - game-auth
owned_paths:
  - app/Identity/Actions/RevokeIdentityGameAuthorizations.php
  - app/Identity/Credentials/IdentityCredentialUpdater.php
  - app/GameAuth/OAuth/IssueGameLoginTicketFromOAuth.php
  - app/GameAuth/OAuth/NativeOAuthGenerationBinding.php
  - app/Http/Controllers/GameAuth/GameLoginTicketIssueController.php
  - app/Providers/AppServiceProvider.php
  - app/Providers/GameAuthOAuthServiceProvider.php
  - bootstrap/providers.php
  - database/migrations/2026_08_07_142500_add_game_auth_generation_to_passport_authorizations.php
  - tests/Feature/GameAuth/OAuth/**
  - tests/Feature/GameAuth/GameAuthRevocationTest.php
  - tests/Feature/GameAuth/Concurrency/GameTicketConcurrencyTest.php
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260807-native-oauth-revocation-generation.md
proven:
  - Identity game authorization revocation increments game_auth_generation transactionally.
  - Native OAuth authorization codes carrying game:ticket persist the current Identity game_auth_generation.
  - Access tokens carrying game:ticket inherit generation from authorization-code or refresh-token security context and fail closed if it differs from current Identity state.
  - Native OAuth bootstrap compares token generation with a locked current Identity generation before issuing a Game Login Ticket.
  - Identity revocation proactively revokes native game:ticket authorization codes, access tokens and refresh descendants in the same transaction as the generation increment.
  - Passport 13.7.5 and league/oauth2-server 9.4.1 payload shapes used by the binding were verified against pinned dependency source.
  - The MariaDB concurrency fixture now reloads the persisted Identity database defaults before constructing synthetic Passport token state; Game Auth Ticket Concurrency run 31195150972 passes on commit 1d282e7df51e1aa41eb65b97d662f99aa8b36abe.
  - The authoritative auth/game-login contract records the Platform-native OAuth generation boundary and explicitly preserves UNKNOWN status for retained Canary, account_sessions, legacy, active-session and production-cutover revocation.
  - Current main f523977f852def1f5f1b722a11fbb98196370f5d was synchronized into the repair branch without overlapping task-owned paths.
derived:
  - Pre-revocation OAuth material cannot cross the generation boundary even if an access-token revoked flag is independently reopened, because bootstrap also enforces generation equality.
unknown: []
conflicts: []
first_failure:
  marker: game-auth-concurrency-31190093475
  evidence: the synthetic Passport token fixture wrote null game_auth_generation because the freshly created Eloquent Identity had not reloaded the database default; both OAuth racers therefore failed closed. Reloading the persisted Identity before token construction repaired the fixture without weakening runtime enforcement.
rejected_hypotheses:
  - Revoking only current access-token rows is sufficient; rejected because authorization codes and refresh descendants also require a deterministic generation boundary.
  - Runtime OAuth bootstrap serialization caused the two-denial concurrency failure; rejected because the non-concurrency revocation suite passed and the fixture token generation was null before either racer executed.
changed_paths:
  - app/Identity/Actions/RevokeIdentityGameAuthorizations.php
  - app/GameAuth/OAuth/IssueGameLoginTicketFromOAuth.php
  - app/GameAuth/OAuth/NativeOAuthGenerationBinding.php
  - app/Providers/GameAuthOAuthServiceProvider.php
  - bootstrap/providers.php
  - database/migrations/2026_08_07_142500_add_game_auth_generation_to_passport_authorizations.php
  - tests/Feature/GameAuth/OAuth/NativeOAuthRevocationGenerationTest.php
  - tests/Feature/GameAuth/Concurrency/GameTicketConcurrencyTest.php
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260807-native-oauth-revocation-generation.md
validation:
  - command: Game Auth Ticket Concurrency / run 31190093475
    result: FAIL
    evidence: identified stale in-memory Identity fixture state as the first actionable failure.
  - command: Game Auth Ticket Concurrency / run 31195150972
    result: PASS
    evidence: exact commit 1d282e7df51e1aa41eb65b97d662f99aa8b36abe passes after the targeted fixture repair.
  - command: CI / run 31195151356
    result: PASS
    evidence: exact commit 1d282e7df51e1aa41eb65b97d662f99aa8b36abe passes repository CI.
  - command: Agent Governance / run 31195152618
    result: PASS
    evidence: exact commit 1d282e7df51e1aa41eb65b97d662f99aa8b36abe passes governance.
  - command: Phase 7 Production-Like Validation / run 31195150409
    result: PASS
    evidence: exact commit 1d282e7df51e1aa41eb65b97d662f99aa8b36abe passes production-like validation without production activation.
blockers:
  - none
next_action: execute HEIGHTENED validation on the final synchronized head, self-review the exact final diff, verify PR review-thread hygiene and merge only if every required gate passes
```

## Notes

Security repair is restricted to this repository. Production activation and external-repository mutation remain out of scope.