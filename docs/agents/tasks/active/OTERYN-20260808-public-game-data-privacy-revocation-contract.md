---
task_id: OTERYN-20260808-public-game-data-privacy-revocation-contract
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
search_first:
  - Issue #908
  - PR #909 audit evidence
  - PR #903 source contract delivery
  - open PRs touching PublicGameData projection/privacy architecture
optional_reads:
  - Issue #902
---

# OTERYN-20260808-public-game-data-privacy-revocation-contract

## Goal

Repair Issue #908 by making native PublicGameData privacy-decision freshness and revocation semantics fail closed independently of game-source freshness, without authorizing runtime, cache/CDN, schema, deployment, production or external-repository changes.

## Acceptance criteria

- [ ] Game-source freshness and Platform privacy-decision freshness are explicitly independent authorities.
- [ ] Public variants bind to monotonic/versioned privacy decision evidence that prevents an older allow from overriding a newer deny.
- [ ] Restrictive privacy changes define an immediate authoritative visibility cutoff plus deterministic cache/search/CDN acknowledgement semantics.
- [ ] Failed, delayed or ambiguous invalidation fails closed for affected private/presentation fields.
- [ ] Privacy dependency outage cannot reuse an unproven cached allow decision.
- [ ] Rebuild, rollback and generation switch cannot resurrect public output predating a newer deny.
- [ ] Required validation scenarios cover delayed/out-of-order preference events, invalidation failure, cache/CDN/search lag, concurrent refresh, privacy dependency outage and rollback after deny.
- [ ] Current Canary-compatible direct-read behavior is not falsely classified as defective.
- [ ] Exact-head self-review and repository-selected CI pass; runtime/browser E2E is NOT_APPLICABLE because this repair changes architecture/contracts only.

## Ownership

```yaml
owned_paths:
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260808-public-game-data-privacy-revocation-contract.md
  - docs/agents/tasks/archive/OTERYN-20260808-public-game-data-privacy-revocation-contract.md
modules:
  - public-game-data
  - privacy
  - architecture-governance
dependencies:
  - Issue #908
  - Issue #902 / PR #903 source contract
blockers:
  - none
cross_repository_tasks:
  - none
shared_paths:
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-08T17:56:00+02:00
head: a4f3d03a66aeba9a0e9fb9997d0614e18311c18f
branch: repair/issue-908
pr: none
status: implementing
phase: implement
execution_mode: github_only
execution_reason: Documentation/contract repair is safely executable through GitHub contents API and exact-head GitHub Actions validation.
session_id: chatgpt-20260808T1756+0200-issue-908
session_role: implementation_owner
context_pressure: medium
context_growth: stable
decomposition_decision: single
validation_level: focused
validation_gate:
  version: 2
  intensity: HEIGHTENED
  classified_by: chatgpt-20260808T1756+0200-issue-908
  classified_at: 2026-08-08T17:56:00+02:00
  risk: high
  triggers:
    - privacy revocation
    - stale-while-servable public data
    - cache/search/CDN propagation
    - cross-generation rollback
  unknown_or_conflict: []
  rationale: The contract controls future confidentiality behavior under stale serving and rollback, so revocation ordering and fail-closed semantics require heightened validation.
  self_review:
    result: PENDING
    exact_head: none
    evidence: []
owned_paths:
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260808-public-game-data-privacy-revocation-contract.md
proven:
  - Issue #908 is open, priority P1, risk high, implementation_authorized=true and agent:ready.
  - No repair/issue-908 branch existed before claim.
  - Open PR #882 owns unrelated homepage-template paths; open PRs #541 and #338 do not overlap this contract.
  - Current contract permits game-source last-known-good stale serving and requires privacy invalidation, but lacks a distinct privacy decision generation/cutoff/failure/rollback rule.
  - Current Canary-compatible PublicCharacterProfileService composes live Platform privacy preferences and is not the defect claimed by Issue #908.
derived:
  - The smallest complete repair is a contract-level privacy authority fence plus a narrow architecture invariant; no runtime implementation is required or authorized.
unknown: []
conflicts: []
first_failure:
  marker: privacy-revocation-freshness-gap
  evidence: Issue #908 / audit PR #909
rejected_hypotheses:
  - Game-source freshness alone can safely authorize stale public variants after a newer Platform privacy deny.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260808-public-game-data-privacy-revocation-contract.md
validation:
  - command: repository governance/path review
    result: PASS
    evidence: Branch created from protected main a4f3d03a66aeba9a0e9fb9997d0614e18311c18f; ownership is non-overlapping.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: Issue #908 explicitly authorizes architecture/contract reconciliation only and forbids runtime/cache/CDN/schema/deployment changes.
blockers:
  - none
next_action: Amend the PublicGameData projection contract and integration architecture with monotonic privacy decision evidence, revocation cutoff, fail-closed invalidation and rollback-safe validation semantics.
```

## Notes

Issue #908 is one-Issue/one-owner remediation. No runtime, schema, cache/CDN implementation, deployment, production action, secret access or external-repository mutation is authorized.