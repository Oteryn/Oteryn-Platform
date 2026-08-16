---
task_id: OTERYN-20260815-ecosystem-repository-architecture
status: completed
project_lane: oteryn-platform-core
execution_mode: github_connector
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/architecture/ARCHITECTURE_AUTHORITY.md
  - docs/architecture/adr/0031-native-oteryn-v2-integration-boundary.md
  - docs/architecture/adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md
search_first:
  - PR #1096
  - closed superseded PR #1065
optional_reads: []
---

# OTERYN-20260815 ecosystem repository architecture — terminal archive

## Terminal outcome

The repository owner's corrected Oteryn ecosystem repository decision was recorded as accepted ADR 0040 through PR #1096.

The accepted target topology is:

- future `Oteryn` meta repository for cross-repository architecture, manifests, compatibility and global integration/governance;
- `Oteryn-Game` as the target name/boundary for the native Oteryn-v2 client/server/protocol/world/Studio product repository;
- `Oteryn-Platform` for Portal, Identity, Accounts, GameAuth, the separately deployable Game Gateway and other application-platform modules;
- future standalone `Oteryn-Atlas` for the first-party browser map product;
- organization-level `.github` for appropriate shared policy/community/reusable workflow material.

ADR 0040 also records that separate permanent Portal, Identity, Login, Gateway, Client, Server and Protocol repositories are not justified now, and that the future meta repository should use explicit immutable SHA/version/digest manifests rather than Git submodules.

## Atlas ownership decision

The owner states that the current OTBM Atlas is located in the legacy `blakinio/Otheryn` project, which is an old Canary/Crystal Server lineage. That repository is therefore classified for target architecture purposes as `LEGACY / MIGRATION SOURCE / HISTORICAL REFERENCE`, not as the future Atlas, meta repository or native game repository.

The accepted ownership split is:

- `Oteryn-Game` owns canonical native world/content semantics, world compiler/bundles/validation, Oteryn Studio and bounded legacy OTBM import/migration;
- `Oteryn-Atlas` owns browser rendering/runtime, search/details, layers/overlays, POI/spawn/NPC presentation, map-specific URL/deep-link behavior and derived map-data ingestion/publication;
- `Oteryn-Platform` integrates the public Atlas product through navigation/route/auth/deployment composition where appropriate, but does not become canonical OTBM/world or Atlas implementation authority.

The target data flow is legacy world/OTBM input -> bounded `Oteryn-Game` importer -> canonical Oteryn World/Content Model -> versioned immutable Atlas export -> `Oteryn-Atlas` -> browser map. The exact export schema/transport and exact legacy Atlas source subtree remain separately authorized follow-up work.

## Predecessor reconciliation

Draft PR #1065 proposed a different premise: keeping the legacy Otheryn project as canonical Atlas producer/source of truth. The owner correction superseded that premise before merge. PR #1065 was intentionally closed unmerged, proposed ADR 0038 never became canonical, no submitted reviews or unresolved review threads remained, and its source branch is absent.

ADR number 0038 remains intentionally unused rather than being recycled after the abandoned draft. ADR 0040 is the accepted corrected decision.

## Validation and review

Implementation PR #1096 changed exactly three documentation paths:

- `docs/agents/tasks/active/OTERYN-20260815-ecosystem-repository-architecture.md`;
- `docs/architecture/adr/0040-oteryn-ecosystem-repository-topology-and-atlas-extraction.md`;
- `docs/architecture/adr/README.md`.

The first ready-state CI generation on head `d0dfcd250e1888cab9e00bfbdf0c2dda8f272af4` failed for two deterministic repository-governance causes and was not retried unchanged:

1. CI `31880128280` failed because the task branch had started before PR #1086 and therefore did not contain the PHP coverage-policy validator files referenced by the current workflow definition;
2. Agent Governance `31880128272` reported the task checkpoint was missing required `context_routes`.

The repair merged current `main@9f58beb5aef26bd8d3cc407c7559b4151d3d46d9` into the task branch without overlapping the three owned paths and added the required checkpoint field. Final implementation head `5428c67e0b8d02fafbdf1dfb6974a71cc5db254a` then passed every observed applicable workflow:

- CI `31880480947` — SUCCESS;
- Agent Governance `31880480911` — SUCCESS;
- Edge Security Emulation `31880480954` — SUCCESS;
- Game Auth Ticket Concurrency `31880480956` — SUCCESS;
- Platform DB Outage Validation `31880480957` — SUCCESS;
- Phase 7 Production-Like Validation `31880480869` — SUCCESS;
- Native protocol contract `31880480898` — SUCCESS;
- Native protocol contract audits `31880480902` — SUCCESS.

There were no PR reviews requesting changes and no review threads. Runtime/browser E2E is `NOT_APPLICABLE`: the delivered change is architecture/governance documentation and creates no executable product or user journey.

PR #1096 auto-merged to protected `main` as `d5468c96a5a6dd5fd6a52d504e9cf8a2757f5a7e`. The implementation source branch `agent/oteryn-20260815-ecosystem-repository-architecture` is absent after merge.

## Context checkpoint

```yaml
checkpoint_version: 1
policy_version: 2
updated_at: 2026-08-15T10:54:00Z
head: d5468c96a5a6dd5fd6a52d504e9cf8a2757f5a7e
branch: docs/oteryn-20260815-ecosystem-repository-architecture-closeout
pr: none
status: completed
context_routes:
  - architecture
  - agent-governance
  - testing
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260815-ecosystem-repository-architecture.md
  - docs/agents/tasks/active/OTERYN-20260815-ecosystem-repository-architecture.md
proven:
  - accepted ADR 0040 is on protected main through PR #1096
  - final implementation head 5428c67e0b8d02fafbdf1dfb6974a71cc5db254a passed all eight observed applicable workflows
  - PR #1096 squash-merged as d5468c96a5a6dd5fd6a52d504e9cf8a2757f5a7e
  - implementation PR changed exactly three declared documentation paths
  - implementation source branch is absent after merge
  - predecessor PR #1065 is closed unmerged and its source branch is absent
  - runtime/browser E2E is not applicable to this documentation-only decision
  - no repository creation/transfer, external/server repository mutation, runtime change, Synology/DNS mutation, deployment or production activation occurred
unknown:
  - final closeout PR number and merge SHA until this lifecycle-only archive transition is delivered
  - closeout branch final absence until after closeout merge
  - exact legacy Atlas source paths/history and exact Game-to-Atlas export contract remain follow-up work by ADR 0040
conflicts: []
first_failure:
  marker: repaired-stale-base-and-checkpoint-schema
  evidence: initial CI 31880128280 and Agent Governance 31880128272 were repaired by current-base merge plus context_routes; final generation is fully green
rejected_hypotheses:
  - retry the first failed CI generation unchanged
  - weaken repository governance checks to merge the ADR
  - preserve PR #1065 as a competing Atlas authority
changed_paths:
  - docs/agents/tasks/archive/OTERYN-20260815-ecosystem-repository-architecture.md
  - docs/agents/tasks/active/OTERYN-20260815-ecosystem-repository-architecture.md
validation:
  - command: final exact-head GitHub Actions generation
    result: PASS
    evidence: 5428c67e0b8d02fafbdf1dfb6974a71cc5db254a; eight observed applicable workflows SUCCESS
  - command: governance/review hygiene
    result: PASS
    evidence: Agent Governance 31880480911 SUCCESS; no reviews requesting changes; no review threads
  - command: implementation source branch closeout
    result: PASS
    evidence: exact Git ref lookup returned 404 after PR #1096 merged
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: architecture documentation only; no executable product behavior was changed
blockers: []
next_action: Merge the lifecycle-only closeout PR after its exact-head docs/governance validation, then verify the closeout branch is absent.
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: the implementation branch is already absent and this lifecycle-only closeout branch has no retention, rollback or recovery purpose after merge
source_branch_evidence: implementation ref lookup returned 404; final absence of docs/oteryn-20260815-ecosystem-repository-architecture-closeout must be verified immediately after closeout merge
```

## Closeout boundary

This closeout changes only task lifecycle state. It does not alter ADR 0040, runtime/application code, CI workflows, repository settings, external repositories, deployment state, credentials, Synology, DNS or production environments.
