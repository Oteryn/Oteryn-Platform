---
task_id: OTERYN-20260815-ecosystem-repository-architecture
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/adr/README.md
search_first:
  - open architecture PRs and overlapping owned paths
  - PR #1065 public Map atlas integration
  - Issue #302 optional maps disposition
optional_reads: []
---

# OTERYN-20260815-ecosystem-repository-architecture

## Goal

Record the repository owner's corrected target Oteryn ecosystem repository topology and Atlas extraction boundary in Oteryn Platform as temporary cross-repository architecture authority until the future Oteryn meta repository exists.

## Acceptance criteria

- [x] Reconcile protected `main`, active task paths and open architecture PR ownership before editing.
- [x] Intentionally close draft PR #1065 because its canonical-Otheryn-producer premise is superseded by the repository owner's corrected Atlas decision.
- [ ] Record an accepted ADR for the target `Oteryn`, `Oteryn-Game`, `Oteryn-Platform` and `Oteryn-Atlas` repository topology.
- [ ] Record the ownership split between canonical world/OTBM migration concerns, Atlas product concerns and Platform public integration.
- [ ] Record that the legacy `blakinio/Otheryn` project is a migration source, not a target architecture authority.
- [ ] Update the ADR registry without reusing the abandoned `0038` draft identifier.
- [ ] Pass repository-required exact-head validation for the documentation-only change.
- [ ] Merge only after the exact-head merge gate is satisfied, then archive/release this task through repository closeout policy.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-ecosystem-repository-architecture.md
  - docs/architecture/adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md
  - docs/architecture/adr/README.md
modules:
  - architecture
  - repository-governance
dependencies:
  - accepted Platform ADR 0031 for native Oteryn-v2 versus legacy Canary compatibility boundaries
blockers:
  - none
cross_repository_tasks:
  - future migration task must inspect the legacy Atlas source before moving code/history
  - future Oteryn meta repository must adopt or supersede this temporary cross-repository authority
```

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-15T10:35:00Z
phase: implement
session_id: chat-20260815-ecosystem-architecture
session_role: implementer
execution_mode: github
execution_reason: bounded documentation-only architecture change using the GitHub connector
project_lane: oteryn-platform-core
head: UNKNOWN
branch: agent/oteryn-20260815-ecosystem-repository-architecture
pr: none
status: implementing
context_routes:
  - agent-governance
  - architecture
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260815-ecosystem-repository-architecture.md
  - docs/architecture/adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md
  - docs/architecture/adr/README.md
proven:
  - Protected main was reconciled at 536c6320f91d1df981600530e50522d84b1c0588 before the task branch was created.
  - Main active task records do not claim this task's new task/ADR paths.
  - Open PR #1093 owns docs/architecture/ARCHITECTURE_AUTHORITY.md, so this task intentionally does not edit that shared authority index.
  - Draft PR #1065 owned docs/architecture/adr/README.md and proposed ADR 0038; it was intentionally closed unmerged on 2026-08-15 with Branch-Disposition: delete before this replacement task claimed the registry path.
  - PR #1065 had no submitted reviews or unresolved review threads when closed.
  - Issue #302 previously deferred interactive maps pending authoritative ownership/provenance; this owner decision now supplies the repository/product ownership direction without authorizing runtime implementation.
  - The repository owner states that the current OTBM Atlas lives in the legacy blakinio/Otheryn project, which is an old Canary/Crystal Server lineage and should be moved out rather than treated as the target Oteryn architecture.
derived:
  - PR #1065's assumption that the legacy Otheryn repository should remain the canonical Atlas producer is no longer valid.
  - A dedicated Oteryn-Atlas product repository avoids coupling the browser map product to the legacy Canary/Crystal server repository while preserving a clean ownership boundary with the future native game/world implementation.
unknown:
  - Exact legacy Atlas source paths, commit boundaries and extraction history because this Platform-only task does not inspect external/server repositories.
  - Exact versioned Game-to-Atlas export schema and transport until a separately authorized producer/consumer contract task defines it.
  - Exact future GitHub organization handle availability and migration date.
conflicts:
  - closed PR #1065 historical draft versus the 2026-08-15 repository-owner Atlas decision; resolved by closing #1065 unmerged and replacing it with ADR 0040.
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - Keep the Atlas permanently inside the legacy Otheryn/Canary/Crystal project; rejected by repository-owner decision.
  - Move the whole legacy Otheryn repository and rename it Oteryn-Atlas; rejected because the repository contains unrelated legacy game-server lineage and would preserve the wrong ownership boundary.
  - Put the Atlas implementation inside Oteryn-Platform; rejected because Platform should integrate the public product rather than own canonical world parsing/map-domain implementation.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260815-ecosystem-repository-architecture.md
  - docs/architecture/adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md
  - docs/architecture/adr/README.md
validation:
  - command: live repository/PR/path ownership reconciliation
    result: PASS
    evidence: protected main 536c6320f91d1df981600530e50522d84b1c0588; predecessor PR #1065 closed unmerged; open architecture PR #1093 does not own this task's three paths
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: this task changes architecture documentation only and creates no executable product or integration journey
  - command: exact-head repository CI
    result: NOT_RUN
    evidence: candidate commit not yet persisted
blockers:
  - none
next_action: Persist ADR 0040 and the ADR registry update on the dedicated task branch, then open the replacement draft PR and run exact-head validation.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is active
source_branch_evidence: dedicated branch created from protected main 536c6320f91d1df981600530e50522d84b1c0588
```

## Notes

This task records architecture only. It does not create repositories, move Git history, access or mutate external/server repositories, change runtime code, deploy Atlas, modify Synology, or activate `/map` in production.