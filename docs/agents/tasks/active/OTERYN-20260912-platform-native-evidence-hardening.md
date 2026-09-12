---
task_id: OTERYN-20260912-platform-native-evidence-hardening
governing_issue: 1388
required_reads:
  - docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/contracts/AUTH_GAME_LOGIN_CONTRACT.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/TEST_STRATEGY.md
search_first:
  - native evidence high-water witness durability and provenance
  - RevokeIdentityGameAuthorizations transaction callers
  - native signing trust revocation lifecycle
  - native evidence mTLS middleware and qualification evidence
optional_reads: []
---

# OTERYN-20260912 Platform native evidence hardening

## Goal

Governing GitHub Issue: #1388 — canonical lifecycle authority for this task.

Harden the existing Platform native-evidence producer for truthful non-production real Platform↔Game interoperability qualification while preserving fail-closed anti-rollback, Platform ownership boundaries and separate production/PKI/credential authority.

Terminal task outcome: `PLATFORM_NATIVE_EVIDENCE_READY_FOR_REAL_INTEROP`.

## Acceptance criteria

- [ ] High-water storage requires and proves the accepted file + directory durability profile; missing/unavailable storage fails closed.
- [ ] Witness-store provenance distinguishes first activation from retained-store loss/replacement.
- [ ] Native security generation can reconcile witness-ahead/DB-behind rollback aftermath only in the forward/revoking direction.
- [ ] Password change/reset, recovery, MFA/email and termination callers remain covered by the shared revocation boundary.
- [ ] `GAME_AUTH_NATIVE_EVIDENCE_REQUESTS_PER_MINUTE` semantics are explicitly truthful under concurrency.
- [ ] Key rotation, key revocation and terminal profile revocation/recovery are distinct and documented/tested without private signing material.
- [ ] Non-production real-interoperability qualification is specified for TLS 1.3 mTLS terminator provenance, all four operations and exact Game fixtures/revisions.
- [ ] Focused tests and repository-required exact-candidate validation pass for the frozen head.

## Ownership

```yaml
owned_paths:
  - app/GameAuth/NativeEvidence/**
  - app/Console/Commands/ReconcileNativeEvidence.php
  - app/Http/Middleware/GameAuth/RequireNativeEvidenceMtlsPeer.php
  - app/Http/Middleware/GameAuth/ThrottleNativeEvidencePeer.php
  - app/Identity/Actions/RevokeIdentityGameAuthorizations.php
  - config/game-auth.php
  - database/migrations/*native_game_evidence*
  - tests/Feature/GameAuth/**
  - docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md
  - docs/agents/evidence/OTERYN-20260912-platform-native-evidence-hardening/**
  - docs/agents/tasks/active/OTERYN-20260912-platform-native-evidence-hardening.md
modules:
  - GameAuth/NativeEvidence
  - Identity security lifecycle
dependencies:
  - Platform main@2271fea9db1e202cacb206dab289efa4cefcec76
  - Game protected main@489e3e390a1bce1ce3439c66521ab75f8a826cd8 read-only consumer evidence
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Game read-only exact consumer compatibility inspection; no Game mutation
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-12T19:12:00Z
head: b4791fbaf742863003e4afae0217c34cc6d66555
branch: agent/platform-native-evidence-hardening-1388
pr: 1389
status: implementing
context_routes:
  - architecture
  - auth-identity
  - database
  - api
  - security
  - testing
owned_paths:
  - app/GameAuth/NativeEvidence/**
  - app/Console/Commands/ReconcileNativeEvidence.php
  - app/Http/Middleware/GameAuth/RequireNativeEvidenceMtlsPeer.php
  - app/Http/Middleware/GameAuth/ThrottleNativeEvidencePeer.php
  - app/Identity/Actions/RevokeIdentityGameAuthorizations.php
  - config/game-auth.php
  - database/migrations/*native_game_evidence*
  - tests/Feature/GameAuth/**
  - docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md
  - docs/agents/evidence/OTERYN-20260912-platform-native-evidence-hardening/**
  - docs/agents/tasks/active/OTERYN-20260912-platform-native-evidence-hardening.md
proven:
  - protected main admission baseline is 2271fea9db1e202cacb206dab289efa4cefcec76
  - Issue #1388 is open and governs this Platform task
  - PR #1389 owns branch agent/platform-native-evidence-hardening-1388
  - pre-hardening witness persistence conditionally skipped fsync when PHP did not expose it
  - current shared revocation action can advance the retained native-generation witness before an enclosing security transaction commits
  - password change/reset, MFA, email, recovery and termination families all route game-authorization revocation through the shared action
  - Game protected main 489e3e390a1bce1ce3439c66521ab75f8a826cd8 retains the accepted four-operation native-evidence wire fixture and consumer codec
  - no overlapping open native-evidence Platform PR was found at admission
  - Game repository is read-only for exact compatibility evidence only
  - production deployment, PKI, secrets and live account/session mutation are outside authority
derived:
  - ambiguous witness-ahead account recovery must move only the database native generation forward and never lower retained authority
  - ambiguous witness-ahead signing trust recovery must conservatively revoke the affected profile version before a successor version can become trusted
  - requests-per-minute limiter semantics are best-effort throughput control; the independent capacity slots own the hard in-flight claim
unknown:
  - final Platform implementation SHA and exact candidate CI result
  - real non-production composed mTLS interoperability result
conflicts: []
first_failure:
  marker: agent-governance@b4791fbaf742863003e4afae0217c34cc6d66555
  evidence: task liveness rejected pr:none after PR #1389 opened; implementation validators themselves passed
rejected_hypotheses:
  - reusing closed Issue #1379/PR #1381 as the live implementation lineage
  - silently blessing a new empty retained-witness directory after history exists
  - automatically hiding witness-ahead recovery inside ordinary producer reads
changed_paths:
  - implementation candidate paths under the ownership set above
validation:
  - command: Agent Governance on b4791fbaf742863003e4afae0217c34cc6d66555
    result: FAIL
    evidence: branch_pr_identity_omitted; corrected by binding this packet to PR #1389
blockers:
  - exact-head CI pending on successor checkpoint commit
next_action: run focused and repository-required validation on the successor exact PR head, resolve any material failures, then prepare real-interoperability handoff evidence
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active under Issue #1388 and PR #1389
source_branch_evidence: pending
```

## Notes

The owner-launched alias supplies the cross-repository handoff scope. This packet does not authorize Game writes, production deployment, PKI/certificate changes, secrets, protected-environment actions or live account/session mutation.
