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

Remove Platform-owned personal-owner repository/GHCR assumptions from the pre-cutover build, deployment, Synology runner and staging-preflight paths while preserving current `blakinio/Oteryn-Platform` behavior before transfer and making the same code valid after transfer to `Oteryn/Oteryn-Platform`.

## Acceptance criteria

- [ ] Platform-owned GHCR publish/deploy paths derive a lowercase namespace from the current repository owner rather than hard-code `blakinio`.
- [ ] Character Bazaar staging control uses the same current repository-owner namespace for Platform/Gateway images while preserving the separately pinned Canary dependency.
- [ ] Liquid20 publication derives its image namespace from the current repository owner.
- [ ] Synology runner image and repository registration URL are explicit/configurable with no hard-coded source repository fallback; an already registered persistent runner remains restart-safe.
- [ ] Production-target preflight validates Platform/Gateway immutable images against the current repository owner without hard-coded `blakinio`.
- [ ] Focused CI tests fail if the source-owner coordinates are reintroduced into executable transfer-sensitive paths.
- [ ] Exact-head required CI, affected workflow lanes and review hygiene pass before merge.
- [ ] No package, runner, secret, staging/production, repository-transfer or Game/server mutation is performed.

## Ownership

```yaml
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/liquid20-synology-control.yml
  - .github/workflows/synology-production-target-preflight.yml
  - deploy/synology/.env.example
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/agents/tasks/active/OTERYN-20260818-platform-transfer-owner-neutral-hardening.md
modules:
  - repository-migration
  - ci-release-provenance
  - synology-staging-control
dependencies:
  - OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION
  - OTERYN-20260818-platform-transfer-readiness
blockers:
  - none for repository hardening
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-18T12:05:00Z
head: ed3a79a0ffe3b0b72e6faec941bab050f513a6d2
branch: ci/platform-transfer-owner-neutral-hardening
pr: none
status: implementing
context_routes:
  - agent-governance
  - testing
  - ci-repair
  - architecture
owned_paths:
  - .github/workflows/build-synology-staging-images.yml
  - .github/workflows/deploy-synology-staging.yml
  - .github/workflows/character-bazaar-staging-control.yml
  - .github/workflows/liquid20-synology-control.yml
  - .github/workflows/synology-production-target-preflight.yml
  - deploy/synology/.env.example
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/scripts/production-target-preflight.sh
  - tests/ci/test_synology_deploy_release_identity.py
  - docs/architecture/migration/OTERYN_PLATFORM_TRANSFER_READINESS.md
  - docs/agents/tasks/active/OTERYN-20260818-platform-transfer-owner-neutral-hardening.md
proven:
  - current Platform main is ed3a79a0ffe3b0b72e6faec941bab050f513a6d2 and remains protected with required classify-changes and test checks
  - active durable tasks own no path in this hardening set; public-domain repair owns only deploy/synology/scripts/health-check.sh among Synology runtime paths and native-auth production verification owns only its task record
  - no open PR was found matching Synology staging GHCR transfer runner hardening intent
  - Platform-owned build/deploy/preflight and runner paths still contain executable blakinio owner coordinates
  - Character Bazaar staging control contains hard-coded Platform/Gateway GHCR owner coordinates plus a separately pinned Canary image
  - Liquid20 control publishes from this repository to a hard-coded blakinio GHCR namespace
  - no Game/server repository was accessed
  - no package runner secret staging production or repository-transfer mutation has occurred
  - container/local clone execution is unavailable because this environment cannot resolve github.com; GitHub Actions will provide exact-head executable validation
derived:
  - a complete Platform transfer hardening must include Character Bazaar and Liquid20 repository-owned package paths in addition to the primary Synology build/deploy workflow
  - existing persistent runner registration can remain restart-safe if RUNNER_URL is required only for first registration and removed as a hard-coded fallback
unknown:
  - live GHCR package objects permissions and repository links remain unobserved and are intentionally deferred to cutover verification
  - existing repository-level Synology runner binding behavior after physical transfer remains unknown and must be observed after transfer
conflicts: []
first_failure:
  marker: local_clone_dns_unavailable
  evidence: container git clone could not resolve github.com; repository work continues through the connected GitHub API and Actions
rejected_hypotheses:
  - hardening only build-synology-staging-images.yml and deploy-synology-staging.yml is sufficient
  - preserving a hard-coded old RUNNER_URL fallback is necessary for an already registered persistent runner
changed_paths:
  - docs/agents/tasks/active/OTERYN-20260818-platform-transfer-owner-neutral-hardening.md
validation:
  - command: exact GitHub live-state and path ownership preflight
    result: PASS
    evidence: current main, active tasks, open PR search and executable owner-coordinate inventory refreshed
  - command: runtime/browser E2E
    result: NOT_APPLICABLE
    evidence: this task changes repository build/deploy configuration contracts only and performs no staging or production operation; affected workflow/contract validation is required instead
blockers: []
next_action: Implement the coherent owner-neutral workflow runner preflight and focused-test package on this dedicated branch, then open a Draft PR and run exact-head validation.
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is implementing
source_branch_evidence: pending
```
