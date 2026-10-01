---
task_id: OTERYN-20260930-prem-p-premium-snapshot-contract
governing_issue: 1431
required_reads:
  - docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md
  - docs/contracts/OTERYN_V2_CHARACTER_BOOTSTRAP_INTENT_PRODUCER_CONTRACT.md
search_first:
  - app/Http/Middleware/GameAuth mTLS peer middleware
  - app/Admin and app/Audit operator RBAC/MFA/audit patterns
optional_reads: []
---

# OTERYN-20260930-prem-p-premium-snapshot-contract

## Goal

Governing GitHub Issue: #1431 (https://github.com/Oteryn/Oteryn-Platform/issues/1431). It is the canonical lifecycle authority for this task. Parent: #322. Cross-repository coordination id: `OTV2-PREMIUM-DELIVERY`, Platform producer slice PREM-P.

This packet covers delivery item 1 of #1431, the contract candidate `docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md`. The candidate:

- defines product `oteryn.premium_time` v1 as a Profile-B entitlement;
- defines operator-only grant, merge and revocation semantics;
- fixes the product/version authority policy values;
- defines the mTLS snapshot read wire;
- records Platform decisions on the Game proposal `PREMIUM-DELIVERY-0` (Oteryn/Oteryn-Game#1369);
- adds reusable JSON Schema fixtures.

The packet is documentation only. The owner accepts the candidate.

## Acceptance criteria

- [x] The contract covers the product, grant and merge rules, revocation, policy values, the endpoint, the wire and ordering, and records a decision on every item in the Game proposal.
- [x] Each conflict with an accepted Platform contract is stated.
- [x] The fixtures validate: 5 valid snapshots pass the schema and cross-field rules, and 20 invalid snapshot cases and 9 invalid request cases are rejected.
- [ ] The owner accepts the candidate, which happens at PR review.

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md
  - docs/contracts/fixtures/premium-snapshot-v1/**
  - composer.lock
  - docs/agents/tasks/active/OTERYN-20260930-prem-p-premium-snapshot-contract.md
  - docs/agents/tasks/archive/OTERYN-20260930-prem-p-premium-snapshot-contract.md
modules:
  - ProductsEntitlements
dependencies:
  - docs/contracts/OTERYN_V2_ENTITLEMENT_GAME_DELIVERY_CONTRACT.md (accepted, unchanged)
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Game PREMIUM-DELIVERY-0 (#1369) consumer proposal; read by relay only, no Game repository access
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-01T07:30:00Z
head: 30091a999c6bf19c99b7c3142da6a4008489931c
branch: claude/prem-p-premium-snapshot-contract
pr: 1432
status: validating
context_routes:
  - architecture
  - api
  - admin-rbac
  - security
owned_paths:
  - docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md
  - docs/contracts/fixtures/premium-snapshot-v1/**
  - composer.lock
  - docs/agents/tasks/active/OTERYN-20260930-prem-p-premium-snapshot-contract.md
  - docs/agents/tasks/archive/OTERYN-20260930-prem-p-premium-snapshot-contract.md
proven:
  - The generic Profile-B contract requires per-product finite lease, refresh, skew and stale-use values before activation.
  - An mTLS peer pattern exists (RequireCharacterBootstrapIntentMtlsPeer) with terminator provenance and an exact subject match.
  - The operator pattern exists as auth + mfa.confirmed + admin.permission with AdminAuditRecorder (marketplace wallet adjustment).
  - The canonical AccountId is identities.account_id, a unique lower-case UUID.
  - No ProductsEntitlements code exists; the module is PLANNED in MODULE_CATALOG.
  - The architect confirmed decisions 1-4 on #1431 (comment 5926527459) and required the constant snapshot member producer_profile = oteryn.entitlement.profile_b.v1, which the Game consumer (PREMIUM-DELIVERY-0 section 4, PREM-1a) requires.
  - The generic Profile-B contract classifies due or failed refresh evidence as STALE_WITHIN_BOUND, so with the accepted stale DENY policy benefit stops at refresh_after without newer accepted evidence (Codex review 4153022073 on PR 1432); sections 3, 4.5, 8.2, 8.5, D13 and the manifest classification text now say so.
  - AccountId is a canonical UUIDv7 (OTERYN_V2_ACCOUNT_IDENTITY_CONTRACT) and entitlement_id is UUIDv7 (section 2.1), so both schemas require version 7 and the RFC variant (Codex review 4153365059 on PR 1432); a non-UUIDv7 account_id is a malformed request (400), not an unknown account (404).
derived:
  - One entitlement per account and product keeps lifecycle_revision a single monotonic sequence.
  - A per-account locked authority row gives a strictly increasing authority_revision that is ordered with grants.
unknown:
  - The exact Game proposal text; only the relayed summary was used, and Oteryn-Game was not read.
conflicts:
  - A Game consumer treating Platform entitlement_state=ACTIVE alone as authorization would conflict with the generic contract; contract section 8.2 is binding.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Treating the composer audit failure as this PR's defect was rejected; it is the league/commonmark advisory fixed in PR 1435, ported here (composer.lock only) and a no-op once PR 1435 merges.
  - Permitting benefit until authority_valid_until without a fresh snapshot was rejected; it would make the refresh-due interval a stale grace that the accepted DENY policy forbids.
  - A fixed refresh_after of 40 min was rejected because it can fall after a clipped cutoff.
  - A new entitlement_id per grant was rejected because it would break per-entitlement lifecycle ordering for merged intervals.
changed_paths:
  - docs/contracts/OTERYN_V2_PREMIUM_TIME_SNAPSHOT_CONTRACT.md
  - docs/contracts/fixtures/premium-snapshot-v1/**
  - composer.lock
  - docs/agents/tasks/active/OTERYN-20260930-prem-p-premium-snapshot-contract.md
validation:
  - command: fixture check (jsonschema Draft 2020-12 plus cross-field rules), local
    result: PASS
    evidence: 5 valid accepted, 29 invalid rejected (4 new UUIDv7 cases); largest valid body 650 bytes
  - command: python tools/agents/checkpoint.py docs/agents/tasks/active/OTERYN-20260930-prem-p-premium-snapshot-contract.md --require-checkpoint
    result: PASS
    evidence: local run before push
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: documentation-only contract candidate; no runtime path changes
blockers:
  - none
next_action: exact-head CI of the UUIDv7 schema fix on PR 1432, then carry UUIDv7 validation into PR 1433 and merge through PR 1436
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

Documentation only. There is no code, migration, route, configuration, secret or workflow change. Jira: no mapped KAN Story exists, so synchronization is pending and no Jira item was created.
