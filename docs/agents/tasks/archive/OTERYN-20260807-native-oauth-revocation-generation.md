---
task_id: OTERYN-20260807-native-oauth-revocation-generation
issue: 801
status: completed
completed_at: 2026-08-07T16:28:18Z
implementation_pull_request: 825
implementation_head: 9183a55c04427ef7a56fa82d097173ef058d8d94
implementation_merge: f6a2b6cefe8ad5993436ac18be8ca4d08919d69b
risk: high
validation_intensity: HEIGHTENED
self_review: PASS
material_findings: 0
production_activation_authorized: false
external_repository_mutation: false
ownership: RELEASED_ON_ARCHIVE_MERGE
---

# OTERYN-20260807 native OAuth revocation generation — Completed

## Result

Issue #801 is repaired. Platform-native OAuth game-login authorization can no longer cross an Identity `game_auth_generation` revocation boundary.

The delivered security boundary:

- binds native OAuth authorization codes carrying `game:ticket` to the Identity generation active when authorization is established;
- propagates that generation to authorization-code and refresh-token access-token descendants;
- fails closed when a `game:ticket` access token has missing or stale generation state;
- locks the current Identity and access token before Game Login Ticket bootstrap and rejects disabled or terminated identities;
- transactionally increments `game_auth_generation` and revokes outstanding native `game:ticket` authorization codes, access tokens and refresh descendants;
- preserves the existing short-lived, PKCE/S256, native-client, scope, expiry, revoked-token and one-bootstrap-use controls;
- adds deterministic regression coverage for stale access tokens, stale refresh descendants, post-revocation authorization, disabled/terminated identities and revocation-versus-bootstrap concurrency;
- updates the authoritative auth/game-login contract without claiming revocation for retained Canary, `account_sessions`, legacy direct-password paths, active game sessions or production cutover.

## Delivery

- Implementation PR: #825.
- Final exact implementation head: `9183a55c04427ef7a56fa82d097173ef058d8d94`.
- Synchronized implementation base/main: `f523977f852def1f5f1b722a11fbb98196370f5d`.
- Protected squash merge: `f6a2b6cefe8ad5993436ac18be8ca4d08919d69b`.
- Issue #801 closed automatically as completed by the merge.

## Exact-head validation

Applicable exact-head evidence on `9183a55c04427ef7a56fa82d097173ef058d8d94`:

- CI `31195817494`: PASS. Required `classify-changes` and `test` branch-protection contexts passed.
- Game Auth Ticket Concurrency `31195817269`: PASS, including revocation-versus-bootstrap serialization.
- Agent Governance `31195817204`: PASS.
- Platform DB Outage Validation `31195817308`: PASS.
- Edge Security Emulation `31195817964`: PASS.
- Phase 7 Production-Like Validation `31195817466`: PASS without production activation.
- Acceptance E2E and Visual UX `31195817350`: PASS.
- Portal Acceptance Contract `31195817730`: PASS.
- Deep System Validation `31195817276`: PASS, including Composer/advisory checks, PHP formatting/static analysis, complete PHP regression/concurrency suites, strict portal evidence contracts and complete zero-retry browser matrix.
- PR review threads: zero.
- Exact-head self-review: PASS with zero material findings.

`Native protocol contract audits` run `31195817484` emitted one unrelated Audit 1 false-positive because a generic `docs/contracts/**` trigger caused unrelated authentication runtime files to be evaluated against the native-protocol producer allowlist. Audit 2–5 passed. The failing check is not a protected `main` required context; the CI defect is tracked separately as Issue #829 and was not used to weaken or bypass any required merge gate.

## Regression evidence

Deterministic coverage proves:

- an access token authorized before generation revocation cannot bootstrap a fresh Game Login Ticket afterwards;
- reopening only the access-token `revoked` flag still cannot cross the generation boundary;
- a pre-revocation refresh token cannot mint a fresh descendant after revocation;
- refresh descendants inherit the original authorization generation;
- a newly authorized token family after revocation receives the current generation and can bootstrap when all other gates pass;
- disabled and terminated identities fail closed before bootstrap;
- concurrent OAuth bootstrap and game-authorization revocation serialize under the Identity-first locking order;
- existing ticket single-use and security-generation behavior remains intact.

## Migration, rollback and compatibility

The Passport schema change is additive. `oauth_auth_codes.game_auth_generation` and `oauth_access_tokens.game_auth_generation` are nullable for rollout compatibility, while `game:ticket` bootstrap intentionally treats missing generation as invalid so pre-binding or stale authorization material is not grandfathered.

The change does not alter retained Canary/login-server credential formats or claim Laravel/Canary password-hash compatibility. Production activation, legacy-path retirement, external-repository mutation and live-session revocation remain outside this repair.

## E2E and safety

Applicable repository E2E passed on the exact implementation head. No production deployment, protected-environment approval, production secret change, live account/session mutation, Canary mutation or external-repository mutation was performed.

## Ownership release

This archival closeout removes the durable active-task lease for Issue #801. Once this archive change is merged, all Issue #801 implementation paths and `module:native-oauth-revocation-generation` coordination ownership are released.
