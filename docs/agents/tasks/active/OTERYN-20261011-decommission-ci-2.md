---
task_id: OTERYN-20261011-decommission-ci-2
governing_issue: 1474
required_reads: []
search_first:
  - .github/workflows/native-protocol-contract-audits.yml
optional_reads: []
---

# OTERYN-20261011 decommission CI 2

## Goal

Governing GitHub Issue: #1474 (native-only migration) — canonical lifecycle authority for this task. Packet CI-2 of the removal plan under `docs/agents/programs/`: remove the dead or purely descriptive legacy-engine mentions from CI workflows and deploy scripts. Behaviour is unchanged.

## Acceptance criteria

- [x] Audit 4 of `native-protocol-contract-audits.yml` loses its vacuous legacy-path diff guard (no tracked path matches it) and its legacy wording; the fixture validation stays. The workflow is path-filtered and cannot be a required check, so renaming the job is safe.
- [x] Comments and the rollback rejection message in `deploy/synology/scripts/{deploy,lib,release-state,upgrade-legacy-release-state}.sh` say "legacy-engine" instead of the engine name. No branch, label filter, state key or environment variable changed.
- [x] The legacy token ratchet passes and `docs/agents/LEGACY_TOKEN_BASELINE.json` is lowered to the measured counts (never raised).
- [x] Everything else in scope is left on purpose, see the checkpoint.

## Ownership

```yaml
owned_paths:
  - .github/workflows/native-protocol-contract-audits.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/release-state.sh
  - deploy/synology/scripts/upgrade-legacy-release-state.sh
  - docs/agents/LEGACY_TOKEN_BASELINE.json
  - docs/agents/tasks/active/OTERYN-20261011-decommission-ci-2.md
  - docs/agents/tasks/archive/OTERYN-20261011-decommission-ci-2.md
modules:
  - ci
  - deploy
dependencies:
  - none
blockers:
  - none
cross_repository_tasks:
  - none
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-10-11T18:00:00Z
head: UNKNOWN
branch: UNKNOWN
pr: 1543
status: validating
context_routes:
  - ci
owned_paths:
  - .github/workflows/native-protocol-contract-audits.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/release-state.sh
  - deploy/synology/scripts/upgrade-legacy-release-state.sh
  - docs/agents/LEGACY_TOKEN_BASELINE.json
  - docs/agents/tasks/active/OTERYN-20261011-decommission-ci-2.md
  - docs/agents/tasks/archive/OTERYN-20261011-decommission-ci-2.md
proven:
  - The Synology compose topology is already native-only; the remaining deploy script matches are comments, a one-time cleanup of the retired container by service label, the rollback rejection of release state carrying the retired image variable, and retired durable Marketplace state keys. The label, variable and key names are functional wire values and stay.
  - `ci.yml` and `phase7-production-like-validation.yml` environment variables, the path filters of `community-data-acceptance.yml` and the two bazaar staging workflows, and `scripts/acceptance/**` still feed code and tests that exist (`app/`, `config/database.php`, `tests/Feature`); they leave with D-9, D-10 and D-1..D-3.
derived:
  - `tests/e2e/native_auth_ephemeral_cutover` has no workflow reference and drives an external legacy harness through its environment variables and module names; renaming identifiers without that harness would break the contract unverifiably. Its directory name carries no legacy token.
unknown:
  - Whether Audit 4 is configured as a required check in branch protection (the integration token cannot read it); it is path-filtered, so it cannot be.
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses: []
changed_paths:
  - .github/workflows/native-protocol-contract-audits.yml
  - deploy/synology/scripts/deploy.sh
  - deploy/synology/scripts/lib.sh
  - deploy/synology/scripts/release-state.sh
  - deploy/synology/scripts/upgrade-legacy-release-state.sh
  - docs/agents/LEGACY_TOKEN_BASELINE.json
validation:
  - command: python tools/validation/legacy_token_ratchet.py
    result: PASS
    evidence: no group above baseline
  - command: product runtime E2E
    result: NOT_APPLICABLE
    evidence: comment, message and dead-guard changes only
blockers:
  - none
next_action: Archive this record once the PR is closed.
terminal_pr_policy: archive_pending
```

## Source branch closeout

```yaml
source_branch_disposition: pending
source_branch_reason: task is still active
source_branch_evidence: pending
```
