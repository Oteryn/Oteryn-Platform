---
task_id: OTERYN-20261012-platform-postgres-plan-fixes
governing_issue: 1474
required_reads:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
search_first:
  - docs/agents/LEGACY_TOKEN_BASELINE.json
optional_reads:
  - deploy/synology/compose.yml
---

# OTERYN-20261012 Platform PostgreSQL plan review fixes

## Goal

Governing GitHub Issue: #1474 (native-only migration) is the canonical lifecycle authority for this task. The top-level `governing_issue` frontmatter field is machine-required while this packet remains active.

Fix two review findings on the merged PostgreSQL migration plan (PR #1561, head `feb62ed`) and the matching ADR 0046 rule:

- From the first witnessed write on PostgreSQL, which comes at the start of P6 step 4 (a provenance-only witness-store init, then the first native admission trust publication attempt, which advances the trust-state witness before its database write), the PostgreSQL witness directory is never emptied, deleted or lowered, and the database is never emptied under it. Recovery keeps the witness and reconciles forward only. A clean-slate re-run of P3 with a witness reset is allowed only before that write. Retiring the MariaDB-era witness with the MariaDB volume after a successful cutover is kept distinct from the live PostgreSQL witness.
- P6 owns, as a cutover precondition, a bounded forward-only recovery for the missing-profile trust state (P6a), stated in the plan as invariants: provenance-only witness-store init before the attempt, durable attempt evidence with a restore-proof `closed-success` record, a repair only on proven P6 failure evidence, every other state unavailable and forward-only, and a successor key id with exactly one custody record (its own generation) and no database history. The mechanism goes to a P6a design note reviewed separately before implementation.
- The P6 smoke tests run against a running application on its loopback origins, not under `php artisan down`. The public edge is closed by switching only the two canonical Cloudflare Tunnel ingress rules to `http_status:503` through new bounded modes of the existing endpoint workflow, with a fail-closed refusal when any other ingress rule (wildcard, legacy or catch-all) targets the same loopback origins, an external closed-check before the smoke tests, no committed bypass secret and a defined reopen on unfreeze or rollback.

This is a documentation-only change. It does not change code, configuration, schema, deployment, secrets or data.

## Acceptance criteria

- [x] Plan P3, P5 and P6 and plan §6 state the witness rule and the access gate consistently.
- [x] ADR 0046 §8 and Reversal match the plan.
- [x] The merged PR #1561 task record is archived as completed.
- [x] No secret in the repository; no new legacy-token occurrence; baseline not raised.
- [ ] Exact-head CI and Agent Governance gates pass.

## Ownership

```yaml
owned_paths:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-postgres-plan-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-postgres-plan-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-postgres-plan.md
modules:
  - architecture-docs
  - operations-docs
dependencies:
  - PR 1561 (merged plan and ADR 0046)
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-13T00:20:00Z
head: UNKNOWN
branch: claude/platform-postgres-plan-fixes-20261012
pr: 1562
status: validating
context_routes:
  - architecture
  - database
owned_paths:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-postgres-plan-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-postgres-plan-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-postgres-plan.md
proven:
  - PR 1561 merged before two review findings on its last head were fixed
  - deploy.sh and deploy-impact.sh require PLATFORM_BIND_ADDRESS and GATEWAY_BIND_ADDRESS to stay loopback-only, and the Cloudflare Tunnel serves both public hostnames from those loopback origins
  - the native evidence witness fails closed when a database binding exists and its directory is empty, or when history is below the retained floor
  - NativeSigningTrustRegistry::publishTrustedKey advances the trust-state witness inside withNamespace before the relational insert commits, so a failed publication can leave the witness ahead
  - game-auth:native-evidence:reconcile takes exactly one of --account-id or --trust=fresh|recovery; trust reconciliation leaves the profile version revoked, and no console command calls publishNextProfileVersion today
  - a first publication whose transaction rolls back after the witness advanced leaves floor 1 with no profile row; reconcileSigningTrustAsRevoked refuses it (NativeEvidenceRecoveryReconciler.php:61-71) and publishTrustedKey refuses it (NativeSigningTrustRegistry.php:47-50), so P6 must own a forward-only missing-profile recovery
  - generate-native-admission-key.sh appends every generated key id to used.kids before returning the key, so a successor id always has its own generation record there
  - NativeEvidenceHighWaterWitness::reconcileStoreProvenance creates witness-store.id and its binding only when a namespace is entered, so with native admission off P3 leaves no store id
  - historicalKeyIdExists() only sees database history, so after a restore or relational loss a synthesized profile would hide earlier key ids; the missing-profile repair therefore needs proof that the state came from the P6 first publication, and successor key ids need a custody log outside the database
  - cloudflare-oteryn-endpoints.sh preserves unrelated ingress rules and bootstrap/app.php has no trusted-host allowlist, so another rule targeting the loopback origins would bypass a gate on the canonical rules
derived:
  - an edge gate on the two canonical tunnel ingress rules keeps the application testable on loopback during cutover without a maintenance bypass secret and without stopping a shared tunnel connector
unknown:
  - none
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Laravel maintenance bypass for smoke tests; it is cookie-based and does not serve machine callers such as health checks and the Gateway
  - LAN-only bind addresses as the gate; deploy scripts require loopback bindings and the tunnel still reaches loopback origins
  - stopping the tunnel connector; the tunnel may carry unrelated ingress rules
changed_paths:
  - docs/operations/PLATFORM_POSTGRES_MIGRATION_PLAN.md
  - docs/architecture/adr/0046-platform-persistence-moves-to-postgresql.md
  - docs/agents/tasks/active/OTERYN-20261012-platform-postgres-plan-fixes.md
  - docs/agents/tasks/archive/OTERYN-20261012-platform-postgres-plan.md
validation:
  - command: python3 tools/validation/adr_registry.py && python3 tools/validation/test_adr_registry.py; python3 tools/validation/legacy_token_ratchet.py; python3 tools/agents/checkpoint.py --tasks docs/agents/tasks/active --require-checkpoint; python3 tools/agents/documentation_ia.py
    result: PASS
    evidence: local run before push
blockers:
  - none
next_action: Lead review; on merge, archive this record.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository docs PR
source_branch_evidence: pending
```
