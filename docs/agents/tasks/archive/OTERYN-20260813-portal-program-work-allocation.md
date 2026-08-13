---
task_id: OTERYN-20260813-portal-program-work-allocation
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
status: completed
required_reads:
  - AGENTS.md
  - docs/agents/CONTEXT_HANDOFF.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/architecture/PORTAL_COMPLETENESS_ARCHITECTURE.md
search_first:
  - active portal completion tasks and PRs
optional_reads:
  - docs/architecture/PLAYER_COMPANION_ARCHITECTURE.md
  - docs/agents/programs/GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md
---

# OTERYN-20260813-portal-program-work-allocation

## Goal

Turn the current portal-completion backlog and the accepted Tibia/RubinOT/TibiaPal-derived product decisions into one durable execution-allocation companion with explicit ownership, dependency routing and terminal completion criteria, while keeping `OTERYN_PORTAL_COMPLETION` as the only work-selection authority.

## Acceptance criteria

- [x] The existing `OTERYN_PORTAL_COMPLETION` programme remains the single orchestration and selection authority; no duplicate scheduler/programme is created.
- [x] A canonical work-allocation record classifies the discussed P0-P3 portal areas as `DONE`, `ARCHITECTURE_READY`, `OPEN`, `BLOCKED`, `IN_PROGRESS`, `DEFERRED`, `DECISION_REQUIRED` or `REJECTED`.
- [x] Core Account Center/Character Portfolio and PublicPortal Today are explicitly allocated and cannot disappear from completion accounting.
- [x] Execution roles are model-agnostic: `ARCHITECTURE_COORDINATOR`, `IMPLEMENTATION_OWNER`, `OWNER_DECISION` and `PROTECTED_ENV_OPERATOR`; Codex suitability is a separate non-authorizing attribute.
- [x] Codex suitability never constitutes permission to consume owner-funded Codex/OpenAI quota; every such invocation still requires explicit owner approval for that exact task/use.
- [x] The companion does not define a competing fallback sequence; detailed next-task selection remains in `OTERYN_PORTAL_COMPLETION.md`.
- [x] PlayerCompanion follow-up ordering matches `PLAYER_COMPANION_ARCHITECTURE.md`.
- [x] Game Catalog work reuses `GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM.md` and keeps Platform-only work separate from server/game work requiring separate authorization.
- [x] Issue #489 and #490 are split by their actual concern/authority boundaries rather than treated as one simplified workstream each.
- [x] `project_lane: oteryn-platform-core` prevents broad programme text from being misclassified into the Bazaar lane.
- [x] Existing architecture authority, module boundaries and live-state-first programme rules are preserved.
- [x] Repaired final head passes repository governance/CI and all material PR review threads are resolved.
- [x] PR #1031 is squash-merged and the task record is moved from `active/` to `archive/` with post-merge evidence.

## Ownership

```yaml
owned_paths:
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/tasks/active/OTERYN-20260813-portal-program-work-allocation.md
  - docs/agents/tasks/archive/OTERYN-20260813-portal-program-work-allocation.md
modules:
  - PublicPortal
  - Accounts
  - Characters
  - Wiki
  - GameCatalog
  - PublicGameData
  - LiveOps
  - Downloads
  - PlatformAPI
  - PlayerCompanion
  - OperationsObservability
  - PublicEdge
dependencies:
  - OTERYN_PORTAL_COMPLETION
  - GAME_CATALOG_PRODUCTION_COMPLETION_PROGRAM for detailed Game Catalog decomposition
  - PR #1028 is independent active PlayerCompanion implementation and was not modified by this documentation task
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-13T21:39:00+02:00
head: 9f7e5587bf799e173473245ee0c77f781d754623
branch: docs/portal-program-allocation-final
pr: 1031
status: completed
phase: closeout
project_lane: oteryn-platform-core
task_kind: implementation
implementation_authorized: true
execution_mode: chat-github
execution_reason: narrow documentation/programme corrections were safely executed through the GitHub connector without owner-funded AI invocation
context_pressure: medium
context_growth: stable
decomposition_decision: single
validation_level: focused
session_rotation_count: 0
heavy_validation_runs: 3
stale_takeover_count: 0
human_interruptions: 0
context_routes:
  - architecture
  - agent-governance
owned_paths:
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/tasks/archive/OTERYN-20260813-portal-program-work-allocation.md
proven:
  - OTERYN_PORTAL_COMPLETION remains the single programme selector and the merged allocation explicitly acts only after canonical live selection.
  - The merged allocation includes explicit Account Center/Character Portfolio and PublicPortal Today workstreams, accepted PlayerCompanion ordering, specialized Game Catalog routing and split #489/#490 concern ownership.
  - Execution roles are model-agnostic and Codex suitability is explicitly non-authorizing.
  - Final PR head 2d77f8e29848082f5f008fbfc230a09123d19504 included current main e4dab032387b9a4c3fa86fd52b1b74f2bd2529c5 as a merge parent before readiness.
  - Final PR #1031 diff contains exactly four intended documentation/programme files and no runtime or external-repository mutation.
  - Review threads PRRT_kwDOTcsYjs6ZDWm5, PRRT_kwDOTcsYjs6ZDWm- and PRRT_kwDOTcsYjs6ZDWnC were all resolved before merge.
  - All eight workflow runs on final head 2d77f8e29848082f5f008fbfc230a09123d19504 completed successfully, including CI 31736757370 and Agent Governance 31736757390.
  - PR #1031 auto-merged with squash to protected main as 9f7e5587bf799e173473245ee0c77f781d754623.
  - Post-merge main verification returned exactly 9f7e5587bf799e173473245ee0c77f781d754623 and contains the four intended documentation/programme changes.
derived:
  - The allocation is now a safe role/dependency companion rather than a competing priority scheduler.
unknown: []
conflicts: []
first_failure:
  marker: checkpoint-validation-result-enum
  evidence: CI run 31736230255 job 94568413227 initially failed because validation item 2 used unsupported result IN_PROGRESS; the root cause was repaired and every later required exact-head generation passed.
rejected_hypotheses:
  - Create a second independent portal-completion programme.
  - Let the allocation board choose a later workstream before the canonical programme selects it.
  - Treat Codex suitability as standing permission to invoke Codex.
  - Treat Issue #489 as only Game Catalog or Issue #490 as only public-edge work.
changed_paths:
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION_WORK_ALLOCATION.md
  - docs/agents/programs/OTERYN_PORTAL_COMPLETION.md
  - docs/architecture/PORTAL_COMPLETION_DELIVERY_PLAN.md
  - docs/agents/tasks/archive/OTERYN-20260813-portal-program-work-allocation.md
validation:
  - command: final synchronization with current main before merge
    result: PASS
    evidence: final head 2d77f8e29848082f5f008fbfc230a09123d19504 has e4dab032387b9a4c3fa86fd52b1b74f2bd2529c5 as its second parent.
  - command: exact final PR full-diff self-review
    result: PASS
    evidence: final PR #1031 diff at head 2d77f8e29848082f5f008fbfc230a09123d19504 contains only the four declared documentation/programme files and all reviewed corrections remain present.
  - command: executable runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: this task changes documentation/programme routing only and introduces no executable user or integration journey.
  - command: PR review-thread hygiene
    result: PASS
    evidence: all three material review threads are resolved.
  - command: repository governance/CI on final head 2d77f8e29848082f5f008fbfc230a09123d19504
    result: PASS
    evidence: CI 31736757370, Agent Governance 31736757390, Native protocol contract 31736757406, Native protocol contract audits 31736757432, Edge Security Emulation 31736757415, Phase 7 Production-Like Validation 31736757387, Platform DB Outage Validation 31736757383 and Game Auth Ticket Concurrency 31736757404 all succeeded.
  - command: squash merge PR #1031
    result: PASS
    evidence: PR #1031 is closed merged with merge commit 9f7e5587bf799e173473245ee0c77f781d754623.
  - command: post-merge main verification
    result: PASS
    evidence: protected main resolved to 9f7e5587bf799e173473245ee0c77f781d754623 immediately after merge and contains the intended four-file documentation delivery.
blockers:
  - none
next_action: re-evaluate OTERYN_PORTAL_COMPLETION from current main and select the first safe, unowned, unblocked and implementation-authorized work item.
```

## Notes

This task changed documentation/programme routing only. It did not invoke Codex, deploy production, mutate protected environments, access or modify a server/game repository, or modify the independent PR #1028 implementation.
