---
task_id: OTERYN-20260814-liveops-architecture
mode: architecture
issue: 1046
status: implementing
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-core
phase: implement
execution_mode: github_connector
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/MODULE_CATALOG.md
  - docs/architecture/adr/0029-platform-world-channel-identity-and-topology.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/adr/0032-portal-composition-tracking-and-server-system-ownership.md
  - docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
search_first:
  - active Platform tasks, open PRs, Issues #317/#319/#320, existing LiveOps ownership and server-save source
optional_reads:
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
---

# OTERYN-20260814-liveops-architecture

## Goal

Define one focused canonical Platform architecture for `LiveOps` current world/service state so the first WorldStatus/Maintenance implementation slice can proceed without fabricating runtime truth and without accessing or changing an external game repository.

## Acceptance criteria

- [x] Current `main`, active tasks, related open PRs and blocked Character-lifecycle predecessors are reconciled from live state.
- [x] Existing accepted runtime-status and portal-composition authority is reused rather than duplicated.
- [x] Focused `LIVEOPS_ARCHITECTURE.md` defines source authority, evidence/freshness, applicability, history, cache, failure, observability and PublicPortal consumption boundaries.
- [x] Server-save timing remains explicitly unknown until an authoritative source is proven; no conventional schedule is invented.
- [ ] `ARCHITECTURE_AUTHORITY.md` routes LiveOps to the focused architecture and accepted runtime-status contract.
- [x] `MODULE_CATALOG.md` was reconciled against the focused design; its existing `LiveOps | PLANNED` status and responsibilities remain semantically correct, so no status promotion or content rewrite is justified by architecture-only work.
- [x] ADR allocation is `NOT_APPLICABLE`: no new durable ownership decision is introduced beyond accepted ADR 0029/0031/0032 and the accepted runtime-status consumer contract.
- [x] The next implementation handoff is bounded to Platform-owned paths and requires exact producer evidence before exposing a capability.
- [ ] Exact final-head documentation/governance validation and PR hygiene pass.
- [x] Runtime/browser E2E is `NOT_APPLICABLE` because this task changes architecture authority only and intentionally implements no user-facing/runtime route.
- [ ] PR is merged, Issue #1046 is closed, task is archived and ownership released.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-liveops-architecture.md
  - docs/architecture/LIVEOPS_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
modules:
  - LiveOps architecture
  - PublicPortal consumption boundary only
dependencies:
  - ADR 0029
  - ADR 0031
  - ADR 0032
  - docs/contracts/OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT.md
blockers: []
cross_repository_tasks:
  - none
```

## Context pressure

```yaml
policy_version: 2
task_kind: architecture
context_pressure: medium
context_growth: stable
context_score: 7
estimate_confidence: high
decomposition_decision: single
decomposition_reason: one cohesive Platform architecture boundary with no runtime or external-repository implementation
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T13:07:10Z
head: 696e2e868c78426b9ecc9df41e1c437e99ba375e
branch: docs/OTERYN-20260814-liveops-architecture
pr: none
status: implementing
context_routes:
  - architecture
  - testing
  - agent-governance
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260814-liveops-architecture.md
  - docs/architecture/LIVEOPS_ARCHITECTURE.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
proven:
  - Trusted main is 696e2e868c78426b9ecc9df41e1c437e99ba375e.
  - The two pre-existing active Platform task records are externally or production blocked and do not own LiveOps architecture.
  - Character deletion Issue #317 and rename Issue #319 are blocked on accepted Oteryn-v2 Character Authority operation semantics; world transfer #320 is additionally blocked on a product decision.
  - No open LiveOps PR was found on the trusted base.
  - ADR 0032 already assigns current operational/schedule/runtime state to LiveOps and PublicPortal Today/World Hub composition to PublicPortal.
  - OTERYN_V2_RUNTIME_STATUS_PROJECTION_CONTRACT is an accepted Platform consumer architecture contract and already defines fresh/stale/unavailable/invalid evidence semantics, canonical WorldId/ChannelId scope and no-fabricated-offline/zero rules.
  - Current Module Catalog already marks LiveOps PLANNED and its existing responsibilities/invariants agree with the focused design.
  - Repository search on the trusted base returned no `server save` or `server_save` implementation/source.
  - Issue #1046 and branch docs/OTERYN-20260814-liveops-architecture were created from trusted main.
derived:
  - LiveOps is the first safe unowned architecture-ready portal slice after higher-priority items that are blocked on external/protected authority.
  - A focused architecture document can make the accepted LiveOps consequences implementation-ready without allocating a new ADR because no accepted durable ownership rule changes.
unknown:
  - exact Oteryn-v2 runtime-status producer transport/IDL/cadence/fencing encoding
  - authoritative server-save schedule source and recurrence semantics
  - exact future LiveOps persistence/cache technology
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Platform configuration alone can be promoted to native runtime truth.
  - Missing or stale state can be represented as offline, zero or no scheduled activity.
  - A conventional Tibia/OTS server-save time can be assumed without an authoritative source.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260814-liveops-architecture.md
  - docs/architecture/LIVEOPS_ARCHITECTURE.md
validation:
  - command: repository source/authority reconciliation
    result: PASS
    evidence: accepted ADR 0032 and runtime-status projection contract align with the focused boundary
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture-only task creates no runtime route, persistence, API or frontend behavior
blockers: []
next_action: Route LiveOps in ARCHITECTURE_AUTHORITY.md, create the exact-head architecture PR, self-review the full diff, then complete required documentation/governance CI and terminal closeout.
```

## Notes

No Oteryn-v2/Canary repository was accessed. No production/protected environment, secret, runtime or deployment operation is authorized or performed by this task.
