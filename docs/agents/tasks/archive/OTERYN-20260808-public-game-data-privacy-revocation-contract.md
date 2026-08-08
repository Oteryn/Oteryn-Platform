---
task_id: OTERYN-20260808-public-game-data-privacy-revocation-contract
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/REMEDIATION_AUDIT_RISK_GATE.md
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/architecture/OTERYN_V2_INTEGRATION_ARCHITECTURE.md
search_first:
  - Issue #908
  - PR #916
  - PR #909 audit evidence
  - PR #903 source contract delivery
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
- [x] Exact-head self-review, Agent Governance and repository-selected CI passed; runtime/browser E2E was NOT_APPLICABLE for this architecture/contract-only repair.
- [x] Delivery PR #916 merged to protected `main`; Issue #908 closed completed.

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

## Terminal checkpoint

```yaml
policy_version: 2
checkpoint_version: 1
updated_at: 2026-08-08T18:05:00+02:00
status: completed
phase: closeout
terminal_pr_policy: delivered
session_id: chatgpt-20260808T1756+0200-issue-908
session_role: implementation_owner
execution_mode: github_only
execution_reason: The bounded architecture/contract repair was delivered through GitHub with exact-head validation and no production mutation.
task_kind: repair
implementation_authorized: true
context_pressure: medium
context_growth: stable
decomposition_decision: single
validation_level: full
validation_intensity: HEIGHTENED
validation_risk: high
validation_triggers: privacy-revocation,stale-while-servable-public-data,cache-search-cdn-propagation,cross-generation-rollback
validation_rationale: The contract governs confidentiality under stale serving and rollback.
self_review_result: PASS
self_review_exact_head: 278d5cf289ff95f7db200429f3ee05558dab007e
last_completed_step: PR #916 squash-merged to protected main as 8d03e57c17f55a3337b7232f5600e2dbff4e1a90 after exact-head Agent Governance and CI passed; Issue #908 closed completed.
issue: 908
branch: repair/issue-908
head: 278d5cf289ff95f7db200429f3ee05558dab007e
base_sha: a4f3d03a66aeba9a0e9fb9997d0614e18311c18f
pr: 916
merge_sha: 8d03e57c17f55a3337b7232f5600e2dbff4e1a90
context_routes:
  - public-game-data
  - privacy
  - architecture-governance
owned_paths:
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/agents/tasks/archive/OTERYN-20260808-public-game-data-privacy-revocation-contract.md
proven:
  - Issue #908 was P1/high, implementation-authorized and independently owned by this repair.
  - PR #916 changed exactly the PublicGameData projection contract and its active task record.
  - The contract now separates game-source freshness from Platform privacy-decision freshness.
  - Privacy-controlled public variants require monotonic/versioned Platform privacy evidence; an older allow cannot override a newer restrictive decision.
  - A restrictive privacy decision is authoritative at its accepted privacy revision/cutoff; purge, rebuild, search reindex and CDN invalidation are propagation rather than authority.
  - Failed, delayed or ambiguous propagation keeps affected fields fail closed and observable.
  - Privacy dependency unavailability cannot silently reuse an unproven cached allow.
  - Projection rebuild/generation rollback cannot roll privacy-controlled public presentation below the newest accepted privacy floor.
  - Future cutover validation explicitly covers delayed/out-of-order privacy events, invalidation failure, cache/search/CDN lag, concurrent projection refresh, privacy outage and rollback after deny.
  - Current Canary-compatible direct-read composition is explicitly not classified as defective by the native cutover repair.
  - Release-candidate head 278d5cf289ff95f7db200429f3ee05558dab007e passed Agent Governance run 31265908976 and CI run 31265909000.
  - Exact-head CI classify-changes, checkpoint validation, Composer metadata/audit, formatting, static analysis and runtime-tests completed successfully under repository-selected routing.
  - Native protocol contract/audit, Edge Security Emulation, Platform DB Outage Validation and Game Auth Ticket Concurrency also completed successfully on the release-candidate head; they were not required to justify the docs-only scope.
  - Runtime/browser E2E was NOT_APPLICABLE because Issue #908 forbade executable runtime/cache/CDN/schema/deployment changes.
  - PR #916 squash-merged as 8d03e57c17f55a3337b7232f5600e2dbff4e1a90 and GitHub closed Issue #908 as completed.
derived:
  - The existing integration architecture already establishes Platform privacy as the upper bound and delegates focused PublicGameData semantics to the accepted contract, so no additional architecture-file mutation was required.
  - The repair resolves the contract ambiguity before native PublicGameData projection/cache/CDN cutover without claiming that implementation or production activation exists.
unknown: []
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Game-source freshness alone can safely authorize stale public variants after a newer Platform privacy deny.
  - A cache purge request can postpone the authority of an already accepted restrictive privacy decision.
changed_paths:
  - docs/contracts/OTERYN_V2_PUBLIC_GAME_DATA_PROJECTION_CONTRACT.md
  - docs/agents/tasks/archive/OTERYN-20260808-public-game-data-privacy-revocation-contract.md
validation:
  - command: exact-head self-review on 278d5cf289ff95f7db200429f3ee05558dab007e
    result: PASS
    evidence: Full PR #916 diff checked against all Issue #908 acceptance criteria; zero material findings and zero review threads.
  - command: Agent Governance run 31265908976
    result: PASS
    evidence: Exact release-candidate head passed checkpoint and live governance validation.
  - command: CI run 31265909000
    result: PASS
    evidence: Exact release-candidate head passed repository-selected CI.
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: Architecture/contract-only repair; executable behavior was neither changed nor authorized.
blockers:
  - none
next_action: none
```

## Delivery evidence

- Finding: `OPA-SEC-0004` / Issue #908.
- Audit source: PR #909.
- Delivery PR: #916.
- Validated release-candidate head: `278d5cf289ff95f7db200429f3ee05558dab007e`.
- Agent Governance: run `31265908976` — PASS.
- CI: run `31265909000` — PASS.
- Protected-main delivery: `8d03e57c17f55a3337b7232f5600e2dbff4e1a90`.
- Issue state after merge: closed / completed.
- Production/runtime mutation: none.
