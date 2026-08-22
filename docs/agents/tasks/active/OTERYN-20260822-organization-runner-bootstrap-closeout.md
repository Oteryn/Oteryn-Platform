---
task_id: OTERYN-20260822-organization-runner-bootstrap-closeout
required_reads:
  - AGENTS.md
  - docs/agents/PLATFORM_AGENT_BOOTSTRAP.md
  - docs/agents/DELIVERY_COMPLETENESS_AND_CLOSEOUT.md
  - docs/agents/ANTI_STALL_AND_EXECUTION_BUDGET.md
  - docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md
  - docs/agents/TERMINAL_ONLY_COMMUNICATION.md
  - docs/agents/GITHUB_ONLY_EXECUTION.md
  - docs/agents/AUTONOMOUS_PROGRAM_CONTINUATION.md
  - docs/agents/REPOSITORY_MAP.md
  - docs/agents/CONTEXT_ROUTING.md
search_first:
  - organization runner bootstrap
  - deploy/synology/runner
optional_reads: []
---

# OTERYN-20260822-organization-runner-bootstrap-closeout

## Goal

Rebase the canonical organization-capable Synology runner bootstrap/hardening from stale PR #1200 onto current Platform `main`, without restoring superseded activation workflows, then terminally reconcile Platform #1199 and PR #1200 after exact-head validation.

## Acceptance criteria

- [x] Current-main candidate contains the reviewed organization/repository registration entrypoint, immutable runner image inputs, self-tests and no-secret organization Compose/runbook.
- [x] Existing repository-scoped `oteryn-staging` restart compatibility remains in the entrypoint and Compose contract.
- [x] Organization mode requires explicit group/name/custom labels, uses `--runnergroup` and `--no-default-labels`, and rejects generic default labels.
- [x] Current runner Compose retains the Platform state mount while gaining generic scope/group/token-file inputs.
- [x] Superseded activation/repair workflows from stale PR #1200 are not reintroduced.
- [ ] Exact-head Platform CI/governance/security/build validation passes and no material review finding remains.
- [ ] PR #1200 is intentionally closed as superseded; #1199 closes only after current-main successor merge and resulting source inspection.

## Ownership

```yaml
owned_paths:
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/Dockerfile
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/compose.organization.example.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/runner/organization.env.example
  - deploy/synology/runner/test-entrypoint.sh
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - docs/agents/tasks/active/OTERYN-20260822-organization-runner-bootstrap-closeout.md
modules:
  - synology-runner-bootstrap
dependencies:
  - Oteryn/Oteryn-Platform#1199
  - Oteryn/Oteryn#34
blockers:
  - none
cross_repository_tasks:
  - Oteryn/Oteryn-Atlas#35
  - Oteryn/Oteryn-Game#34
```

## Context checkpoint

```yaml
checkpoint_version: 1
updated_at: 2026-08-22T02:38:00Z
head: 526e9d054e5785a6bb551716925f9be01ab1e800
branch: infra/issue-1199-runner-bootstrap-current-main
pr: 1213
status: validating
context_routes:
  - testing
  - ci-repair
  - agent-governance
owned_paths:
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/Dockerfile
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/compose.organization.example.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/runner/organization.env.example
  - deploy/synology/runner/test-entrypoint.sh
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - docs/agents/tasks/active/OTERYN-20260822-organization-runner-bootstrap-closeout.md
proven:
  - current Platform main is a8d46c91571a8df5193d6ddd2c28b35db2e85934 after Atlas scaffold terminal cleanup
  - stale PR 1200 head 7f5b77361cb8719e163704b528845d1f3463e1a6 is 11 commits behind current main and must not merge as-is
  - canonical runner-core bytes from PR 1200 were selectively carried forward; stale organization-runner activation workflows were omitted
  - Dockerfile pins Actions Runner 2.336.0 tarball sha256 04cf0be1aff4c3ec3554466c39124ca250e3effd8873bb7e8d68535aa9505d5d plus immutable Ubuntu and Docker CLI bases
  - entrypoint self-test covers organization group routing, no-default-labels, repository backward compatibility, token-file registration, malformed identity failures and persistent restart
  - current live organization runners previously proved Actions Runner 2.336.0 and immutable Oteryn image digest sha256:f0c452798a17df09006a12d437e83a72d681dcd338ef22ed01fca329d1bbab8d
derived:
  - the safe current-main closeout is bounded to reusable bootstrap/provenance/test/runbook surfaces and does not mutate the live runner control plane
unknown:
  - exact-head validation result of PR 1213 after this checkpoint generation
conflicts: []
first_failure:
  marker: none
  evidence: none
rejected_hypotheses:
  - merge PR 1200 directly; rejected because its base diverged and it contains superseded activation workflows
changed_paths:
  - deploy/synology/runner/.env.example
  - deploy/synology/runner/Dockerfile
  - deploy/synology/runner/compose.organization.example.yml
  - deploy/synology/runner/compose.yml
  - deploy/synology/runner/entrypoint.sh
  - deploy/synology/runner/organization.env.example
  - deploy/synology/runner/test-entrypoint.sh
  - docs/operations/SYNOLOGY_ORGANIZATION_RUNNERS.md
  - docs/agents/tasks/active/OTERYN-20260822-organization-runner-bootstrap-closeout.md
validation:
  - command: full exact diff review against Platform main and stale PR 1200
    result: PASS
    evidence: current candidate contains nine intended files only; no temporary activation workflow is present
  - command: E2E classification
    result: NOT_APPLICABLE
    evidence: repository-only bootstrap/templates perform no live registration or product mutation; executable contract is build-time entrypoint self-test plus exact-head Synology package/image validation
  - command: PR 1213 exact-head workflows
    result: NOT_RUN
    evidence: final checkpoint commit creates the exact-head generation to observe next
blockers:
  - none
next_action: require all exact-head PR 1213 workflows to pass, then reconcile stale PR 1200 and squash merge
```

## Recovery checkpoint

```yaml
recovery:
  policy_version: 1
  generation: 2
  session_id: runner-bootstrap-closeout-20260822-001
  session_started_at: 2026-08-22T02:20:00Z
  checkpointed_at: 2026-08-22T02:38:00Z
  last_progress_at: 2026-08-22T02:38:00Z
  phase: exact-head-validation
  exact_head: 526e9d054e5785a6bb551716925f9be01ab1e800
  pull_request: 1213
  active_operation: GitHub exact-head workflow generation
  external_run_ids: [32546589483, 32546589476, 32546589494, 32546589398, 32546589443, 32546589348, 32546589445, 32546589435]
  operation_started_at: 2026-08-22T02:33:44Z
  wait_deadline_at: 2026-08-22T03:18:44Z
  check_generation: final-candidate
  checks_used: 1
  status: active
  safe_to_resume: true
  resume_condition: PR 1213 exact-head required checks are terminal
  next_action: require all exact-head PR 1213 workflows to pass, then reconcile stale PR 1200 and squash merge
```

## Self-review

```yaml
self_review:
  result: PASS
  exact_head: 526e9d054e5785a6bb551716925f9be01ab1e800
  acceptance_checked: true
  full_diff_checked: true
  negative_paths_checked: true
  rollback_checked: true
  compatibility_checked: true
  related_prs_checked: true
  findings: []
  evidence:
    - repository mode retains exact owner/repository URL validation and defaults to oteryn-synology-staging/oteryn-staging
    - organization mode requires strict group/name/custom label and appends --runnergroup plus --no-default-labels
    - self-test rejects malformed URLs/groups/labels/default labels and proves restart without re-registration
    - organization reference Compose isolates registration/work volumes and withholds Docker/root from Game
    - stale PR 1200 temporary activation workflows are excluded from the current-main candidate
```

## Source branch closeout

```yaml
source_branch_disposition: auto_delete_after_merge
source_branch_reason: ordinary same-repository current-main successor for Platform #1199
source_branch_evidence: pending PR 1213 exact-head validation and squash merge
```

## Notes

This task is repository hardening/closeout only. It does not create runner groups, registration tokens, or mutate the live Synology estate.