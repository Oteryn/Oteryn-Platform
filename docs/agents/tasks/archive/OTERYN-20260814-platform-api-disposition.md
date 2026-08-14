---
task_id: OTERYN-20260814-platform-api-disposition
mode: architecture
issue: 490
status: completed
programme: OTERYN_PLATFORM_ARCHITECTURE_REVIEW
project_lane: oteryn-platform-core
phase: close
execution_mode: github_connector
---

# OTERYN-20260814-platform-api-disposition

## Goal

Resolve the remaining PlatformAPI architecture/product-disposition gap from Issue #490 without misclassifying bounded game-auth/internal endpoints as a general API and without implementing a speculative general surface before an approved named consumer exists.

## Terminal outcome

The repository owner selected **Option A** on 2026-08-14: explicitly defer the general Platform API until an approved named consumer/use case exists.

- ADR 0036 is `Accepted`.
- `docs/architecture/PLATFORM_API_ARCHITECTURE.md` is the focused canonical activation/adaptation/compatibility owner.
- Existing game-auth and internal Gateway endpoints remain specialized contracts, not general PlatformAPI completeness.
- Platform API is `DEFERRED` in portal work allocation with no speculative implementation handoff.
- The PlatformAPI slice of Issue #490 is terminal; Issue #490 remains open for PublicEdge protected-environment proof and direct production evidence.
- Concurrent PR #1028 PlayerCompanion changes were preserved and its stale post-merge active task was terminally archived as a bounded repository-governance repair.

## Acceptance criteria

- [x] Current API/internal routes were classified from exact source.
- [x] ADR 0036 records the selected Option A and alternatives/trade-offs.
- [x] `PLATFORM_API_ARCHITECTURE.md` defines the named-consumer activation gate and API invariants.
- [x] ARCH-DEC-0005 was removed from the active decision backlog after acceptance.
- [x] Authority routing, Module Catalog, OperationsObservability residuals and portal work allocation were reconciled.
- [x] No general API runtime route, auth scope/client, migration, workflow or production activation was created.
- [x] The automated older-head P2 stale-reference review finding was repaired and its thread resolved/outdated.
- [x] Concurrent PR #1028/current-main synchronization preserved both PlayerCompanion and PlatformAPI canonical state.
- [x] The orphaned #1028 task-liveness blocker was repaired with terminal active→archive lifecycle evidence and `PlayerCompanion foundation = DONE`.
- [x] Exact final PR #1044 head `53aaa8b06a754fe71ee54a903bf2d298eaa49d87` passed all eight emitted workflows.
- [x] PR #1044 squash-merged as `714f52bab6d3115bc1396ce0ccfd524df219dfd6`.
- [x] Issue #490 comment `5291599962` records the PlatformAPI slice terminal while preserving remaining residuals.
- [x] Active task ownership is released by this archive move.
- [x] Runtime/browser E2E for this architecture/lifecycle delta is `NOT_APPLICABLE`; PlayerCompanion executable E2E was already proven on its own exact delivery head.

## Terminal checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-14T11:28:00+02:00
head: 714f52bab6d3115bc1396ce0ccfd524df219dfd6
branch: docs/OTERYN-20260814-platform-api-disposition
pr: 1044
status: completed
context_routes:
  - architecture
  - security
  - agent-governance
owned_paths: []
proven:
  - Repository owner selected ADR 0036 Option A on 2026-08-14.
  - PR #1044 exact final head was 53aaa8b06a754fe71ee54a903bf2d298eaa49d87.
  - Exact-head workflows all succeeded: Native protocol contract 31787098676; Edge Security Emulation 31787098675; Game Auth Ticket Concurrency 31787098687; Platform DB Outage Validation 31787098694; Agent Governance 31787098693; Native protocol contract audits 31787098748; Phase 7 Production-Like Validation 31787098690; CI 31787098695.
  - PR #1044 was mergeable and zero commits behind exact main immediately before squash merge.
  - The sole P2 review thread PRRT_kwDOTcsYjs6ZNajT was repaired and resolved/outdated; no REQUEST_CHANGES review remained.
  - PR #1044 squash-merged as 714f52bab6d3115bc1396ce0ccfd524df219dfd6.
  - Issue #490 comment 5291599962 records PlatformAPI terminal and keeps PublicEdge/direct-production evidence open.
  - Exact final #1028 head de8742d1062ddbbfda263c4d3c3975bd11e16b36 had 24/24 emitted workflows success and its three material review threads resolved before merge dfd7acc29f16252a8d83d9de398f915875d36aab.
  - #1044 repaired the post-#1028 stale active-task governance blocker without modifying PlayerCompanion runtime behavior.
derived:
  - General PlatformAPI is intentionally deferred, not missing by accident; future activation requires a named consumer/use case and the accepted checklist.
  - Remaining #490 work is independent protected-environment/production evidence and must not be inferred complete from this architecture closeout.
unknown:
  - Future named API consumer, exact resource inventory, token/scopes model, rate budgets and compatibility/deprecation windows remain intentionally deferred until an activation trigger exists.
conflicts: []
first_failure:
  marker: current-main-terminal-task-liveness-after-concurrent-merge
  evidence: Governance on synchronized pre-final head detected the merged #1028 stale active task; the repair was included before the final 8/8 successful generation.
changed_paths:
  - terminal lifecycle archive only in this closeout
validation:
  - command: exact final PR #1044 workflows
    result: PASS
    evidence: 8/8 success on exact head 53aaa8b06a754fe71ee54a903bf2d298eaa49d87
  - command: full-diff and review hygiene
    result: PASS
    evidence: 11-path architecture/governance/lifecycle diff reviewed; sole P2 thread resolved/outdated; no REQUEST_CHANGES
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: no executable behavior change in #1044 architecture/lifecycle delta
  - command: merge
    result: PASS
    evidence: squash merge 714f52bab6d3115bc1396ce0ccfd524df219dfd6
blockers: []
next_action: none — terminal task; architecture programme returns to ready risk-based rotation.
```

## Notes

No explicit owner-funded Codex/OpenAI/API invocation was authorized or requested by the agent. A repository-configured automated Codex review triggered earlier when PR #1044 was marked ready; it reviewed an older head and produced the repaired P2 finding. No protected-environment operation, production secret, live-system mutation or external-repository access was used.
