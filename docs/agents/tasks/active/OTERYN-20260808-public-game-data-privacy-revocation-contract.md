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

- [x] Game-source freshness and Platform privacy-decision freshness are explicitly independent authorities.
- [x] Public variants bind to monotonic/versioned privacy decision evidence that prevents an older allow from overriding a newer deny.
- [x] Restrictive privacy changes define an immediate authoritative visibility cutoff plus deterministic cache/search/CDN acknowledgement semantics.
- [x] Failed, delayed or ambiguous invalidation fails closed for affected private/presentation fields.
- [x] Privacy dependency outage cannot reuse an unproven cached allow decision.
- [x] Rebuild, rollback and generation switch cannot resurrect public output predating a newer deny.
- [x] Required validation scenarios cover delayed/out-of-order preference events, invalidation failure, cache/CDN/search lag, concurrent refresh, privacy dependency outage and rollback after deny.
- [x] Current Canary-compatible direct-read behavior is not falsely classified as defective.
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
updated_at: 2026-08-08T18:02:00+02:00
head: 5b2e44490254fbef9f4554e03b1a0c5195f038f6
branch: repair/issue-908
pr: 916
status: validating
phase: validate
execution_mode: github_only
execution_reason: Documentation/contract repair is safely executable through GitHub contents API and exact-head GitHub Actions validation.
session_id: chatgpt-20260808T1756+0200-issue-908
session_role: implementation_owner
context_pressure: medium
context_growth: stable
decomposition_decision: single
validation_level: full
owned_paths:
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260808-public-game-data-privacy-revocation-contract.md
proven:
  - Issue #908 is open, priority P1, risk high, implementation_authorized=true and agent:ready.
  - No repair/issue-908 branch existed before claim.
  - Open PR #882 owns unrelated homepage-template paths; open PRs #541 and #338 do not overlap this contract.
  - PR #916 is the one-Issue/one-owner delivery PR for this repair.
  - The contract now separates game-source freshness from Platform privacy-decision freshness.
  - The contract now requires monotonic privacy decision evidence, an immediate restrictive visibility cutoff, fail-closed propagation/fencing, privacy-dependency failure semantics and rollback-safe privacy floors.
  - Required future validation includes delayed/out-of-order privacy evidence, invalidation failure, cache/search/CDN lag, concurrent game refresh, privacy outage and rollback after deny.
  - Current Canary-compatible direct-read composition is explicitly not classified as defective by this native cutover contract repair.
derived:
  - The existing integration architecture already establishes Platform privacy as the upper bound and delegates focused PublicGameData semantics to this accepted contract; no additional architecture-file mutation is required to close Issue #908.
unknown: []
conflicts: []
first_failure:
  marker: checkpoint-validation-format
  evidence: Agent Governance run 31265778753 rejected nested validation_gate metadata in the task checkpoint; contract semantics were not the failing path.
rejected_hypotheses:
  - Game-source freshness alone can safely authorize stale public variants after a newer Platform privacy deny.
  - A cache purge request can postpone the authority of an already accepted restrictive privacy decision.
changed_paths:
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/agents/tasks/active/OTERYN-20260808-public-game-data-privacy-revocation-contract.md
validation:
  - command: full PR #916 diff and Issue #908 acceptance self-review on implementation head 5b2e44490254fbef9f4554e03b1a0c5195f038f6
    result: PASS
    evidence: All eight semantic acceptance requirements are represented directly in the contract; no runtime/schema/cache/CDN/deployment/external-repository path changed.
  - command: Agent Governance run 31265778753
    result: FAIL
    evidence: Task checkpoint formatting only; nested validation_gate was unsupported and is removed by this checkpoint update.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: Issue #908 authorizes architecture/contract reconciliation only and forbids executable runtime/cache/CDN/schema/deployment changes.
blockers:
  - none
next_action: Validate the updated task checkpoint, record exact-final-head self-review, and require Agent Governance plus repository-selected CI to pass before merge.
```

## Remediation risk gate

```yaml
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
    exact_head: pending-final-task-checkpoint-head
    acceptance_checked: true
    full_diff_checked: true
    related_prs_checked: true
    negative_paths_checked: true
    rollback_checked: true
    compatibility_checked: true
    findings: []
    evidence:
      - PR #916 full diff reviewed against Issue #908 acceptance on implementation head 5b2e44490254fbef9f4554e03b1a0c5195f038f6.
      - Final exact-head review will be recorded durably on PR #916 after this checkpoint commit establishes the release-candidate head.
```

## Notes

Issue #908 is one-Issue/one-owner remediation. No runtime, schema, cache/CDN implementation, deployment, production action, secret access or external-repository mutation is authorized.
