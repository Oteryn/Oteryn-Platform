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

Terminal repository outcome: `PLATFORM_NATIVE_EVIDENCE_READY_FOR_REAL_INTEROP`.

## Acceptance criteria

- [x] High-water storage requires and proves the accepted file + directory durability profile; missing/unavailable storage fails closed.
- [x] Witness-store provenance distinguishes first activation from retained-store loss/replacement when at least one independent authority record survives.
- [x] Native security generation can reconcile witness-ahead/DB-behind rollback aftermath only in the forward/revoking direction.
- [x] Password change/reset, recovery, MFA/email and termination callers remain covered by the shared revocation boundary.
- [x] `GAME_AUTH_NATIVE_EVIDENCE_REQUESTS_PER_MINUTE` semantics are explicitly truthful under concurrency.
- [x] Key rotation, key revocation and terminal profile revocation/recovery are distinct and documented/tested without private signing material.
- [x] Non-production real-interoperability qualification is specified for TLS 1.3 mTLS terminator provenance, all four operations and exact Game fixtures/revisions.
- [x] Relevant real MariaDB/filesystem/concurrency/process-restart recovery proof and repository-required exact-candidate validation pass for the frozen runtime head.

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
  - none for repository implementation/evidence readiness
cross_repository_tasks:
  - Oteryn/Oteryn-Game read-only exact consumer compatibility inspection; no Game mutation
```

## Context checkpoint

```yaml
checkpoint_version: 2
updated_at: 2026-09-12T21:04:24Z
head: 00ea71d79b19a2578068bb0b047a0c6f9814f03f
branch: agent/platform-native-evidence-hardening-1388
pr: 1389
status: ready
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
  - protected Platform main readback remains 2271fea9db1e202cacb206dab289efa4cefcec76
  - Issue #1388 is open and PR #1389 owns branch agent/platform-native-evidence-hardening-1388
  - PR #1389 is non-draft and mergeable with runtime implementation candidate 00ea71d79b19a2578068bb0b047a0c6f9814f03f validated against base 2271fea9db1e202cacb206dab289efa4cefcec76
  - witness persistence requires working PHP fsync for both the written file and containing directory
  - witness-store identity is durably bound across retained storage and the Platform database and a replacement empty store fails closed while binding/history survives
  - native security-generation witness-ahead rollback recovery is explicit and forward-only
  - ambiguous signing-trust rollback recovery is conservative-revoked and terminal profile revocation is preserved through successor profile versions
  - password change/reset, MFA, email, recovery and termination families share RevokeIdentityGameAuthorizations
  - requests-per-minute limiting is documented as best-effort throughput control while NativeEvidenceCapacity owns the hard two-in-flight claim
  - protected Game main readback remains 489e3e390a1bce1ce3439c66521ab75f8a826cd8 with the accepted four-operation consumer wire fixture/codec
  - Game Auth Ticket Concurrency run 34718503838 passed the independent-process MariaDB proof on runtime head 00ea71d79b19a2578068bb0b047a0c6f9814f03f, including retained-filesystem account rollback/restart/reconciliation and signing-trust rollback/restart/reconciliation with immutable successor-profile recovery
  - CI run 34718503798 passed formatting, static analysis, PHPUnit and required platform-gate on runtime head 00ea71d79b19a2578068bb0b047a0c6f9814f03f
  - Native protocol contract run 34718503901 and contract-audit run 34718503788 passed on the same runtime head
  - Agent Governance run 34718503779, Edge Security run 34718503783 and Platform DB Outage run 34718503793 passed on the same runtime head
  - Phase 7 Production-Like Validation run 34718503807 and Build Synology Staging Images run 34718503850 passed on the same runtime head
  - Portal Acceptance Contract run 34718503839 passed strict coverage and the complete zero-retry account lifecycle on the same runtime head
  - Acceptance E2E and Visual UX run 34718503805 passed routed smoke, browser portability, responsive, resilience and keyboard accessibility profiles and wrote non-secret evidence on the same runtime head
  - PR #1389 has zero review submissions, zero inline review threads and zero PR comments at repository-readiness readback
  - production deployment, PKI, secrets, live account/session mutation and Game writes remain outside authority
derived:
  - ambiguous witness-ahead recovery never lowers retained authority and therefore cannot resurrect an older authorization state
  - a pre-existing native_security_generation above one is not proof that native-evidence producer history already existed
  - simultaneous loss or rollback of both the relational provenance/history and independently retained witness volume removes both authorities and is not a qualified restore topology; it cannot be promoted to a recovery PASS
  - the earlier SQLite-only recovery evidence gap is closed by an exact-head real MariaDB + retained filesystem + independent PHP process proof without weakening migration or anti-rollback guards
  - Platform-local PASS is sufficient only for PLATFORM_NATIVE_EVIDENCE_READY_FOR_REAL_INTEROP, not composed or production proof
unknown:
  - real non-production composed TLS 1.3 mTLS Platform↔Game interoperability result
conflicts: []
first_failure:
  marker: none
  evidence: no unresolved failure remains on validated runtime head 00ea71d79b19a2578068bb0b047a0c6f9814f03f
rejected_hypotheses:
  - reusing closed Issue #1379/PR #1381 as the live implementation lineage
  - describing the Laravel requests-per-minute limiter as a strict atomic concurrent-admission reservation
  - silently blessing a replacement retained-witness directory while surviving authority history identifies the retained store
  - automatically hiding witness-ahead account or trust recovery inside ordinary producer reads
  - treating native_security_generation greater than one as prior native-evidence producer activation
  - claiming repository server-variable injection as composed real mTLS proof
  - treating SQLite-only feature coverage as sufficient evidence for the real-DB/restart acceptance boundary
changed_paths:
  - app/Console/Commands/ReconcileNativeEvidence.php
  - app/GameAuth/NativeEvidence/NativeEvidenceHighWaterWitness.php
  - app/GameAuth/NativeEvidence/NativeEvidenceRecoveryReconciler.php
  - app/GameAuth/NativeEvidence/NativeEvidenceSource.php
  - app/GameAuth/NativeEvidence/NativeSigningTrustRegistry.php
  - database/migrations/2026_09_12_204800_harden_native_game_evidence_authority.php
  - docs/agents/evidence/OTERYN-20260912-platform-native-evidence-hardening/REAL_INTEROP_QUALIFICATION.md
  - docs/agents/tasks/active/OTERYN-20260912-platform-native-evidence-hardening.md
  - docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md
  - tests/Feature/GameAuth/NativeEvidenceHardeningTest.php
  - tests/Feature/GameAuth/Concurrency/GameTicketConcurrencyTest.php
validation:
  - command: CI / runtime-tests + platform-gate on 00ea71d79b19a2578068bb0b047a0c6f9814f03f
    result: PASS
    evidence: run 34718503798; formatting, static analysis, PHPUnit and required aggregate gate succeeded
  - command: independent-process MariaDB native-evidence rollback/restart/reconciliation proof on 00ea71d79b19a2578068bb0b047a0c6f9814f03f
    result: PASS
    evidence: Game Auth Ticket Concurrency run 34718503838; account and signing-trust recovery use the real MariaDB service, retained filesystem witness state and fresh PHP artisan processes after database rollback/reconnect
  - command: Native protocol contract + audits on 00ea71d79b19a2578068bb0b047a0c6f9814f03f
    result: PASS
    evidence: runs 34718503901 and 34718503788
  - command: security/outage/production-like/build gates on 00ea71d79b19a2578068bb0b047a0c6f9814f03f
    result: PASS
    evidence: runs 34718503783, 34718503793, 34718503807 and 34718503850
  - command: Portal Acceptance Contract on 00ea71d79b19a2578068bb0b047a0c6f9814f03f
    result: PASS
    evidence: run 34718503839; strict portal closure and complete zero-retry account lifecycle succeeded
  - command: Acceptance E2E and Visual UX routed profile on 00ea71d79b19a2578068bb0b047a0c6f9814f03f
    result: PASS
    evidence: run 34718503805; smoke, browser portability, responsive, resilience and keyboard accessibility profiles succeeded and non-secret evidence was written
  - command: final PR review/readback before checkpoint
    result: PASS
    evidence: PR #1389 mergeable=true with zero submitted reviews, zero inline review threads and zero PR comments at readback
  - command: real composed non-production TLS 1.3 mTLS interoperability
    result: NOT_RUN
    evidence: separately gated environment/PKI/consumer execution is intentionally not authorized by this repository task; procedure is recorded in REAL_INTEROP_QUALIFICATION.md
blockers:
  - none for PLATFORM_NATIVE_EVIDENCE_READY_FOR_REAL_INTEROP
next_action: pass PR #1389 through the organization-governed exact-head protected integration path; after integration, execute REAL_INTEROP_QUALIFICATION only in a separately authorized non-production topology before any Game WP5 S3 composed-readiness claim
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository delivery branch for mergeable PR #1389; retain no post-merge task authority on the source ref
source_branch_evidence: PR #1389 remains open and unmerged; verify source-ref disappearance only after normal protected merge
```

## Notes

Repository implementation/evidence terminal state is `PLATFORM_NATIVE_EVIDENCE_READY_FOR_REAL_INTEROP` on runtime candidate `00ea71d79b19a2578068bb0b047a0c6f9814f03f`. `REAL_INTEROP_PROVEN` and `PRODUCTION_PROVEN` are explicitly not claimed. The owner-launched alias does not authorize Game writes, production deployment, PKI/certificate changes, secrets, protected-environment actions or live account/session mutation.
