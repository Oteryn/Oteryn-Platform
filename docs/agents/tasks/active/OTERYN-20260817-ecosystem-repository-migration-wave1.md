---
task_id: OTERYN-20260817-ecosystem-repository-migration-wave1
required_reads:
  - AGENTS.md
  - docs/agents/AGENTS.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/EXECUTION_PROTOCOL.md
  - docs/agents/PROJECT_LANES.json
  - docs/architecture/adr/0041-ecosystem-repository-authority-contracts-and-atlas-integration.md
---

# OTERYN-20260817-ecosystem-repository-migration-wave1

## Goal

Execute Wave 1 of `OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION`: reconstruct fresh repository state, classify migration-critical coordinates and publish a fail-closed physical-cutover readiness decision before any rename/create/transfer/extraction.

Issue: #1130
PR: #1131
Branch: `docs/oteryn-ecosystem-repository-migration-wave1`

## Acceptance criteria

- [x] Current source repositories and target-name existence are re-read from GitHub.
- [x] Current authenticated organization membership is checked.
- [x] Current ADR 0041 topology authority is re-read and no superseding topology decision is located.
- [x] Migration-critical Oteryn-v2 control-plane paths are inspected.
- [x] Current Oteryn-v2 Releases state is inspected.
- [x] GHCR/package inventory access is attempted and any access gap is recorded as UNKNOWN, not guessed.
- [x] Migration-relevant Oteryn-Platform native-protocol workflows are inspected.
- [x] Current Otheryn Atlas automation is revalidated against fresh main, including the private Synology deployment path.
- [x] A-G repository-coordinate inventory is persisted.
- [x] Selective Atlas extraction manifest is persisted.
- [x] META, Game, Platform and Atlas receive explicit readiness states and one next action.
- [x] No physical repository rename/create/transfer/extraction is performed while blockers remain.
- [ ] Exact-head repository-selected CI/governance validation passes for the documentation delivery.
- [ ] Whole-diff self-review has zero material findings.
- [ ] PR is integrated through protected main, then task lifecycle is archived and ownership released.

## Ownership

```yaml
owned_paths:
  - docs/architecture/migration/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_READINESS.md
  - docs/architecture/migration/oteryn-repository-coordinate-inventory.json
  - docs/architecture/migration/oteryn-atlas-extraction-manifest.json
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/agents/tasks/active/OTERYN-20260817-ecosystem-repository-migration-wave1.md
modules:
  - ecosystem-repository-migration
  - architecture-governance
project_lane: oteryn-platform-core
blockers:
  - future Oteryn GitHub organization not visible to authenticated account
  - Oteryn-v2 GHCR/package inventory inaccessible through current integration
  - external Actions/reusable-workflow caller inventory not exhaustive
  - Otheryn Atlas remains tied to active private Synology deployment and mixed path ownership
cross_repository_tasks:
  - repository: blakinio/Oteryn-v2
    mode: read_only_discovery
  - repository: blakinio/Otheryn
    mode: read_only_discovery
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-17T14:53:00+02:00
head: none
branch: docs/oteryn-ecosystem-repository-migration-wave1
pr: 1131
status: validating
context_routes:
  - agent-governance
  - architecture
  - testing
  - ci-repair
owned_paths:
  - docs/architecture/migration/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_READINESS.md
  - docs/architecture/migration/oteryn-repository-coordinate-inventory.json
  - docs/architecture/migration/oteryn-atlas-extraction-manifest.json
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/agents/tasks/active/OTERYN-20260817-ecosystem-repository-migration-wave1.md
proven:
  - Oteryn-Platform observation baseline for this wave was 3724349a8738bff8229b83239244120876bdedfd.
  - Oteryn-v2 observation baseline was c8a3ac666845f3e8679e55dd4c84d1e440e830c8.
  - Otheryn advanced during the audit and was refreshed to observation baseline 5001cb42b9027d763d87a099696136acd3e12e83.
  - Target names blakinio/Oteryn, blakinio/Oteryn-Game and blakinio/Oteryn-Atlas returned Not Found at inspection.
  - Authenticated GitHub organization membership returned an empty organization set at inspection.
  - Oteryn-v2 repository-policy application and merge-gate use dynamic current-repository identity for the inspected routing-critical calls.
  - Oteryn-v2 Releases endpoint returned an empty list at the inspected baseline.
  - Oteryn-v2 package inventory endpoint returned 403 Resource not accessible by integration.
  - Oteryn-v2 had open draft PRs 314 and 317 at inspection.
  - Platform native-protocol contract and audit workflows inspected in this wave contain no literal old Game executable Actions coordinate.
  - Otheryn current Atlas deployment request explicitly targets private-synology and dispatches the canonical full-world workflow through GITHUB_REPOSITORY.
  - Otheryn full-world Atlas workflow includes ots/synology execution, /volume1/docker/otheryn/atlas state and mixed tools/deploy paths.
  - Oteryn-v2 tools/game-atlas-profile-spike exists at the inspected Game baseline and contains README.md plus spike.py.
derived:
  - Accepted four-repository architecture remains valid but physical topology is not ready.
  - Game rename is NO_GO_YET pending external Actions caller and GHCR/package proof.
  - META is architecture-ready but physically blocked on the intended future organization.
  - Platform remains one repository and needs no physical change in Wave 1.
  - Atlas remains extractable-with-refactor but physical extraction is unsafe while mixed ownership and production deployment authority remain coupled.
unknown:
  - exact future Oteryn GitHub organization identity and permissions
  - exhaustive external Actions or reusable-workflow callers of Oteryn-v2
  - exact Oteryn-v2 GHCR/package names, links, consumers and permissions
  - complete path-level Atlas ownership split inside tools/otbm_atlas and tools/otbm_atlas_facts
conflicts: []
first_failure:
  marker: agent-governance-checkpoint-schema
  evidence: Agent Governance run 32031726541 first failed because first_failure was null; follow-up run 32031936572 exposed the remaining checkpoint-v1 contract mismatches before this repair.
rejected_hypotheses:
  - Creating empty temporary blakinio/Oteryn, blakinio/Oteryn-Game or blakinio/Oteryn-Atlas repositories is an acceptable substitute for the intended topology.
  - Ordinary GitHub rename redirects are sufficient proof for GitHub Actions and reusable-workflow consumers.
  - Otheryn Atlas is dormant code; current main has active private Synology deployment automation.
  - Checkpoint version 2 is accepted by current Oteryn-Platform governance; GOVERNANCE_CONTRACT.json requires structural checkpoint version 1.
changed_paths:
  - docs/architecture/migration/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_READINESS.md
  - docs/architecture/migration/oteryn-repository-coordinate-inventory.json
  - docs/architecture/migration/oteryn-atlas-extraction-manifest.json
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/agents/tasks/active/OTERYN-20260817-ecosystem-repository-migration-wave1.md
validation:
  - command: fresh GitHub source/target live-state readback
    result: PASS
    evidence: observation baselines and target Not Found results are recorded in the readiness report
  - command: authenticated organization membership inspection
    result: PASS
    evidence: inspection completed and returned zero visible organizations; this outcome is recorded as a migration blocker
  - command: migration-critical Game, Platform and Atlas file inspection
    result: PASS
    evidence: coordinate inventory and Atlas extraction manifest preserve unresolved gates as UNKNOWN instead of promoting physical cutover
  - command: exact-head repository-selected CI and Agent Governance after this checkpoint repair
    result: NOT_RUN
    evidence: this checkpoint rewrite creates a new PR head that must be validated by the repository-selected workflows
  - command: runtime, component and browser E2E
    result: NOT_APPLICABLE
    evidence: Wave 1 changes discovery and documentation only; no executable product or deployment path changed
blockers:
  - intended future Oteryn organization is unavailable to the current authenticated account
  - package and external executable-caller proofs are incomplete
  - Atlas deployment and path ownership require a later separately authorized cutover design
next_action: Validate PR 1131 on its exact current head; after integration and archival, create or identify the intended future Oteryn GitHub organization and make it visible to the authenticated GitHub account.
```

## Safety boundary

No writes were made to `blakinio/Oteryn-v2`, `blakinio/Otheryn`, `blakinio/canary` or `blakinio/otclient`. No Synology, production, DNS, package, secret, payment or live-game mutation was performed. No owner-funded Codex/OpenAI/API call was used.
