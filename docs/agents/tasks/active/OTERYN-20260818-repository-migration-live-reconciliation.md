---
task_id: OTERYN-20260818-repository-migration-live-reconciliation
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
search_first:
  - OTERYN-REPO-MIGRATION-ULTRA
optional_reads: []
---

# OTERYN-20260818-repository-migration-live-reconciliation

## Goal

Reconcile the canonical repository-migration programme and Platform transfer readiness with current verified GitHub state after the Game history-preserving migration, creation of the organization Platform bootstrap target, and subsequent Platform source-main drift.

## Acceptance criteria

- [x] Verify current META, Game, Atlas, Platform source and Platform target repository identities from live GitHub state.
- [x] Reconcile canonical META repository coordinates with the completed Game history-preserving migration and existing Platform bootstrap target.
- [ ] Update Platform transfer readiness to the current source `main`, intended-target identity and exact remaining transfer blockers.
- [ ] Update the durable migration programme so it no longer reports Game as pending or Atlas as absent and does not pretend Platform is ready for transfer.
- [ ] Preserve the fail-closed rule against copying/force-pushing Platform as a substitute for the separately gated repository transfer.
- [ ] Pass exact-head documentation/governance validation and merge the reconciliation through the normal PR gate.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260818-repository-migration-live-reconciliation.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
modules:
  - agent-governance
  - repository-migration-programme
dependencies:
  - Oteryn/Oteryn@20f87798d6429555031fa4e63e0a115db83adffb
blockers:
  - physical Platform repository transfer is not exposed by the connected GitHub action surface
  - intended organization Platform target already exists as bootstrap-only repository, so native same-name transfer requires explicit target cleanup/rename before transfer
  - live GHCR package linkage and repository-level self-hosted runner post-transfer behavior remain unobservable through the current connector
cross_repository_tasks:
  - Oteryn/Oteryn repository-state manifest reconciliation completed by PR #4
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-18T15:46:00Z
head: 77fa480a3f4e847dac98f76e05b6acd27cca4a57
branch: docs/reconcile-repository-migration-live-state-20260818
pr: none
status: implementing
context_routes:
  - agent-governance
  - architecture
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260818-repository-migration-live-reconciliation.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
proven:
  - Oteryn/Oteryn-Game is the active Game coordinate after a history-preserving migration from blakinio/Oteryn-v2; target PR #4 and later repository work prove continuity.
  - Oteryn/Oteryn-Atlas exists with repository-local governance and active DYN-ATLAS-001 work; stale Platform blocker PR #1141 was closed without merge.
  - Oteryn/Oteryn-platform exists as public organization repository ID 1338405017 but its main contains only one-off migration bootstrap workflow scaffolding.
  - Previous Platform history-seed run 32140478830 produced a verified complete bundle but all head ref pushes were rejected because GITHUB_TOKEN lacked workflows permission; head_push_rc=1.
  - Current Platform source main is 77fa480a3f4e847dac98f76e05b6acd27cca4a57, newer than the bootstrap seed pin c567da9d9ae444110262774f8febf5a5abab2a90.
  - Canonical META PR #4 merged as 20f87798d6429555031fa4e63e0a115db83adffb after meta-gate PASS and now records Game as migrated and Platform as bootstrap-only transfer pending.
derived:
  - The existing bootstrap target is intended migration scaffolding, not an unrelated repository, but its existence prevents a native same-name repository transfer until the target coordinate is freed or the transfer plan is explicitly changed.
unknown:
  - live GHCR package object permissions/linkage after owner transfer
  - repository-level Synology runner attachment after owner transfer
  - resulting organization ruleset/protection state after owner transfer
conflicts:
  - prior durable programme state predates completed Game migration and Platform target creation
first_failure:
  marker: platform-history-seed-head-push
  evidence: target workflow run 32140478830 recorded head_push_rc=1 because workflow-bearing refs could not be updated with the Actions GITHUB_TOKEN
rejected_hypotheses:
  - Treat the successful seed job conclusion as proof that Platform refs were migrated.
  - Bypass the canonical transfer gate by force-pushing a history copy into the bootstrap target.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260818-repository-migration-live-reconciliation.md
validation:
  - command: live GitHub repository/PR/workflow evidence refresh
    result: PASS
    evidence: exact repository IDs, current source main, META merge, Game migration PR evidence and seed workflow logs inspected
blockers:
  - connected GitHub action surface exposes no repository-transfer operation
  - existing target coordinate must be resolved before same-name native transfer
  - live package/runner post-transfer evidence remains unavailable
next_action: reconcile the two canonical migration documents to this verified live state and open the exact-head PR
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository documentation/governance reconciliation PR
source_branch_evidence: pending
```

## Notes

This task does not authorize production, staging, DNS, secret, package, runner or repository-transfer mutation. It records exact live state and keeps the physical cutover fail-closed until its canonical transaction can lawfully become READY_TO_EXECUTE.
