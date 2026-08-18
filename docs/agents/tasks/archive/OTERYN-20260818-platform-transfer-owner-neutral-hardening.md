---
task_id: OTERYN-20260818-platform-transfer-owner-neutral-hardening
required_reads:
  - AGENTS.md
  - AGENTS.override.md
  - docs/agents/AGENTS.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/agents/CI_WORKFLOW_LIFECYCLE.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
search_first:
  - ghcr.io/blakinio
  - https://github.com/blakinio/Oteryn-Platform
optional_reads: []
---

# OTERYN-20260818-platform-transfer-owner-neutral-hardening

## Goal

Remove Platform-owned personal-owner repository/GHCR assumptions from pre-cutover build, deployment, Synology runner and staging-preflight paths while preserving current behavior before transfer and making the same repository code valid after transfer to `Oteryn/Oteryn-Platform`.

## Acceptance criteria

- [x] Platform-owned GHCR publish/deploy paths derive a lowercase namespace from the current repository owner rather than hard-code `blakinio`.
- [x] Character Bazaar uses the current repository-owner namespace for Platform/Gateway images while preserving the separately pinned Canary dependency.
- [x] Liquid20 uses the current repository-owner namespace for its package while preserving the pinned external Freqtrade source coordinate.
- [x] Synology runner image and repository registration URL are explicit/configurable with no hard-coded source repository fallback; an already registered persistent runner remains restart-safe.
- [x] Production-target preflight validates Platform/Gateway immutable images against the current repository owner.
- [x] Focused tests fail if old Platform-owner coordinates are reintroduced.
- [x] The latent ADR-registry validation gap supports the already-canonical cross-repository META successor only with exact merge provenance.
- [x] PR validation remains broad while a repository-only hardening merge cannot automatically publish the privileged deploy-runner image or bootstrap Liquid20 on Synology.
- [x] Full exact-head diff/self-review had zero open material findings.
- [x] Exact-head required CI and every affected workflow lane passed.
- [x] Review hygiene was clean and PR #1153 squash-merged.
- [x] Resulting `main`, branch protection and source-branch cleanup were verified.
- [x] No package, runner, secret, staging/production, repository-transfer or Game/server mutation was performed by this task.

## Ownership

```yaml
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260818-platform-transfer-owner-neutral-hardening.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/architecture/migration/oteryn-platform-transfer-inventory.json
modules:
  - repository-migration
  - ci-release-provenance
  - synology-staging-control
  - architecture-registry-validation
dependencies:
  - OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION
  - OTERYN-20260818-platform-transfer-readiness
blockers:
  - none for this completed hardening task
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-18T12:46:00Z
head: 6a3b92cae0099b36d4b58048657fbfa8aea7b9bf
branch: none
pr: 1153
status: completed
context_routes:
  - agent-governance
  - testing
  - ci-repair
  - architecture
owned_paths:
  - docs/agents/tasks/archive/OTERYN-20260818-platform-transfer-owner-neutral-hardening.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/architecture/migration/oteryn-platform-transfer-inventory.json
proven:
  - implementation PR 1153 exact final head was 43f7649b32eefc50e8e8bdc669d44bf4e5de7338
  - final exact-head Agent Governance run 32137944890 passed
  - final exact-head CI run 32137944800 passed; required classify-changes job 95713558651 runtime-tests job 95713599239 and required test job 95714103528 all passed
  - final exact-head Character Bazaar Staging Validation run 32137944744 passed
  - final exact-head Liquid20 Synology Control run 32137944776 passed
  - final exact-head Synology Rollback Contract run 32137944930 passed
  - final exact-head Synology Production Target Preflight run 32137944818 passed
  - final exact-head Build Synology Staging Images run 32137944703 passed
  - final exact-head Edge Security Emulation run 32137944714 passed
  - final exact-head Platform DB Outage Validation run 32137944713 passed
  - final exact-head Phase 7 Production-Like Validation run 32137944771 passed
  - final exact-head Game Auth Ticket Concurrency run 32137944724 passed
  - PR 1153 had zero reviews zero inline review threads and zero top-level comments at merge gate
  - PR 1153 squash-merged as 6a3b92cae0099b36d4b58048657fbfa8aea7b9bf
  - resulting Platform main is 6a3b92cae0099b36d4b58048657fbfa8aea7b9bf and remains protected with required classify-changes and test contexts
  - source branch ci/platform-transfer-owner-neutral-hardening is absent after merge
  - repository-ghcr-image.sh lowercases and validates the current GitHub repository owner before constructing Platform-owned GHCR coordinates
  - Synology build deploy Character Bazaar Liquid20 and production-target preflight now consume owner-neutral Platform package coordinates
  - pinned Canary and pinned external blakinio/freqtrade dependency coordinates were intentionally preserved as external provenance rather than rewritten with the Platform owner
  - runner first registration now requires explicit RUNNER_URL while an existing persistent .runner configuration remains restart-safe without the old source-coordinate fallback
  - ADR registry now supports the canonical cross-repository META successor only when exactly one bounded successor and one lowercase 40-hex successor merge are declared; local successor existence checks remain fail-closed
  - pull-request validation remains broad but repository-only hardening changes no longer automatically publish the privileged deploy-runner package or bootstrap Liquid20 on Synology after merge
  - no package publication runner registration secret environment staging production repository transfer or Game/server operation was performed by this task
derived:
  - the repository pre-cutover owner-coordinate hardening gate is complete
  - current code preserves the present blakinio GHCR namespace before transfer and resolves lowercase oteryn package coordinates after an owner transfer through repository context
  - live GHCR package ownership permissions repository links and existing runner attachment remain cutover evidence and are not proven by repository code or CI
unknown:
  - live GHCR package objects permissions and repository links for Platform-owned images
  - existing repository-level Synology runner attachment behavior immediately after physical owner transfer
  - target organization branch/ruleset result until the transferred repository is observed
conflicts: []
first_failure:
  marker: adr_registry_cross_repository_successor_not_supported
  evidence: first-head CI run 32136239455 runtime-tests job 95708164496 artifact 9324137637 isolated one latent ADR registry failure for unchanged ADR 0041; bounded validator repair then passed final exact-head full CI
rejected_hypotheses:
  - hardening only primary build and deploy workflows is sufficient
  - preserving a hard-coded old RUNNER_URL fallback is necessary for persistent runner restart
  - pinned Canary or Freqtrade dependency coordinates should be rewritten with the Platform owner
  - changing ADR 0041 back to Accepted or weakening supersession validation is an acceptable CI repair
  - repository-only hardening may safely trigger package publication or Synology bootstrap without separate live-operation authority
changed_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/liquid20-synology-control.yml
  - .github/workflows/synology-production-target-preflight.yml
  - deploy/synology/.env.example
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/scripts/repository-ghcr-image.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - tests/ci/test_synology_deploy_release_identity.py
  - tools/validation/adr_registry.py
  - tools/validation/test_adr_registry.py
  - docs/agents/tasks/archive/OTERYN-20260818-platform-transfer-owner-neutral-hardening.md
  - docs/agents/programs/OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION.md
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/architecture/migration/oteryn-platform-transfer-inventory.json
validation:
  - command: exact final-head Agent Governance
    result: PASS
    evidence: run 32137944890 on 43f7649b32eefc50e8e8bdc669d44bf4e5de7338
  - command: exact final-head CI
    result: PASS
    evidence: run 32137944800 on 43f7649b32eefc50e8e8bdc669d44bf4e5de7338 with classify-changes runtime-tests and test successful
  - command: exact final-head affected workflow lanes
    result: PASS
    evidence: runs 32137944744 32137944776 32137944930 32137944818 32137944703 32137944714 32137944713 32137944771 32137944724 all completed successfully
  - command: full exact-head diff self-review
    result: PASS
    evidence: no open material finding after main-push side-effect and ADR-validator output-contract repairs
  - command: PR review hygiene and merge
    result: PASS
    evidence: zero reviews zero inline threads zero comments; PR 1153 squash-merged as 6a3b92cae0099b36d4b58048657fbfa8aea7b9bf
  - command: source branch disposition
    result: PASS
    evidence: branch lookup returned no ci/platform-transfer-owner-neutral-hardening ref after merge
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: repository CI package-coordinate runner and validation contracts only; no user-facing runtime or live environment operation was executed
blockers: []
next_action: Continue the migration programme by resolving live Platform GHCR package ownership/linkage and self-hosted runner cutover evidence; do not perform the physical transfer until those live gates and an authorized transfer-capable GitHub surface are available.
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: 43f7649b32eefc50e8e8bdc669d44bf4e5de7338
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - exact changed-file list and per-file PR patches
    - exact-head CI and affected workflow run results
    - clean PR review/comment/thread state
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: implementation branch had no continuing purpose after PR 1153 canonicalized the pre-cutover hardening
source_branch_evidence: PR 1153 squash-merged as 6a3b92cae0099b36d4b58048657fbfa8aea7b9bf and branch ci/platform-transfer-owner-neutral-hardening was verified absent
```

## Terminal evidence

```yaml
implementation_pr: 1153
implementation_final_head: 43f7649b32eefc50e8e8bdc669d44bf4e5de7338
implementation_merge: 6a3b92cae0099b36d4b58048657fbfa8aea7b9bf
agent_governance_run: 32137944890
ci_run: 32137944800
required_checks:
  - classify-changes
  - test
affected_workflow_runs:
  - 32137944744
  - 32137944776
  - 32137944930
  - 32137944818
  - 32137944703
  - 32137944714
  - 32137944713
  - 32137944771
  - 32137944724
review_count: 0
inline_thread_count: 0
comment_count: 0
source_branch_deleted: true
runtime_e2e: NOT_APPLICABLE
package_publication_performed: false
runner_registration_performed: false
staging_or_production_operation_performed: false
physical_transfer_performed: false
```
