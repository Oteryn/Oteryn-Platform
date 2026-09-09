---
task_id: OTERYN-20260909-wp5-native-evidence-producer
governing_issue: 1379
required_reads:
  - docs/architecture/adr/0028-platform-accountid-cross-boundary-identity.md
  - docs/contracts/OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT.md
  - docs/architecture/SECURITY_ARCHITECTURE.md
  - docs/architecture/DATA_OWNERSHIP.md
  - docs/architecture/TEST_STRATEGY.md
  - docs/agents/BUILD_TEST_MATRIX.md
search_first:
  - Oteryn/Oteryn-Game#470 protected producer handoff
  - Identity/GameAuth/revocation/signing-trust paths
optional_reads: []
---

# OTERYN-20260909-wp5-native-evidence-producer

## Goal

Governing GitHub Issue: #1379 — canonical lifecycle authority for this task.

Implement the bounded Platform-native evidence producer accepted by the owner task and protected Game #470 without promoting Canary IDs, Passport/OAuth keys, legacy ticket credentials or production deployment state into native authority.

## Acceptance criteria

- [ ] Persist unique immutable UUIDv7 AccountId with one-time existing-Identity backfill.
- [ ] Persist positive native security generation and serialize every existing GameAuth revoke against it.
- [ ] Serve fresh/recovery account observations with one shared per-AccountId source ordering.
- [ ] Persist public signing-trust state with key/profile revisions and durable revocation.
- [ ] Retain an independent producer high-water witness; missing/regressed floors fail closed.
- [ ] Enforce the accepted strict/bounded V1/V2 wire and authenticated TLS-client boundary.
- [ ] Prove controlled counterpart interoperability plus security/database/concurrency rollback cases.
- [ ] Obtain independent exact-head P0/P1/P2=0 review, canonical CI, FULL Merge Queue and protected-main readback.

## Ownership

```yaml
owned_paths:
  - app/GameAuth/NativeEvidence/**
  - app/Http/Controllers/GameAuth/NativeEvidenceController.php
  - app/Http/Middleware/GameAuth/*NativeEvidence*.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - app/Identity/Support/CanonicalAccountId.php
  - app/Identity/Models/Identity.php
  - app/Identity/Actions/RevokeIdentityGameAuthorizations.php
  - config/game-auth.php
  - routes/internal.php
  - database/migrations/2026_09_09_120900_add_native_game_evidence_state.php
  - tests/Feature/GameAuth/NativeEvidenceProducerTest.php
  - docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260909-wp5-native-evidence-producer.md
modules:
  - Identity
  - GameAuth/NativeEvidence
dependencies:
  - Platform ADR 0028
  - protected Oteryn/Oteryn-Game#470 handoff
blockers:
  - none
cross_repository_tasks:
  - real authenticated Platform↔Game interoperability remains required before WP5 S3
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-09-09T12:08:17Z
head: UNKNOWN
branch: coord/wp5-platform-native-evidence-1379
pr: none
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
  - app/Http/Controllers/GameAuth/NativeEvidenceController.php
  - app/Http/Middleware/GameAuth/*NativeEvidence*.php
  - app/Http/Middleware/GameAuth/PreventSensitiveGameAuthResponseCaching.php
  - app/Identity/Support/CanonicalAccountId.php
  - app/Identity/Models/Identity.php
  - app/Identity/Actions/RevokeIdentityGameAuthorizations.php
  - config/game-auth.php
  - routes/internal.php
  - database/migrations/2026_09_09_120900_add_native_game_evidence_state.php
  - tests/Feature/GameAuth/NativeEvidenceProducerTest.php
  - docs/contracts/OTERYN_V2_NATIVE_EVIDENCE_PRODUCER_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260909-wp5-native-evidence-producer.md
proven:
  - Game #470 is protected at merge commit ae22132fd9ddb6c83f5bf386fdc267cb670d16aa.
  - Platform protected main allocation base is 13e207d5e826b2a24513664d7b8b6dc5d918f9b0.
  - Protected Game recovery contract requires V1/V2 account observations to share one AccountId security source-revision namespace and owner generation floor.
  - Platform current revocation seam is transactionally locked and is called by password, MFA, email, recovery and termination-sensitive mutations.
derived:
  - A separately persisted positive native_security_generation avoids source-only zero translation while preserving legacy game_auth_generation compatibility.
  - Retained source, security-generation and trust-state high-water outside the relational restore unit are required to detect database restore below previously committed producer/security truth.
unknown:
  - real deployed TLS terminator/client certificate/descriptor interoperability; intentionally deferred to authorized cross-repo qualification before WP5 S3
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - export legacy game_auth_generation plus one
  - promote Canary integer IDs to AccountId
  - reuse Passport/OAuth/Gateway/ticket credentials as native signing trust
  - store rollback witness only inside the same relational backup unit
changed_paths:
  - pending first implementation commit
validation:
  - command: not-run
    result: NOT_RUN
    evidence: exact branch candidate not committed yet
blockers:
  - none
next_action: publish the coherent implementation package on the dedicated branch and run exact-candidate focused validation
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository implementation lineage
source_branch_evidence: pending protected merge and source-ref readback
```

## Notes

The implementation deliberately does not select or rotate production keys/certificates. Signing trust stores public verification state only; private-key custody remains a separate authority.
