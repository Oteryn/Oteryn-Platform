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
- [ ] Security regression coverage and the authoritative auth/game-login contract are updated and validated.
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
updated_at: 2026-08-07T14:40:40Z
head: 4849d61958f5b1708806bdac8f7f6a5194b2e60c
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
  - Native OAuth authorization codes carrying game:ticket now persist the current Identity game_auth_generation.
  - Access tokens carrying game:ticket inherit generation from authorization-code or refresh-token security context and fail closed if it differs from current Identity state.
  - Native OAuth bootstrap now compares token generation with a locked current Identity generation before issuing a Game Login Ticket.
  - Identity revocation proactively revokes native game:ticket authorization codes, access tokens and refresh descendants in the same transaction as the generation increment.
  - Passport 13.7.5 and league/oauth2-server 9.4.1 payload shapes used by the binding were verified against pinned dependency source.
derived:
  - Pre-revocation OAuth material cannot cross the generation boundary even if an access-token revoked flag is independently reopened, because bootstrap also enforces generation equality.
unknown: []
conflicts: []
first_failure:
  marker: acceptance-e2e-31188457028
  evidence: eager service-provider binding resolution required APP_KEY during composer package discovery; fixed by lazy event-time resolution in 4849d61958f5b1708806bdac8f7f6a5194b2e60c
rejected_hypotheses:
  - Revoking only current access-token rows is sufficient; rejected because authorization codes and refresh descendants also require a deterministic generation boundary.
changed_paths:
  - app/Identity/Actions/RevokeIdentityGameAuthorizations.php
  - app/GameAuth/OAuth/IssueGameLoginTicketFromOAuth.php
  - app/GameAuth/OAuth/NativeOAuthGenerationBinding.php
  - app/Providers/GameAuthOAuthServiceProvider.php
  - bootstrap/providers.php
  - database/migrations/2026_08_07_142500_add_game_auth_generation_to_passport_authorizations.php
  - tests/Feature/GameAuth/OAuth/NativeOAuthRevocationGenerationTest.php
  - tests/Feature/GameAuth/Concurrency/GameTicketConcurrencyTest.php
  - docs/agents/tasks/active/OTERYN-20260807-native-oauth-revocation-generation.md
validation:
  - command: Agent Governance / run 31188458065
    result: FAIL
    evidence: branch PR identity was omitted from the task checkpoint; repaired by recording PR 825, pending exact-head revalidation.
  - command: Acceptance E2E and Visual UX / run 31188457028
    result: FAIL
    evidence: composer package discovery failed because the provider eagerly resolved Encrypter without APP_KEY; repaired by lazy event-time resolution, pending exact-head revalidation.
blockers:
  - none
next_action: update the authoritative auth contract, synchronize with current main, then execute heightened exact-head validation and repair the first attributable failure
```

## Notes

Security repair is restricted to this repository. Production activation and external-repository mutation remain out of scope.