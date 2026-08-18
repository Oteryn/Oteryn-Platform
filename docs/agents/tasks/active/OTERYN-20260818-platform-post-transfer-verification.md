---
task_id: OTERYN-20260818-platform-post-transfer-verification
required_reads:
  - AGENTS.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/prompts/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION_PROGRAM.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/architecture/migration/oteryn-platform-transfer-inventory.json
search_first:
  - OTERYN-PLATFORM-TRANSFER-20260818
optional_reads: []
---

# OTERYN-20260818-platform-post-transfer-verification

## Goal

Verify the completed physical owner transfer of canonical Platform repository ID `1305155726` to `Oteryn/Oteryn-Platform`, prove bounded GHCR and self-hosted-runner cutover behavior without using protected staging, then reconcile the provider and META migration state only to the level actually proven.

## Acceptance criteria

- [x] Verify repository ID `1305155726` now resolves to `Oteryn/Oteryn-Platform` with connector admin access.
- [x] Verify exact pre-cutover `main` continuity at `42f6741deacbd3ba9e1c4f609bb1073ebe0cff7b`.
- [x] Verify `main` remains protected with required `classify-changes` and `test` contexts.
- [x] Verify historical PR identity survived transfer by reading PR #1161 at the new coordinate.
- [ ] Prove target GHCR publication/readback for the three Platform-owned image names using a bounded verification tag and GitHub-hosted runner only.
- [ ] Prove whether a runner matching `self-hosted` + `oteryn-staging` still accepts a repository job after transfer, without running protected staging.
- [ ] Remove the task-specific verification workflow before terminal delivery merge.
- [ ] Reconcile provider programme/readiness/inventory with exact post-transfer evidence.
- [ ] Reconcile META manifest and Issue #7 only after provider verification is terminal.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260818-platform-post-transfer-verification.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/architecture/migration/oteryn-platform-transfer-inventory.json
  - .github/workflows/one-off-platform-post-transfer-verification.yml
modules:
  - agent-governance
  - repository-migration-programme
  - ci-verification
  - synology-runner-contract
dependencies:
  - physical owner transfer completed by repository owner
  - Oteryn/Oteryn#7
blockers: []
cross_repository_tasks:
  - Oteryn/Oteryn#7 tracks terminal ecosystem cutover closeout
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-18T16:53:00Z
head: 42f6741deacbd3ba9e1c4f609bb1073ebe0cff7b
branch: chore/platform-post-transfer-verification-20260818
pr: none
status: implementing
context_routes:
  - agent-governance
  - testing
owned_paths:
  - docs/agents/tasks/active/OTERYN-20260818-platform-post-transfer-verification.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/architecture/migration/oteryn-platform-transfer-inventory.json
  - .github/workflows/one-off-platform-post-transfer-verification.yml
proven:
  - repository ID 1305155726 resolves to Oteryn/Oteryn-Platform after the owner transfer
  - connected GitHub integration has admin access to the transferred repository
  - transferred main is exactly 42f6741deacbd3ba9e1c4f609bb1073ebe0cff7b matching the pre-cutover head
  - transferred main remains protected with required classify-changes and test checks
  - PR 1161 is preserved at the new coordinate with the same merge SHA 42f6741deacbd3ba9e1c4f609bb1073ebe0cff7b
  - pre-existing open PRs are visible at the new coordinate
  - migration backup repository ID 1338405017 remains separate at Oteryn/Oteryn-Platform-Migration-Backup-20260818
derived:
  - physical owner transfer and core repository identity/history continuity succeeded
unknown:
  - target GHCR package publication/read/link behavior after owner transfer
  - repository-level self-hosted runner attachment/online behavior after owner transfer
conflicts: []
first_failure: null
rejected_hypotheses:
  - Treat the owner operation as a copy into a new repository identity.
  - Mark the migration COMPLETED before package and runner cutover verification.
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260818-platform-post-transfer-verification.md
validation:
  - command: live repository ID and transferred main/protection readback
    result: PASS
    evidence: repository ID 1305155726 at Oteryn/Oteryn-Platform and exact main 42f6741deacbd3ba9e1c4f609bb1073ebe0cff7b
  - command: transferred historical PR readback
    result: PASS
    evidence: PR 1161 retained number state merge SHA and body at the new coordinate
blockers: []
next_action: open the Draft PR, then add a task-specific branch-only verification workflow to prove GHCR target publication/readback and self-hosted runner attachment
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: bounded post-transfer verification and migration-state reconciliation
source_branch_evidence: pending
```

## Notes

The task-specific verification workflow is temporary evidence scaffolding. It must be removed from the branch before terminal delivery merge. It may publish only bounded verification-tagged images and may not invoke protected staging or production operations.
