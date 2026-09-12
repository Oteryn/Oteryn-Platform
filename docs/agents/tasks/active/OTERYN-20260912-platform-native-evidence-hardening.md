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
  - accepted Game native-evidence consumer contract (read-only compatibility evidence)
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Game read-only exact consumer compatibility inspection; no Game mutation
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-12T18:52:04Z
head: UNKNOWN
branch: agent/platform-native-evidence-hardening-1388
pr: none
status: investigating
context_routes:
  - architecture
  - auth-identity
  - database
  - api
  - security
  - testing
owned_paths:
  - app/GameAuth/NativeEvidence/**
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
  - current witness persistence conditionally skips fsync when PHP does not expose it
  - current revocation advances the native-generation witness inside a database transaction before outer workflow commit is proven
  - no overlapping open native-evidence Platform PR was found at admission
  - Game repository is read-only for exact compatibility evidence only
  - production deployment, PKI, secrets and live account/session mutation are outside authority
derived:
  - witness-ahead/DB-behind recovery must be forward-only so recovery cannot resurrect older authorization
  - limiter semantics must be documented as approximate unless atomic reservation is proven
unknown:
  - exact Game protected revision and consumer fixture set to bind in final cross-repository evidence
  - exact repository validation delta required after implementation
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - reusing closed Issue #1379/PR #1381 as the live implementation lineage
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260912-platform-native-evidence-hardening.md
validation:
  - command: not-run
    result: NOT_RUN
    evidence: task admitted; implementation not yet complete
blockers:
  - none
next_action: inspect the exact durability, provenance, recovery, signing-trust and real-interoperability boundaries and implement the smallest fail-closed hardening delta
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active under Issue #1388
source_branch_evidence: pending
```

## Notes

The owner-launched alias supplies the cross-repository handoff scope. This packet does not authorize Game writes, production deployment, PKI/certificate changes, secrets, protected-environment actions or live account/session mutation.