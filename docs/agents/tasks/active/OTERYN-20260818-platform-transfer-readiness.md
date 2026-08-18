---
task_id: OTERYN-20260818-platform-transfer-readiness
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
search_first:
  - blakinio/Oteryn-Platform
  - ghcr.io/blakinio
optional_reads: []
---

# OTERYN-20260818-platform-transfer-readiness

## Goal

Refresh and persist exact readiness for transferring `blakinio/Oteryn-Platform` to `Oteryn/Oteryn-Platform`, with executable-coordinate, GHCR, runner, CI/protection, rollback and tool-capability evidence, without performing the physical transfer or any protected live operation.

## Acceptance criteria

- [x] Current source repository identity, admin permission, `main` SHA and required checks are refreshed.
- [x] Target-coordinate collision is checked in organization `Oteryn`.
- [x] Executable Platform-owned old-coordinate/GHCR/runner references are identified and classified.
- [x] Current GitHub transfer/package behavior is reconciled into the cutover gate.
- [x] A fail-closed transaction state and one concrete next action are persisted.
- [ ] Exact-head repository-required CI passes and the task is terminally closed after merge.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260818-platform-transfer-readiness.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/architecture/migration/oteryn-platform-transfer-inventory.json
modules:
  - repository-migration
  - ci-release-provenance
dependencies:
  - OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION
  - Oteryn/Oteryn canonical META ADR 0001
blockers:
  - none for readiness documentation
cross_repository_tasks:
  - Oteryn/Oteryn META CI hardening is independent and does not authorize Platform transfer
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-18T11:30:00Z
head: UNKNOWN
branch: docs/platform-transfer-readiness-20260818
pr: 1151
status: validating
context_routes:
  - agent-governance
  - architecture
  - testing
  - ci-repair
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260818-platform-transfer-readiness.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/architecture/migration/oteryn-platform-transfer-inventory.json
proven:
  - source repository blakinio/Oteryn-Platform exists, is public and grants the connected GitHub integration admin/write access
  - source main is 132cc41d5c722911bdb4f3e30c200c5d8b47f1ec and is protected with required contexts classify-changes and test
  - Oteryn/Oteryn-Platform was not found in the target organization search
  - source build/deploy/runner/preflight paths contain executable blakinio owner and GHCR coordinates that cannot be treated as historical-only references
  - GitHub documents that repository content, Issues, PRs, releases and settings move with transfer while package linkage/ownership requires registry-specific handling
  - the connected GitHub tool surface exposes repository content/PR operations but no repository-transfer or branch-protection/ruleset mutation action
  - Draft PR 1151 owns this exact task branch and bounded three-path readiness change
  - full PR changed-file list and full diff were inspected with no material self-review finding
  - no physical transfer, package mutation, runner re-registration, secret mutation or staging/production operation was performed
  - no Game/server repository was accessed in this task
derived:
  - the organization-destination blocker from Wave 1 is resolved for Platform planning
  - Platform transfer is not yet CUTOVER_READY because GHCR namespace and self-hosted-runner cutover behavior need a pre-transfer hardening delivery and live cutover verification
unknown:
  - live GHCR package objects, permissions and repository links for the three Platform-owned images
  - whether the existing repository-level Synology runner registration will remain usable immediately after owner transfer without re-registration
  - target organization repository ruleset/protection state after transfer until observed
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - ordinary GitHub repository redirects are sufficient for GHCR package ownership and runner registration
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260818-platform-transfer-readiness.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/architecture/migration/oteryn-platform-transfer-inventory.json
validation:
  - command: exact changed-file and full-diff review for PR 1151
    result: PASS
    evidence: exactly three declared readiness paths; no unrelated runtime or protected-operation change
  - command: repository-required GitHub Actions
    result: NOT_RUN
    evidence: exact final candidate head is awaiting required checks
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: documentation/evidence-only migration readiness task; no executable product or live environment mutation
blockers:
  - physical transfer remains held until pre-cutover GHCR/runner hardening is merged and the transfer operation is available to an authorized owner/runtime
next_action: Observe required CI for PR 1151 on the current exact head; repair only evidence-based failures, then merge and archive this readiness task if all gates pass.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```

## Notes

The physical Platform transfer is a later Tier-2 transaction. This task only converts stale Wave-1 assumptions into current, executable preparation evidence.
