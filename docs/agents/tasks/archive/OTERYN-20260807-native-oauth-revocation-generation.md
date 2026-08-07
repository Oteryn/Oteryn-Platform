---
task_id: OTERYN-20260807-native-oauth-revocation-generation
issue: 801
status: completed
completed_at: 2026-08-07T16:29:14Z
implementation_pull_request: 825
implementation_head: 9183a55c04427ef7a56fa82d097173ef058d8d94
implementation_merge: f6a2b6cefe8ad5993436ac18be8ca4d08919d69b
risk: high
validation_intensity: HEIGHTENED
self_review: PASS
material_findings: 0
production_activation_authorized: false
ownership: RELEASED_ON_ARCHIVE_MERGE
---

# OTERYN-20260807 native OAuth revocation generation — Completed

## Result

Issue #801 is repaired. Native OAuth game-login authorization material created before an Identity game-authorization generation change can no longer mint a fresh current-generation Game Login Ticket after revocation.

The delivered boundary:

- persists `game_auth_generation` on `game:ticket` authorization codes and access tokens;
- propagates the authorization generation through access-token and refresh-token descendants;
- compares the token-bound generation with a locked current Identity generation before Game Login Ticket bootstrap;
- proactively revokes outstanding native `game:ticket` authorization codes, access tokens and refresh descendants transactionally with the Identity generation increment;
- preserves existing native-client, scope, expiry, single-bootstrap, disabled and terminated Identity gates;
- leaves production activation, legacy Canary/password paths, `account_sessions`, active Game Sessions and external repositories outside this repair.

## Delivery

- Implementation PR: #825.
- Final exact implementation head: `9183a55c04427ef7a56fa82d097173ef058d8d94`.
- Synchronized base/main before merge: `f523977f852def1f5f1b722a11fbb98196370f5d`.
- Protected merge commit: `f6a2b6cefe8ad5993436ac18be8ca4d08919d69b`.
- Issue #801 closed as completed by the merge.
- Related audit PR #802 is merged and terminal.

## Exact-head validation

Applicable evidence on `9183a55c04427ef7a56fa82d097173ef058d8d94`:

- CI `31195817494`: PASS.
- Game Auth Ticket Concurrency `31195817269`: PASS.
- Agent Governance `31195817204`: PASS.
- Acceptance E2E and Visual UX `31195817350`: PASS.
- Deep System Validation `31195817276`: PASS, including complete PHP regression/concurrency lanes and the zero-retry browser matrix.
- Phase 7 Production-Like Validation `31195817466`: PASS.
- Platform DB Outage Validation `31195817308`: PASS.
- Edge Security Emulation `31195817964`: PASS.
- PR review threads: zero.
- Requested changes/submitted reviews: zero.

The HEIGHTENED exact-head self-review recorded on PR #825 returned PASS with zero material findings.

`Native protocol contract audits` run `31195817484` emitted one unrelated Audit 1 false positive because a generic `docs/contracts/**` trigger incorrectly applies native-protocol producer ownership rules to unrelated runtime changes. The other four native-protocol audit jobs passed. This CI routing defect is independently tracked as Issue #829 and does not weaken the Issue #801 validation evidence.

## Regression evidence

Deterministic coverage proves:

- pre-revocation access tokens cannot bootstrap after generation change;
- pre-revocation refresh material cannot mint a descendant that crosses the generation boundary;
- refreshed descendants preserve the authorization generation;
- post-revocation newly authorized tokens use the current generation and can bootstrap;
- disabled and terminated identities fail closed;
- concurrent OAuth bootstrap remains single-use;
- game-authorization revocation serializes safely against OAuth bootstrap.

The initial concurrency failure was isolated to stale in-memory test fixture state: the synthetic Passport token received a null generation because the Identity model had not reloaded its database default. Reloading persisted Identity state repaired the fixture without weakening runtime enforcement.

## Rollback and compatibility

The Passport schema change is additive and nullable so existing non-game Passport authorization remains compatible. Missing generation on `game:ticket` material fails closed. Application rollback must preserve the migration/data ordering required by the repository and must not be represented as production activation or legacy-path retirement.

## Safety and scope

No production activation, protected-environment mutation, credential migration, Canary/login-server mutation or external-repository change was performed by this repair.

## Ownership release

This archival closeout removes the durable active-task lease for Issue #801. Once this archive change is merged, the `module:native-oauth-revocation-generation` coordination key and all Issue #801 task-owned paths are released.